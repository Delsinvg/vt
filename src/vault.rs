use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use walkdir::{DirEntry, WalkDir};

use crate::project::{Project, repos_in};
use crate::todo::Todo;

const CHANGED_SINCE_READ: &str = "the note changed while vt was running; run the command again";

#[derive(Debug, PartialEq)]
pub struct VaultTodo {
    pub note: PathBuf,
    pub line: usize,
    pub todo: Todo,
}

pub struct Vault {
    root: PathBuf,
}

impl Vault {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
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

        let indentation = &current[..current.len() - current.trim_start().len()];
        let replacement = format!("{indentation}{}", updated.render());
        lines[index] = &replacement;

        fs::write(path, lines.join("\n"))
    }

    fn notes(&self) -> io::Result<Vec<PathBuf>> {
        let mut notes = Vec::new();

        for entry in WalkDir::new(&self.root)
            .into_iter()
            .filter_entry(is_visible)
        {
            let entry = entry?;
            if is_markdown(entry.path()) {
                notes.push(entry.path().strip_prefix(&self.root).unwrap().to_path_buf());
            }
        }

        notes.sort();
        Ok(notes)
    }
}

fn todos_in<'a>(note: &'a Path, content: &'a str) -> impl Iterator<Item = VaultTodo> + 'a {
    content.lines().enumerate().filter_map(|(index, line)| {
        Todo::parse(line).map(|todo| VaultTodo {
            note: note.to_path_buf(),
            line: index + 1,
            todo,
        })
    })
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
}
