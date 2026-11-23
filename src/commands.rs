use std::error::Error;
use std::io::{self, Write};
use std::path::Path;

use chrono::NaiveDate;

use crate::inbox;
use crate::output::Style;
use crate::select;
use crate::todo::Todo;
use crate::vault::{Vault, VaultTodo};

pub fn add_to_inbox(
    vault: &Vault,
    inbox_note: &Path,
    words: &[String],
    today: NaiveDate,
    out: &mut impl Write,
) -> Result<(), Box<dyn Error>> {
    let todo = inbox::compose(words, today)?;
    vault.append_line(inbox_note, &todo.line())?;

    let text = Style { terminal: false }.links(&todo.text);
    match todo.due {
        Some(due) => writeln!(out, "Added to inbox: {text} (due {due})")?,
        None => writeln!(out, "Added to inbox: {text}")?,
    }
    Ok(())
}

pub fn list_todos(
    todos: &[VaultTodo],
    project: Option<&str>,
    style: Style,
    out: &mut impl Write,
) -> io::Result<()> {
    let open: Vec<&VaultTodo> = todos.iter().filter(|found| !found.todo.done).collect();
    let note_names = unique_note_names(&open);
    let candidates: Vec<&str> = note_names.iter().map(String::as_str).collect();

    let shown = match project {
        Some(query) => select::matching(&candidates, query),
        None => candidates,
    };

    if shown.is_empty() {
        return match project {
            Some(query) => writeln!(out, "No open todos in a note matching '{query}'"),
            None => writeln!(out, "No open todos"),
        };
    }

    for (position, note) in shown.iter().enumerate() {
        if position > 0 {
            writeln!(out)?;
        }
        writeln!(out, "{}", style.heading(note))?;
        for found in open.iter().filter(|found| note_name(found) == *note) {
            writeln!(
                out,
                "{} {}",
                style.checkbox(false),
                describe(&found.todo, style)
            )?;
        }
    }

    Ok(())
}

fn unique_note_names(todos: &[&VaultTodo]) -> Vec<String> {
    let mut names: Vec<String> = todos.iter().map(|found| note_name(found)).collect();
    names.dedup();
    names
}

fn note_name(found: &VaultTodo) -> String {
    found.note.to_string_lossy().into_owned()
}

fn describe(todo: &Todo, style: Style) -> String {
    let text = style.links(&todo.text);
    match todo.due {
        Some(due) => format!("{text} 📅 {due}"),
        None => text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use std::path::PathBuf;

    const PLAIN: Style = Style { terminal: false };

    fn found(note: &str, text: &str, done: bool) -> VaultTodo {
        VaultTodo {
            note: PathBuf::from(note),
            line: 1,
            todo: Todo {
                text: text.to_string(),
                done,
                due: None,
                completed: None,
            },
        }
    }

    fn render(todos: &[VaultTodo], project: Option<&str>) -> String {
        let mut out = Vec::new();
        list_todos(todos, project, PLAIN, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    fn sample() -> Vec<VaultTodo> {
        vec![
            found("Inbox.md", "Call the plumber", false),
            found("Projects/Garden/Garden.md", "Water the plants", false),
            found("Projects/Garden/Garden.md", "Buy seeds", true),
            found("Projects/Kitchen/Kitchen.md", "Fix the tap", false),
        ]
    }

    #[test]
    fn groups_open_todos_per_note() {
        assert_eq!(
            render(&sample(), None),
            "Inbox.md\n\
             - [ ] Call the plumber\n\
             \n\
             Projects/Garden/Garden.md\n\
             - [ ] Water the plants\n\
             \n\
             Projects/Kitchen/Kitchen.md\n\
             - [ ] Fix the tap\n"
        );
    }

    #[test]
    fn shows_only_the_matching_project() {
        assert_eq!(
            render(&sample(), Some("kitchn")),
            "Projects/Kitchen/Kitchen.md\n- [ ] Fix the tap\n"
        );
    }

    #[test]
    fn shows_the_due_date() {
        let mut todos = vec![found("Inbox.md", "Call the plumber", false)];
        todos[0].todo.due = NaiveDate::from_ymd_opt(2026, 12, 1);

        assert_eq!(
            render(&todos, None),
            "Inbox.md\n- [ ] Call the plumber 📅 2026-12-01\n"
        );
    }

    #[test]
    fn says_so_when_no_note_matches() {
        assert_eq!(
            render(&sample(), Some("zzz")),
            "No open todos in a note matching 'zzz'\n"
        );
    }

    #[test]
    fn says_so_when_everything_is_done() {
        let todos = vec![found("Inbox.md", "Buy seeds", true)];

        assert_eq!(render(&todos, None), "No open todos\n");
    }
}
