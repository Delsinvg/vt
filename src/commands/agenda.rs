use std::cmp::Ordering;
use std::io::{self, Write};

use chrono::NaiveDate;

use super::{in_notes_matching, note_title, open_todos, repeat_label};
use crate::output::Style;
use crate::vault::VaultTodo;

const OVERDUE: &str = "overdue";
const TODAY: &str = "today";
const LABEL_WIDTH: usize = 7;

pub fn agenda(
    todos: &[VaultTodo],
    project: Option<&str>,
    today: NaiveDate,
    style: Style,
    out: &mut impl Write,
) -> io::Result<()> {
    let mut dated: Vec<(NaiveDate, &VaultTodo)> = in_notes_matching(open_todos(todos), project)
        .into_iter()
        .filter_map(|found| found.todo.due.map(|due| (due, found)))
        .collect();
    dated.sort_by(|(left_due, left), (right_due, right)| {
        (left_due, &left.note, left.line).cmp(&(right_due, &right.note, right.line))
    });

    if dated.is_empty() {
        return writeln!(out, "No open todos with a due date");
    }

    for (due, found) in dated {
        let date = due.to_string();
        let (date, label) = match due.cmp(&today) {
            Ordering::Less => (
                style.overdue(&date),
                style.overdue(&format!("{OVERDUE:<LABEL_WIDTH$}")),
            ),
            Ordering::Equal => (
                style.due_today(&date),
                style.due_today(&format!("{TODAY:<LABEL_WIDTH$}")),
            ),
            Ordering::Greater => (date, " ".repeat(LABEL_WIDTH)),
        };
        let repeats = match found.todo.recurrence.as_deref() {
            Some(rule) => format!("  {}", repeat_label(rule, style)),
            None => String::new(),
        };
        writeln!(
            out,
            "{date}  {label}  {}{repeats}  {}",
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

    fn friday() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 20).unwrap()
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
        agenda(todos, project, friday(), PLAIN, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    fn sample() -> Vec<VaultTodo> {
        vec![
            found(
                "Projects/Garden/Garden.md",
                "- [ ] Water the plants 🔁 every 3 days 📅 2026-11-27",
            ),
            found("Inbox.md", "- [ ] Call the plumber 📅 2026-11-20"),
            found("Inbox.md", "- [ ] Buy milk"),
            found(
                "Projects/Kitchen/Kitchen.md",
                "- [ ] Fix the tap 📅 2026-11-18",
            ),
            found("Inbox.md", "- [x] Pay the bill 📅 2026-11-10 ✅ 2026-11-09"),
        ]
    }

    #[test]
    fn lists_open_dated_todos_earliest_first_with_labels() {
        assert_eq!(
            render(&sample(), None),
            "2026-11-18  overdue  Fix the tap  (Kitchen)\n\
             2026-11-20  today    Call the plumber  (Inbox)\n\
             2026-11-27           Water the plants  🔁 every 3 days  (Garden)\n"
        );
    }

    #[test]
    fn can_be_limited_to_one_project() {
        assert_eq!(
            render(&sample(), Some("garden")),
            "2026-11-27           Water the plants  🔁 every 3 days  (Garden)\n"
        );
    }

    #[test]
    fn says_so_when_nothing_has_a_date() {
        let todos = vec![found("Inbox.md", "- [ ] Buy milk")];

        assert_eq!(render(&todos, None), "No open todos with a due date\n");
    }
}
