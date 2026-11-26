use std::io::{self, Write};

use chrono::NaiveDate;

use super::{in_notes_matching, note_title};
use crate::dates::Period;
use crate::output::Style;
use crate::vault::VaultTodo;

pub fn recap(
    todos: &[VaultTodo],
    period: &Period,
    project: Option<&str>,
    style: Style,
    out: &mut impl Write,
) -> io::Result<()> {
    let done: Vec<&VaultTodo> = todos.iter().filter(|found| found.todo.done).collect();
    let mut finished: Vec<(NaiveDate, &VaultTodo)> = in_notes_matching(done, project)
        .into_iter()
        .filter_map(|found| found.todo.completed.map(|completed| (completed, found)))
        .filter(|(completed, _)| period.contains(*completed))
        .collect();
    finished.sort_by(|(left_completed, left), (right_completed, right)| {
        right_completed
            .cmp(left_completed)
            .then_with(|| (&left.note, left.line).cmp(&(&right.note, right.line)))
    });

    if finished.is_empty() {
        return writeln!(
            out,
            "Nothing checked off between {} and {}",
            period.from, period.until
        );
    }

    for (completed, found) in finished {
        writeln!(
            out,
            "{completed}  {}  {}",
            style.links(&found.todo.text),
            style.dim(&format!("({})", note_title(found)))
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::todo::Todo;
    use std::path::PathBuf;

    const PLAIN: Style = Style { terminal: false };

    fn date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, day).unwrap()
    }

    fn this_week() -> Period {
        Period {
            from: date(16),
            until: date(20),
        }
    }

    fn found(note: &str, line: &str) -> VaultTodo {
        VaultTodo {
            note: PathBuf::from(note),
            line: 1,
            headings: Vec::new(),
            todo: Todo::parse(line).unwrap(),
        }
    }

    fn render(todos: &[VaultTodo], project: Option<&str>) -> String {
        let mut out = Vec::new();
        recap(todos, &this_week(), project, PLAIN, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    fn sample() -> Vec<VaultTodo> {
        vec![
            found("Inbox.md", "- [x] Buy milk ✅ 2026-11-17"),
            found("Projects/Garden/Garden.md", "- [x] Buy seeds ✅ 2026-11-19"),
            found("Inbox.md", "- [x] Pay the bill ✅ 2026-11-09"),
            found("Inbox.md", "- [ ] Call the plumber"),
            found("Inbox.md", "- [x] Old habit without a date"),
        ]
    }

    #[test]
    fn lists_what_was_checked_off_in_the_period_newest_first() {
        assert_eq!(
            render(&sample(), None),
            "2026-11-19  Buy seeds  (Garden)\n\
             2026-11-17  Buy milk  (Inbox)\n"
        );
    }

    #[test]
    fn can_be_limited_to_one_project() {
        assert_eq!(
            render(&sample(), Some("garden")),
            "2026-11-19  Buy seeds  (Garden)\n"
        );
    }

    #[test]
    fn says_so_when_nothing_was_checked_off() {
        let todos = vec![found("Inbox.md", "- [x] Pay the bill ✅ 2026-11-09")];

        assert_eq!(
            render(&todos, None),
            "Nothing checked off between 2026-11-16 and 2026-11-20\n"
        );
    }

    #[test]
    fn todos_checked_off_on_the_same_day_follow_the_notes() {
        let todos = vec![
            found("Projects/Garden/Garden.md", "- [x] Buy seeds ✅ 2026-11-19"),
            found("Inbox.md", "- [x] Buy milk ✅ 2026-11-19"),
        ];

        assert_eq!(
            render(&todos, None),
            "2026-11-19  Buy milk  (Inbox)\n\
             2026-11-19  Buy seeds  (Garden)\n"
        );
    }
}
