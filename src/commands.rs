mod add;
mod change;
mod choose;
mod list;

pub use add::add_to_inbox;
pub use change::{set_due, split_date, tick};
pub use list::{Scope, list_todos};

use crate::output::Style;
use crate::vault::VaultTodo;

fn open_todos(todos: &[VaultTodo]) -> Vec<&VaultTodo> {
    todos.iter().filter(|found| !found.todo.done).collect()
}

fn plain(text: &str) -> String {
    Style { terminal: false }.links(text)
}
