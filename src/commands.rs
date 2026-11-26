mod add;
mod agenda;
mod change;
mod choose;
mod hook;
mod list;
mod recap;
mod relocate;

pub use add::add_todo;
pub use agenda::agenda;
pub use change::{set_due, split_date, tick};
pub use hook::{default_instruction, session_start};
pub use list::{Scope, list_todos};
pub use recap::recap;
pub use relocate::move_todos;

use std::path::Path;

use crate::config::Layout;
use crate::output::Style;
use crate::select;
use crate::vault::VaultTodo;

#[derive(Clone, Copy, PartialEq)]
pub enum Owner {
    Me,
    Agent,
}

pub fn owned_by(todos: Vec<VaultTodo>, layout: &Layout, owner: Owner) -> Vec<VaultTodo> {
    todos
        .into_iter()
        .filter(|found| {
            let in_agent_section = found.headings.contains(&layout.agent_heading);
            in_agent_section == (owner == Owner::Agent)
        })
        .collect()
}

fn bottom_up(chosen: Vec<&VaultTodo>) -> Vec<(usize, &VaultTodo)> {
    let mut ordered: Vec<_> = chosen.into_iter().enumerate().collect();
    ordered
        .sort_by(|(_, left), (_, right)| (&right.note, right.line).cmp(&(&left.note, left.line)));
    ordered
}

fn open_todos(todos: &[VaultTodo]) -> Vec<&VaultTodo> {
    todos.iter().filter(|found| !found.todo.done).collect()
}

fn plain(text: &str) -> String {
    Style { terminal: false }.links(text)
}

fn note_name(found: &VaultTodo) -> String {
    found.note.to_string_lossy().into_owned()
}

fn note_title(found: &VaultTodo) -> String {
    title_of(&found.note)
}

fn title_of(note: &Path) -> String {
    let stem = note
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    let folder = note
        .parent()
        .and_then(Path::file_name)
        .map(|folder| folder.to_string_lossy().into_owned());

    match folder {
        Some(folder) if folder != stem => format!("{folder}/{stem}"),
        _ => stem,
    }
}

fn in_notes_matching<'a>(todos: Vec<&'a VaultTodo>, query: Option<&str>) -> Vec<&'a VaultTodo> {
    let Some(query) = query else {
        return todos;
    };

    let mut names: Vec<String> = todos.iter().map(|found| note_name(found)).collect();
    names.sort();
    names.dedup();
    let candidates: Vec<&str> = names.iter().map(String::as_str).collect();
    let wanted = select::matching(&candidates, query);

    todos
        .into_iter()
        .filter(|found| wanted.contains(&note_name(found).as_str()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::todo::Todo;
    use std::path::PathBuf;

    fn found(headings: &[&str], text: &str) -> VaultTodo {
        VaultTodo {
            note: PathBuf::from("Garden.md"),
            line: 1,
            headings: headings.iter().map(|heading| heading.to_string()).collect(),
            todo: Todo::parse(&format!("- [ ] {text}")).unwrap(),
        }
    }

    fn layout() -> Layout {
        Layout {
            inbox: PathBuf::from("Inbox.md"),
            section_heading: "## Todo".to_string(),
            agent_heading: "## Agent".to_string(),
        }
    }

    fn texts(todos: Vec<VaultTodo>) -> Vec<String> {
        todos.into_iter().map(|found| found.todo.text).collect()
    }

    fn sample() -> Vec<VaultTodo> {
        vec![
            found(&["## Todo"], "Buy seeds"),
            found(&["## Agent"], "Refactor the watering schedule"),
            found(&["## Agent", "### Branch 2"], "Add tests for the schedule"),
            found(
                &["## Running", "### Agent notes"],
                "A heading that merely mentions agent",
            ),
        ]
    }

    #[test]
    fn my_todos_leave_out_the_agent_section() {
        assert_eq!(
            texts(owned_by(sample(), &layout(), Owner::Me)),
            vec!["Buy seeds", "A heading that merely mentions agent"]
        );
    }

    #[test]
    fn the_agent_todos_include_its_subsections() {
        assert_eq!(
            texts(owned_by(sample(), &layout(), Owner::Agent)),
            vec![
                "Refactor the watering schedule",
                "Add tests for the schedule"
            ]
        );
    }
}
