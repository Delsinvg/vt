use std::io::{self, Write};
use std::path::PathBuf;

use super::open_todos;
use crate::output::Style;
use crate::select;
use crate::todo::Todo;
use crate::vault::VaultTodo;

pub enum Scope {
    All,
    Matching(String),
    Folder(PathBuf),
}

pub fn list_todos(
    todos: &[VaultTodo],
    scope: &Scope,
    style: Style,
    out: &mut impl Write,
) -> io::Result<()> {
    let open = open_todos(todos);
    let note_names = unique_note_names(&open);
    let candidates: Vec<&str> = note_names.iter().map(String::as_str).collect();

    let shown = match scope {
        Scope::All => candidates,
        Scope::Matching(query) => select::matching(&candidates, query),
        Scope::Folder(folder) => candidates
            .into_iter()
            .filter(|note| PathBuf::from(note).starts_with(folder))
            .collect(),
    };

    if shown.is_empty() {
        return match scope {
            Scope::All => writeln!(out, "No open todos"),
            Scope::Matching(query) => {
                writeln!(out, "No open todos in a note matching '{query}'")
            }
            Scope::Folder(folder) => writeln!(
                out,
                "No open todos in {}; 'vt todo --all' shows everything",
                folder.display()
            ),
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

    fn render(todos: &[VaultTodo], scope: Scope) -> String {
        let mut out = Vec::new();
        list_todos(todos, &scope, PLAIN, &mut out).unwrap();
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
            render(&sample(), Scope::All),
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
            render(&sample(), Scope::Matching("kitchn".to_string())),
            "Projects/Kitchen/Kitchen.md\n- [ ] Fix the tap\n"
        );
    }

    #[test]
    fn shows_the_due_date() {
        let mut todos = vec![found("Inbox.md", "Call the plumber", false)];
        todos[0].todo.due = NaiveDate::from_ymd_opt(2026, 12, 1);

        assert_eq!(
            render(&todos, Scope::All),
            "Inbox.md\n- [ ] Call the plumber 📅 2026-12-01\n"
        );
    }

    #[test]
    fn says_so_when_no_note_matches() {
        assert_eq!(
            render(&sample(), Scope::Matching("zzz".to_string())),
            "No open todos in a note matching 'zzz'\n"
        );
    }

    #[test]
    fn says_so_when_everything_is_done() {
        let todos = vec![found("Inbox.md", "Buy seeds", true)];

        assert_eq!(render(&todos, Scope::All), "No open todos\n");
    }

    #[test]
    fn a_folder_shows_every_note_inside_it() {
        let mut todos = sample();
        todos.push(found("Projects/Garden/Notes.md", "Ask about tiles", false));

        assert_eq!(
            render(&todos, Scope::Folder(PathBuf::from("Projects/Garden"))),
            "Projects/Garden/Garden.md\n\
             - [ ] Water the plants\n\
             \n\
             Projects/Garden/Notes.md\n\
             - [ ] Ask about tiles\n"
        );
    }

    #[test]
    fn an_empty_folder_points_to_all() {
        assert_eq!(
            render(&sample(), Scope::Folder(PathBuf::from("Projects/Attic"))),
            "No open todos in Projects/Attic; 'vt todo --all' shows everything\n"
        );
    }
}
