use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use walkdir::{DirEntry, WalkDir};

use crate::markdown;
use crate::project::{Project, repos_in};
use crate::todo::Todo;

const CHANGED_SINCE_READ: &str = "the note changed while vt was running; run the command again";

fn missing_section(note: &Path, heading: &str) -> io::Error {
    io::Error::other(format!("{} has no '{heading}' section", note.display()))
}

#[derive(Debug, PartialEq)]
pub struct VaultTodo {
    pub note: PathBuf,
    pub line: usize,
    pub headings: Vec<String>,
    pub todo: Todo,
}

pub struct Vault {
    root: PathBuf,
    ignored: Vec<PathBuf>,
}

impl Vault {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            ignored: Vec::new(),
        }
    }

    pub fn ignoring(mut self, folders: Vec<PathBuf>) -> Self {
        self.ignored = folders;
        self
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn todos(&self) -> io::Result<Vec<VaultTodo>> {
        let mut todos = Vec::new();

        for note in self.notes()? {
            let content = fs::read_to_string(self.root.join(&note))?;
            todos.extend(todos_in(&note, &content));
        }

        Ok(todos)
    }

    pub fn projects(&self, home: &Path) -> io::Result<Vec<Project>> {
        let mut projects = Vec::new();

        for note in self.notes()? {
            let content = fs::read_to_string(self.root.join(&note))?;
            let repos = repos_in(&content, home);
            if !repos.is_empty() {
                projects.push(Project { note, repos });
            }
        }

        Ok(projects)
    }

    pub fn append_line(&self, note: &Path, line: &str) -> io::Result<()> {
        let path = self.root.join(note);
        let existing = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
            Err(error) => return Err(error),
        };
        let separator = if existing.is_empty() || existing.ends_with('\n') {
            ""
        } else {
            "\n"
        };

        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        writeln!(file, "{separator}{line}")
    }

    pub fn replace_todo(&self, found: &VaultTodo, updated: &Todo) -> io::Result<()> {
        let path = self.root.join(&found.note);
        let content = fs::read_to_string(&path)?;
        let mut lines: Vec<&str> = content.split('\n').collect();
        let index = found.line - 1;

        let current = lines
            .get(index)
            .copied()
            .filter(|line| Todo::parse(line).as_ref() == Some(&found.todo))
            .ok_or_else(|| io::Error::other(CHANGED_SINCE_READ))?;

        let indentation = leading_whitespace(current);
        let replacement = format!("{indentation}{}", updated.render());
        lines[index] = &replacement;

        fs::write(path, lines.join("\n"))
    }

    pub fn check_off_and_repeat(
        &self,
        found: &VaultTodo,
        checked: &Todo,
        next: &Todo,
    ) -> io::Result<()> {
        let mut lines = self.lines_of(&found.note)?;
        let index = found.line - 1;
        let current = lines
            .get(index)
            .filter(|line| Todo::parse(line).as_ref() == Some(&found.todo))
            .ok_or_else(|| io::Error::other(CHANGED_SINCE_READ))?;

        let indentation = leading_whitespace(current).to_string();
        let details = &lines[index + 1..markdown::block_end(&lines, index)];
        let repeated: Vec<String> = std::iter::once(format!("{indentation}{}", next.render()))
            .chain(details.iter().map(|line| reopened(line)))
            .collect();

        lines[index] = format!("{indentation}{}", checked.render());
        lines.splice(index..index, repeated);
        self.write_lines(&found.note, &lines)
    }

    pub fn notes_with_section(&self, heading: &str) -> io::Result<Vec<PathBuf>> {
        let mut found = Vec::new();
        for note in self.notes()? {
            if markdown::has_section(&fs::read_to_string(self.root.join(&note))?, heading) {
                found.push(note);
            }
        }
        Ok(found)
    }

    pub fn add_to_section(&self, note: &Path, heading: &str, line: &str) -> io::Result<()> {
        let mut lines = self.lines_of(note)?;
        if !markdown::insert_under_section(&mut lines, heading, vec![line.to_string()]) {
            return Err(missing_section(note, heading));
        }
        self.write_lines(note, &lines)
    }

    pub fn move_todo(
        &self,
        found: &VaultTodo,
        destination: &Path,
        heading: &str,
    ) -> io::Result<()> {
        if found.note == destination {
            return Err(io::Error::other(format!(
                "'{}' is already in {}",
                found.todo.text,
                destination.display()
            )));
        }

        let mut source = self.lines_of(&found.note)?;
        let index = found.line - 1;
        let still_there = source
            .get(index)
            .and_then(|line| Todo::parse(line))
            .as_ref()
            == Some(&found.todo);
        if !still_there {
            return Err(io::Error::other(CHANGED_SINCE_READ));
        }

        let block = markdown::take_block(&mut source, index);
        let mut target = self.lines_of(destination)?;
        if !markdown::insert_under_section(&mut target, heading, block) {
            return Err(missing_section(destination, heading));
        }

        self.write_lines(destination, &target)?;
        self.write_lines(&found.note, &source)
    }

    fn lines_of(&self, note: &Path) -> io::Result<Vec<String>> {
        let content = fs::read_to_string(self.root.join(note))?;
        Ok(content.lines().map(String::from).collect())
    }

    fn write_lines(&self, note: &Path, lines: &[String]) -> io::Result<()> {
        fs::write(self.root.join(note), lines.join("\n") + "\n")
    }

    fn notes(&self) -> io::Result<Vec<PathBuf>> {
        let mut notes = Vec::new();

        for entry in WalkDir::new(&self.root)
            .into_iter()
            .filter_entry(|entry| is_visible(entry) && !self.is_ignored(entry.path()))
        {
            let entry = entry?;
            if is_markdown(entry.path()) {
                notes.push(entry.path().strip_prefix(&self.root).unwrap().to_path_buf());
            }
        }

        notes.sort();
        Ok(notes)
    }

    fn is_ignored(&self, path: &Path) -> bool {
        path.strip_prefix(&self.root).is_ok_and(|relative| {
            self.ignored
                .iter()
                .any(|folder| relative.starts_with(folder))
        })
    }
}

fn leading_whitespace(line: &str) -> &str {
    &line[..line.len() - line.trim_start().len()]
}

fn reopened(line: &str) -> String {
    let Some(todo) = Todo::parse(line).filter(|todo| todo.done) else {
        return line.to_string();
    };
    let indentation = leading_whitespace(line);
    let open = Todo {
        done: false,
        completed: None,
        ..todo
    };
    format!("{indentation}{}", open.render())
}

fn todos_in(note: &Path, content: &str) -> Vec<VaultTodo> {
    let mut todos = Vec::new();
    let mut headings: Vec<(usize, String)> = Vec::new();
    let mut inside_code = false;

    for (index, line) in content.lines().enumerate() {
        if markdown::is_code_fence(line) {
            inside_code = !inside_code;
            continue;
        }
        if inside_code {
            continue;
        }
        if let Some(level) = markdown::heading_level(line) {
            headings.retain(|(open_level, _)| *open_level < level);
            if level > 1 {
                headings.push((level, line.trim_end().to_string()));
            }
            continue;
        }
        if let Some(todo) = Todo::parse(line) {
            todos.push(VaultTodo {
                note: note.to_path_buf(),
                line: index + 1,
                headings: headings
                    .iter()
                    .map(|(_, heading)| heading.clone())
                    .collect(),
                todo,
            });
        }
    }
    todos
}

fn is_visible(entry: &DirEntry) -> bool {
    entry.depth() == 0 || !entry.file_name().to_string_lossy().starts_with('.')
}

fn is_markdown(path: &Path) -> bool {
    path.is_file() && path.extension().is_some_and(|extension| extension == "md")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn vault_with(notes: &[(&str, &str)]) -> TempDir {
        let directory = TempDir::new().unwrap();
        for (path, content) in notes {
            let full_path = directory.path().join(path);
            fs::create_dir_all(full_path.parent().unwrap()).unwrap();
            fs::write(full_path, content).unwrap();
        }
        directory
    }

    fn locations(todos: &[VaultTodo]) -> Vec<(String, usize, &str)> {
        todos
            .iter()
            .map(|found| {
                (
                    found.note.display().to_string(),
                    found.line,
                    found.todo.text.as_str(),
                )
            })
            .collect()
    }

    #[test]
    fn finds_todos_in_nested_notes_with_their_line_numbers() {
        let directory = vault_with(&[
            ("Inbox.md", "# Inbox\n\n- [ ] Call the plumber\n"),
            (
                "Projects/Garden/Garden.md",
                "# Garden\n## Tasks\n- [ ] Water the plants\n- [x] Buy seeds\n",
            ),
        ]);

        let todos = Vault::new(directory.path()).todos().unwrap();

        assert_eq!(
            locations(&todos),
            vec![
                ("Inbox.md".to_string(), 3, "Call the plumber"),
                (
                    "Projects/Garden/Garden.md".to_string(),
                    3,
                    "Water the plants"
                ),
                ("Projects/Garden/Garden.md".to_string(), 4, "Buy seeds"),
            ]
        );
    }

    #[test]
    fn remembers_the_headings_a_todo_sits_under() {
        let directory = vault_with(&[(
            "Garden.md",
            "# Garden\n## Running\n### Spring\n- [ ] Rake\n## Todo\n- [ ] Buy seeds\n",
        )]);

        let todos = Vault::new(directory.path()).todos().unwrap();

        assert_eq!(todos[0].headings, vec!["## Running", "### Spring"]);
        assert_eq!(todos[1].headings, vec!["## Todo"]);
    }

    #[test]
    fn ignores_todos_and_headings_inside_code_blocks() {
        let directory = vault_with(&[(
            "Notes.md",
            "## Todo\n```markdown\n## Example\n- [ ] not a real todo\n```\n- [ ] real todo\n",
        )]);

        let todos = Vault::new(directory.path()).todos().unwrap();

        assert_eq!(
            locations(&todos),
            vec![("Notes.md".to_string(), 6, "real todo")]
        );
        assert_eq!(todos[0].headings, vec!["## Todo"]);
    }

    #[test]
    fn ignores_files_that_are_not_markdown() {
        let directory = vault_with(&[("notes.txt", "- [ ] not markdown\n")]);

        assert!(Vault::new(directory.path()).todos().unwrap().is_empty());
    }

    #[test]
    fn ignores_hidden_folders() {
        let directory = vault_with(&[
            (".obsidian/plugin.md", "- [ ] plugin\n"),
            (".trash/old.md", "- [ ] thrown away\n"),
        ]);

        assert!(Vault::new(directory.path()).todos().unwrap().is_empty());
    }

    #[test]
    fn skips_ignored_folders_and_everything_inside_them() {
        let directory = vault_with(&[
            ("Templates/Project.md", "- [ ] template todo\n"),
            ("Archive/2024/Old.md", "- [ ] old todo\n"),
            (
                "Projects/Archive.md",
                "- [ ] a note that is merely called Archive\n",
            ),
        ]);

        let todos = Vault::new(directory.path())
            .ignoring(vec![PathBuf::from("Templates"), PathBuf::from("Archive")])
            .todos()
            .unwrap();

        assert_eq!(
            locations(&todos),
            vec![(
                "Projects/Archive.md".to_string(),
                1,
                "a note that is merely called Archive"
            )]
        );
    }

    #[test]
    fn lists_only_notes_that_name_repositories_as_projects() {
        let directory = vault_with(&[
            (
                "Projects/Garden/Garden.md",
                "---\nrepos:\n  - ~/code/garden\n---\n# Garden\n",
            ),
            ("Inbox.md", "# Inbox\n"),
        ]);

        let projects = Vault::new(directory.path())
            .projects(Path::new("/home/someone"))
            .unwrap();

        assert_eq!(
            projects,
            vec![Project {
                note: PathBuf::from("Projects/Garden/Garden.md"),
                repos: vec![PathBuf::from("/home/someone/code/garden")],
            }]
        );
    }

    #[test]
    fn a_missing_vault_is_an_error() {
        assert!(Vault::new("/does/not/exist").todos().is_err());
    }

    fn appended(existing: Option<&str>) -> String {
        let directory = TempDir::new().unwrap();
        let note = Path::new("Inbox.md");
        if let Some(content) = existing {
            fs::write(directory.path().join(note), content).unwrap();
        }

        Vault::new(directory.path())
            .append_line(note, "- [ ] Call the plumber")
            .unwrap();
        fs::read_to_string(directory.path().join(note)).unwrap()
    }

    #[test]
    fn appends_a_line_below_the_existing_content() {
        assert_eq!(
            appended(Some("# Inbox\n\n")),
            "# Inbox\n\n- [ ] Call the plumber\n"
        );
    }

    #[test]
    fn starts_a_new_line_when_the_note_does_not_end_with_one() {
        assert_eq!(
            appended(Some("# Inbox\n- [ ] Buy milk")),
            "# Inbox\n- [ ] Buy milk\n- [ ] Call the plumber\n"
        );
    }

    #[test]
    fn creates_the_note_when_it_does_not_exist() {
        assert_eq!(appended(None), "- [ ] Call the plumber\n");
    }

    const GARDEN: &str = "# Garden\n\n- [ ] Water the plants\n\t- [ ] Buy a hose\n\t\t- details\n";

    fn replaced(
        content: &str,
        line_text: &str,
        update: impl FnOnce(&mut Todo),
    ) -> io::Result<String> {
        let directory = vault_with(&[("Garden.md", content)]);
        let vault = Vault::new(directory.path());
        let found = vault
            .todos()
            .unwrap()
            .into_iter()
            .find(|found| found.todo.text == line_text)
            .unwrap();

        let mut updated = found.todo.clone();
        update(&mut updated);
        vault.replace_todo(&found, &updated)?;
        Ok(fs::read_to_string(directory.path().join("Garden.md")).unwrap())
    }

    #[test]
    fn replaces_only_the_chosen_todo() {
        assert_eq!(
            replaced(GARDEN, "Water the plants", |todo| todo.done = true).unwrap(),
            "# Garden\n\n- [x] Water the plants\n\t- [ ] Buy a hose\n\t\t- details\n"
        );
    }

    #[test]
    fn keeps_the_indentation_of_a_nested_todo() {
        assert_eq!(
            replaced(GARDEN, "Buy a hose", |todo| todo.done = true).unwrap(),
            "# Garden\n\n- [ ] Water the plants\n\t- [x] Buy a hose\n\t\t- details\n"
        );
    }

    #[test]
    fn refuses_when_the_line_changed_in_the_meantime() {
        let directory = vault_with(&[("Garden.md", GARDEN)]);
        let vault = Vault::new(directory.path());
        let found = vault.todos().unwrap().remove(0);
        fs::write(
            directory.path().join("Garden.md"),
            "# Garden\n\nsomething else\n",
        )
        .unwrap();

        assert!(vault.replace_todo(&found, &found.todo).is_err());
    }

    const TASKS: &str = "## Tasks";

    fn garden_and_inbox() -> TempDir {
        vault_with(&[
            (
                "Inbox.md",
                "# Inbox\n\n- [ ] Buy a hose\n\t- the long one\n- [ ] Call the plumber\n",
            ),
            ("Garden.md", "# Garden\n## Tasks\n\n## Links\n"),
            ("Kitchen.md", "# Kitchen\n"),
        ])
    }

    fn read(directory: &TempDir, note: &str) -> String {
        fs::read_to_string(directory.path().join(note)).unwrap()
    }

    #[test]
    fn finds_the_notes_that_have_the_section() {
        let directory = garden_and_inbox();

        assert_eq!(
            Vault::new(directory.path())
                .notes_with_section(TASKS)
                .unwrap(),
            vec![PathBuf::from("Garden.md")]
        );
    }

    #[test]
    fn moves_a_todo_with_its_details_into_the_section() {
        let directory = garden_and_inbox();
        let vault = Vault::new(directory.path());
        let hose = vault.todos().unwrap().remove(0);

        vault
            .move_todo(&hose, Path::new("Garden.md"), TASKS)
            .unwrap();

        assert_eq!(
            read(&directory, "Garden.md"),
            "# Garden\n## Tasks\n\n- [ ] Buy a hose\n\t- the long one\n\n## Links\n"
        );
        assert_eq!(
            read(&directory, "Inbox.md"),
            "# Inbox\n\n- [ ] Call the plumber\n"
        );
    }

    #[test]
    fn refuses_to_move_into_a_note_without_the_section() {
        let directory = garden_and_inbox();
        let vault = Vault::new(directory.path());
        let hose = vault.todos().unwrap().remove(0);

        let error = vault
            .move_todo(&hose, Path::new("Kitchen.md"), TASKS)
            .unwrap_err();

        assert_eq!(error.to_string(), "Kitchen.md has no '## Tasks' section");
        assert!(read(&directory, "Inbox.md").contains("Buy a hose"));
    }

    #[test]
    fn adds_a_new_line_to_a_section() {
        let directory = garden_and_inbox();

        Vault::new(directory.path())
            .add_to_section(Path::new("Garden.md"), TASKS, "- [ ] Rake the leaves")
            .unwrap();

        assert_eq!(
            read(&directory, "Garden.md"),
            "# Garden\n## Tasks\n\n- [ ] Rake the leaves\n\n## Links\n"
        );
    }

    #[test]
    fn refuses_to_repeat_when_the_line_changed_in_the_meantime() {
        let directory = vault_with(&[("Garden.md", GARDEN)]);
        let vault = Vault::new(directory.path());
        let found = vault.todos().unwrap().remove(0);
        let changed = "# Garden\n\nsomething else\n";
        fs::write(directory.path().join("Garden.md"), changed).unwrap();

        let error = vault
            .check_off_and_repeat(&found, &found.todo, &found.todo)
            .unwrap_err();

        assert_eq!(error.to_string(), CHANGED_SINCE_READ);
        assert_eq!(read(&directory, "Garden.md"), changed);
    }

    #[test]
    fn refuses_to_move_when_the_line_changed_in_the_meantime() {
        let directory = garden_and_inbox();
        let vault = Vault::new(directory.path());
        let hose = vault.todos().unwrap().remove(0);
        fs::write(directory.path().join("Inbox.md"), "# Inbox\n").unwrap();

        let error = vault
            .move_todo(&hose, Path::new("Garden.md"), TASKS)
            .unwrap_err();

        assert_eq!(error.to_string(), CHANGED_SINCE_READ);
        assert_eq!(
            read(&directory, "Garden.md"),
            "# Garden\n## Tasks\n\n## Links\n"
        );
    }

    #[test]
    fn refuses_to_move_a_todo_into_its_own_note() {
        let directory = garden_and_inbox();
        let vault = Vault::new(directory.path());
        let hose = vault.todos().unwrap().remove(0);
        let before = read(&directory, "Inbox.md");

        let error = vault
            .move_todo(&hose, Path::new("Inbox.md"), TASKS)
            .unwrap_err();

        assert_eq!(error.to_string(), "'Buy a hose' is already in Inbox.md");
        assert_eq!(read(&directory, "Inbox.md"), before);
    }

    #[test]
    fn refuses_to_add_to_a_note_without_the_section() {
        let directory = garden_and_inbox();

        let error = Vault::new(directory.path())
            .add_to_section(Path::new("Kitchen.md"), TASKS, "- [ ] Fix the tap")
            .unwrap_err();

        assert_eq!(error.to_string(), "Kitchen.md has no '## Tasks' section");
        assert_eq!(read(&directory, "Kitchen.md"), "# Kitchen\n");
    }
}
