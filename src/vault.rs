use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use walkdir::{DirEntry, WalkDir};

use crate::todo::Todo;

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
    fn a_missing_vault_is_an_error() {
        assert!(Vault::new("/does/not/exist").todos().is_err());
    }
}
