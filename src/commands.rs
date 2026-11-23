mod add;
mod agenda;
mod change;
mod choose;
mod list;
mod recap;
mod relocate;

pub use add::add_todo;
pub use agenda::agenda;
pub use change::{set_due, split_date, tick};
pub use list::{Scope, list_todos};
pub use recap::recap;
pub use relocate::move_todos;

use std::path::Path;

use crate::output::Style;
use crate::select;
use crate::vault::VaultTodo;

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
    note.file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default()
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
