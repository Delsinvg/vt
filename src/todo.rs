use chrono::NaiveDate;

const DUE_MARKER: &str = "📅";
const COMPLETED_MARKER: &str = "✅";
const RECURRENCE_MARKER: &str = "🔁";
const DATE_FORMAT: &str = "%Y-%m-%d";

#[derive(Debug, Clone, PartialEq)]
pub struct Todo {
    pub text: String,
    pub done: bool,
    pub recurrence: Option<String>,
    pub due: Option<NaiveDate>,
    pub completed: Option<NaiveDate>,
}

enum Field<'a> {
    Recurrence(&'a str),
    Due(NaiveDate),
    Completed(NaiveDate),
}

impl Todo {
    pub fn parse(line: &str) -> Option<Self> {
        let (done, mut text) = split_checkbox(line.trim_start())?;
        let mut todo = Self {
            text: String::new(),
            done,
            recurrence: None,
            due: None,
            completed: None,
        };

        while let Some((before, field)) = split_trailing_field(text)
            && todo.fill(field)
        {
            text = before;
        }

        todo.text = text.to_string();
        Some(todo)
    }

    fn fill(&mut self, field: Field) -> bool {
        match field {
            Field::Recurrence(rule) if self.recurrence.is_none() => {
                self.recurrence = Some(rule.to_string());
            }
            Field::Due(date) if self.due.is_none() => self.due = Some(date),
            Field::Completed(date) if self.completed.is_none() => self.completed = Some(date),
            _ => return false,
        }
        true
    }

    pub fn render(&self) -> String {
        let checkbox = if self.done { "- [x]" } else { "- [ ]" };
        let mut line = format!("{checkbox} {}", self.text);
        if let Some(rule) = &self.recurrence {
            line.push_str(&format!(" {RECURRENCE_MARKER} {rule}"));
        }
        if let Some(due) = self.due {
            line.push_str(&format!(" {DUE_MARKER} {}", due.format(DATE_FORMAT)));
        }
        if let Some(completed) = self.completed {
            line.push_str(&format!(
                " {COMPLETED_MARKER} {}",
                completed.format(DATE_FORMAT)
            ));
        }
        line
    }
}

fn split_checkbox(line: &str) -> Option<(bool, &str)> {
    let rest = line.strip_prefix("- [")?;
    let (mark, rest) = rest.split_at_checked(1)?;
    let text = rest.strip_prefix("] ")?;

    match mark {
        " " => Some((false, text)),
        "x" | "X" => Some((true, text)),
        _ => None,
    }
}

fn split_trailing_field(text: &str) -> Option<(&str, Field<'_>)> {
    let (position, marker) = [RECURRENCE_MARKER, DUE_MARKER, COMPLETED_MARKER]
        .into_iter()
        .filter_map(|marker| Some((text.rfind(marker)?, marker)))
        .max()?;
    let before = text[..position].trim_end();
    let value = text[position + marker.len()..].trim();

    let field = match marker {
        RECURRENCE_MARKER if looks_like_a_rule(value) => Field::Recurrence(value),
        DUE_MARKER => Field::Due(parse_date(value)?),
        COMPLETED_MARKER => Field::Completed(parse_date(value)?),
        _ => return None,
    };
    Some((before, field))
}

/// @see https://publish.obsidian.md/tasks/Getting+Started/Recurring+Tasks
fn looks_like_a_rule(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || " ,!".contains(character))
}

pub fn with_completion(line: &str, completed: NaiveDate) -> String {
    format!(
        "{} {COMPLETED_MARKER} {}",
        line.trim_end(),
        completed.format(DATE_FORMAT)
    )
}

fn parse_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value, DATE_FORMAT).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn open_todo(text: &str) -> Todo {
        Todo {
            text: text.to_string(),
            done: false,
            recurrence: None,
            due: None,
            completed: None,
        }
    }

    #[test]
    fn parses_an_unchecked_todo() {
        assert_eq!(Todo::parse("- [ ] Buy milk"), Some(open_todo("Buy milk")));
    }

    #[test]
    fn a_marker_inside_the_text_is_not_a_rule() {
        let line = "- [ ] Show `🔁` in the list (with details)";

        let todo = Todo::parse(line).unwrap();

        assert_eq!(todo.recurrence, None);
        assert_eq!(todo.text, "Show `🔁` in the list (with details)");
        assert_eq!(todo.render(), line);
    }

    #[test]
    fn adds_the_completion_date_and_leaves_the_rest_of_the_line_alone() {
        assert_eq!(
            with_completion("- [x] Buy  milk 📅 2026-11-20 ", date(2026, 11, 26)),
            "- [x] Buy  milk 📅 2026-11-20 ✅ 2026-11-26"
        );
    }

    #[test]
    fn parses_a_checked_todo() {
        let todo = Todo::parse("- [x] Buy milk").unwrap();

        assert!(todo.done);
        assert_eq!(todo.text, "Buy milk");
    }

    #[test]
    fn uppercase_x_also_means_done() {
        assert!(Todo::parse("- [X] Buy milk").unwrap().done);
    }

    #[test]
    fn parses_an_indented_todo() {
        assert_eq!(Todo::parse("\t- [ ] detail"), Some(open_todo("detail")));
    }

    #[test]
    fn extracts_the_due_date() {
        let todo = Todo::parse("- [ ] Call the plumber 📅 2026-11-27").unwrap();

        assert_eq!(todo.text, "Call the plumber");
        assert_eq!(todo.due, Some(date(2026, 11, 27)));
    }

    #[test]
    fn extracts_due_and_completed_dates() {
        let todo = Todo::parse("- [x] Call the plumber 📅 2026-11-27 ✅ 2026-11-25").unwrap();

        assert_eq!(todo.text, "Call the plumber");
        assert_eq!(todo.due, Some(date(2026, 11, 27)));
        assert_eq!(todo.completed, Some(date(2026, 11, 25)));
    }

    #[test]
    fn keeps_an_invalid_date_as_text() {
        let todo = Todo::parse("- [ ] Call the plumber 📅 tomorrow").unwrap();

        assert_eq!(todo.text, "Call the plumber 📅 tomorrow");
        assert_eq!(todo.due, None);
    }

    #[test]
    fn plain_text_is_not_a_todo() {
        assert_eq!(Todo::parse("Just some text"), None);
    }

    #[test]
    fn a_heading_is_not_a_todo() {
        assert_eq!(Todo::parse("# Groceries"), None);
    }

    #[test]
    fn a_bullet_without_checkbox_is_not_a_todo() {
        assert_eq!(Todo::parse("- just a bullet"), None);
    }

    #[test]
    fn an_unknown_checkbox_mark_is_not_a_todo() {
        assert_eq!(Todo::parse("- [?] unsure"), None);
    }

    #[test]
    fn renders_an_open_todo() {
        assert_eq!(open_todo("Buy milk").render(), "- [ ] Buy milk");
    }

    #[test]
    fn renders_the_dates_in_the_order_they_are_read() {
        let todo = Todo {
            text: "Call the plumber".to_string(),
            done: true,
            recurrence: Some("every week".to_string()),
            due: Some(date(2026, 11, 27)),
            completed: Some(date(2026, 11, 25)),
        };

        assert_eq!(
            todo.render(),
            "- [x] Call the plumber 🔁 every week 📅 2026-11-27 ✅ 2026-11-25"
        );
    }

    #[test]
    fn extracts_the_recurrence() {
        let todo = Todo::parse("- [ ] Water the plants 🔁 every 3 days 📅 2026-11-27").unwrap();

        assert_eq!(todo.text, "Water the plants");
        assert_eq!(todo.recurrence.as_deref(), Some("every 3 days"));
        assert_eq!(todo.due, Some(date(2026, 11, 27)));
    }

    #[test]
    fn reads_the_fields_in_any_order() {
        let todo = Todo::parse("- [x] Water the plants ✅ 2026-11-25 📅 2026-11-27 🔁 every week")
            .unwrap();

        assert_eq!(todo.text, "Water the plants");
        assert_eq!(todo.recurrence.as_deref(), Some("every week"));
        assert_eq!(todo.due, Some(date(2026, 11, 27)));
        assert_eq!(todo.completed, Some(date(2026, 11, 25)));
    }

    #[test]
    fn keeps_a_recurrence_it_does_not_understand() {
        let line = "- [ ] Water the plants 🔁 every monday and thursday 📅 2026-11-27";

        assert_eq!(Todo::parse(line).unwrap().render(), line);
    }

    #[test]
    fn a_repeated_field_stays_in_the_text() {
        let todo = Todo::parse("- [ ] Call 📅 2026-11-20 the plumber 📅 2026-11-27").unwrap();

        assert_eq!(todo.text, "Call 📅 2026-11-20 the plumber");
        assert_eq!(todo.due, Some(date(2026, 11, 27)));
    }

    #[test]
    fn a_lone_recurrence_marker_is_text() {
        let todo = Todo::parse("- [ ] Fix the 🔁").unwrap();

        assert_eq!(todo.text, "Fix the 🔁");
        assert_eq!(todo.recurrence, None);
    }

    #[test]
    fn rendering_and_parsing_give_back_the_same_todo() {
        let line = "- [x] Review [shop!12](https://gitlab.com/acme/shop/-/merge_requests/12) 📅 2026-12-04 ✅ 2026-12-01";

        assert_eq!(Todo::parse(line).unwrap().render(), line);
    }
}
