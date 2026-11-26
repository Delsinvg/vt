use std::cmp::Ordering;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use chrono::NaiveDate;

use super::{note_name, open_todos, plain, repeat_label, title_of};
use crate::markdown::heading_text;
use crate::output::Style;
use crate::select;
use crate::vault::VaultTodo;

const BULLET: &str = "○";
const UNDER_NOTE: &str = "  ";
const UNDER_HEADING: &str = "    ";

pub enum Scope {
    All,
    Matching(String),
    Folder(PathBuf),
}

pub fn list_todos(
    todos: &[VaultTodo],
    scope: &Scope,
    today: NaiveDate,
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
        let in_note: Vec<&VaultTodo> = open
            .iter()
            .copied()
            .filter(|found| note_name(found) == *note)
            .collect();
        write_note(Path::new(note), &in_note, today, style, out)?;
    }

    Ok(())
}

fn write_note(
    note: &Path,
    todos: &[&VaultTodo],
    today: NaiveDate,
    style: Style,
    out: &mut impl Write,
) -> io::Result<()> {
    writeln!(out, "{}", style.heading(&title_of(note)))?;

    let widest = todos
        .iter()
        .map(|found| plain(&found.todo.text).chars().count())
        .max()
        .unwrap_or(0);
    let mut current_heading: Option<&str> = None;

    for found in todos {
        let heading = found.headings.last().map(|heading| heading_text(heading));
        if heading != current_heading {
            if let Some(heading) = heading {
                writeln!(out, "{UNDER_NOTE}{}", style.subheading(heading))?;
            }
            current_heading = heading;
        }

        let indent = if heading.is_some() {
            UNDER_HEADING
        } else {
            UNDER_NOTE
        };
        let labels: Vec<String> = [
            found.todo.due.map(|due| due_label(due, today, style)),
            found
                .todo
                .recurrence
                .as_deref()
                .map(|rule| repeat_label(rule, style)),
        ]
        .into_iter()
        .flatten()
        .collect();
        let text = style.links(&found.todo.text);
        if labels.is_empty() {
            writeln!(out, "{indent}{BULLET} {text}")?;
        } else {
            let padding = widest - plain(&found.todo.text).chars().count();
            writeln!(
                out,
                "{indent}{BULLET} {text}{}  {}",
                " ".repeat(padding),
                labels.join("  ")
            )?;
        }
    }
    Ok(())
}

fn due_label(due: NaiveDate, today: NaiveDate, style: Style) -> String {
    let label = format!("📅 {due}");
    match due.cmp(&today) {
        Ordering::Less => style.overdue(&label),
        Ordering::Equal => style.due_today(&label),
        Ordering::Greater => style.dim(&label),
    }
}

fn unique_note_names(todos: &[&VaultTodo]) -> Vec<String> {
    let mut names: Vec<String> = todos.iter().map(|found| note_name(found)).collect();
    names.dedup();
    names
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::todo::Todo;

    const PLAIN: Style = Style { terminal: false };

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 20).unwrap()
    }

    fn found(note: &str, headings: &[&str], line: &str) -> VaultTodo {
        VaultTodo {
            note: PathBuf::from(note),
            line: 1,
            headings: headings.iter().map(|heading| heading.to_string()).collect(),
            todo: Todo::parse(line).unwrap(),
        }
    }

    fn render(todos: &[VaultTodo], scope: Scope) -> String {
        let mut out = Vec::new();
        list_todos(todos, &scope, today(), PLAIN, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    fn sample() -> Vec<VaultTodo> {
        vec![
            found("Inbox.md", &[], "- [ ] Call the plumber"),
            found(
                "Projects/Garden/Garden.md",
                &["## Running", "### Spring"],
                "- [ ] Rake the leaves",
            ),
            found(
                "Projects/Garden/Garden.md",
                &["## Running", "### Spring"],
                "- [ ] Buy seeds 📅 2026-11-27",
            ),
            found(
                "Projects/Garden/Garden.md",
                &["## Todo"],
                "- [ ] Water the plants 🔁 every 3 days 📅 2026-11-18",
            ),
            found(
                "Projects/Garden/Garden.md",
                &["## Todo"],
                "- [x] Mow the lawn",
            ),
            found(
                "Projects/Kitchen/Kitchen.md",
                &["## Todo"],
                "- [ ] Fix the tap",
            ),
        ]
    }

    #[test]
    fn groups_todos_per_note_and_heading_with_aligned_dates() {
        assert_eq!(
            render(&sample(), Scope::All),
            "Inbox\n\
             \x20 ○ Call the plumber\n\
             \n\
             Garden\n\
             \x20 Spring\n\
             \x20   ○ Rake the leaves\n\
             \x20   ○ Buy seeds         📅 2026-11-27\n\
             \x20 Todo\n\
             \x20   ○ Water the plants  📅 2026-11-18  🔁 every 3 days\n\
             \n\
             Kitchen\n\
             \x20 Todo\n\
             \x20   ○ Fix the tap\n"
        );
    }

    #[test]
    fn a_repetition_without_due_date_is_shown_after_the_text() {
        let todos = vec![
            found("Inbox.md", &[], "- [ ] Go to the gym 🔁 every week"),
            found("Inbox.md", &[], "- [ ] Call the plumber"),
        ];

        assert_eq!(
            render(&todos, Scope::All),
            "Inbox\n  ○ Go to the gym     🔁 every week\n  ○ Call the plumber\n"
        );
    }

    #[test]
    fn shows_only_the_matching_project() {
        assert_eq!(
            render(&sample(), Scope::Matching("kitchn".to_string())),
            "Kitchen\n  Todo\n    ○ Fix the tap\n"
        );
    }

    #[test]
    fn a_note_named_differently_from_its_folder_shows_the_folder() {
        let todos = vec![found(
            "Projects/Garden/Notes.md",
            &[],
            "- [ ] Ask about tiles",
        )];

        assert_eq!(
            render(&todos, Scope::All),
            "Garden/Notes\n  ○ Ask about tiles\n"
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
        let todos = vec![found("Inbox.md", &[], "- [x] Buy seeds")];

        assert_eq!(render(&todos, Scope::All), "No open todos\n");
    }

    #[test]
    fn a_folder_shows_every_note_inside_it() {
        let mut todos = sample();
        todos.push(found(
            "Projects/Garden/Notes.md",
            &[],
            "- [ ] Ask about tiles",
        ));

        let rendered = render(&todos, Scope::Folder(PathBuf::from("Projects/Garden")));

        assert!(rendered.starts_with("Garden\n"));
        assert!(rendered.contains("\nGarden/Notes\n  ○ Ask about tiles\n"));
        assert!(!rendered.contains("Kitchen"));
    }

    #[test]
    fn an_empty_folder_points_to_all() {
        assert_eq!(
            render(&sample(), Scope::Folder(PathBuf::from("Projects/Attic"))),
            "No open todos in Projects/Attic; 'vt todo --all' shows everything\n"
        );
    }
}
