use chrono::NaiveDate;

const DUE_MARKER: &str = "📅";
const COMPLETED_MARKER: &str = "✅";
const DATE_FORMAT: &str = "%Y-%m-%d";

#[derive(Debug, Clone, PartialEq)]
pub struct Todo {
    pub text: String,
    pub done: bool,
    pub due: Option<NaiveDate>,
    pub completed: Option<NaiveDate>,
}

impl Todo {
    pub fn parse(line: &str) -> Option<Self> {
        let (done, rest) = split_checkbox(line.trim_start())?;
        let (rest, completed) = split_trailing_date(rest, COMPLETED_MARKER);
        let (text, due) = split_trailing_date(rest, DUE_MARKER);

        Some(Self {
            text: text.to_string(),
            done,
            due,
            completed,
        })
    }

    pub fn render(&self) -> String {
        let checkbox = if self.done { "- [x]" } else { "- [ ]" };
        let mut line = format!("{checkbox} {}", self.text);
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

fn split_trailing_date<'a>(text: &'a str, marker: &str) -> (&'a str, Option<NaiveDate>) {
    let Some((before, after)) = text.rsplit_once(marker) else {
        return (text, None);
    };

    match NaiveDate::parse_from_str(after.trim(), DATE_FORMAT) {
        Ok(date) => (before.trim_end(), Some(date)),
        Err(_) => (text, None),
    }
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
            due: None,
            completed: None,
        }
    }

    #[test]
    fn parses_an_unchecked_todo() {
        assert_eq!(Todo::parse("- [ ] Buy milk"), Some(open_todo("Buy milk")));
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
            due: Some(date(2026, 11, 27)),
            completed: Some(date(2026, 11, 25)),
        };

        assert_eq!(
            todo.render(),
            "- [x] Call the plumber 📅 2026-11-27 ✅ 2026-11-25"
        );
    }

    #[test]
    fn rendering_and_parsing_give_back_the_same_todo() {
        let line = "- [x] Review [shop!12](https://gitlab.com/acme/shop/-/merge_requests/12) 📅 2026-12-04 ✅ 2026-12-01";

        assert_eq!(Todo::parse(line).unwrap().render(), line);
    }
}
