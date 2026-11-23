use std::fmt;

use chrono::NaiveDate;

use crate::dates::{Calendar, UnknownDate, parse_date};
use crate::links;

#[derive(Debug, PartialEq)]
pub struct NewTodo {
    pub text: String,
    pub due: Option<NaiveDate>,
    pub project: Option<String>,
}

impl NewTodo {
    pub fn line(&self) -> String {
        match self.due {
            Some(due) => format!("- [ ] {} 📅 {due}", self.text),
            None => format!("- [ ] {}", self.text),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ComposeError {
    Empty,
    Date(UnknownDate),
}

impl fmt::Display for ComposeError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Empty => write!(formatter, "nothing to add: give the todo some text"),
            Self::Date(unknown) => unknown.fmt(formatter),
        }
    }
}

impl std::error::Error for ComposeError {}

pub fn compose(words: &[String], calendar: &Calendar) -> Result<NewTodo, ComposeError> {
    let mut text_parts = Vec::new();
    let mut due = None;
    let mut project = None;

    for word in words
        .iter()
        .flat_map(|argument| argument.split_whitespace())
    {
        if let Some(when) = word.strip_prefix('@') {
            due = Some(parse_date(when, calendar).map_err(ComposeError::Date)?);
        } else if let Some(name) = word.strip_prefix('+').filter(|name| !name.is_empty()) {
            project = Some(name.to_string());
        } else if links::is_url(word) {
            text_parts.push(links::markdown_link(word));
        } else {
            text_parts.push(word.to_string());
        }
    }

    if text_parts.is_empty() {
        return Err(ComposeError::Empty);
    }

    Ok(NewTodo {
        text: text_parts.join(" "),
        due,
        project,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn friday() -> Calendar {
        Calendar::on(NaiveDate::from_ymd_opt(2026, 11, 20).unwrap())
    }

    fn words(text: &str) -> Vec<String> {
        text.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn plain_words_become_the_todo_text() {
        let todo = compose(&words("Call the plumber"), &friday()).unwrap();

        assert_eq!(todo.line(), "- [ ] Call the plumber");
    }

    #[test]
    fn an_at_word_becomes_the_due_date() {
        let todo = compose(&words("Call the plumber @nextweek"), &friday()).unwrap();

        assert_eq!(todo.line(), "- [ ] Call the plumber 📅 2026-11-27");
    }

    #[test]
    fn the_due_date_may_be_written_anywhere() {
        let todo = compose(&words("@morgen Call the plumber"), &friday()).unwrap();

        assert_eq!(todo.line(), "- [ ] Call the plumber 📅 2026-11-21");
    }

    #[test]
    fn a_url_becomes_a_short_markdown_link() {
        let todo = compose(
            &words("Review https://gitlab.com/acme/shop/-/merge_requests/12"),
            &friday(),
        )
        .unwrap();

        assert_eq!(
            todo.text,
            "Review [shop!12](https://gitlab.com/acme/shop/-/merge_requests/12)"
        );
    }

    #[test]
    fn a_quoted_sentence_is_split_into_words() {
        let quoted = vec!["Review https://gitlab.com/acme/shop/-/issues/3 @tomorrow".to_string()];
        let todo = compose(&quoted, &friday()).unwrap();

        assert_eq!(
            todo.line(),
            "- [ ] Review [shop#3](https://gitlab.com/acme/shop/-/issues/3) 📅 2026-11-21"
        );
    }

    #[test]
    fn a_plus_word_names_the_project() {
        let todo = compose(&words("Rake the leaves +garden @friday"), &friday()).unwrap();

        assert_eq!(todo.project, Some("garden".to_string()));
        assert_eq!(todo.line(), "- [ ] Rake the leaves 📅 2026-11-27");
    }

    #[test]
    fn a_lone_plus_is_just_text() {
        let todo = compose(&words("Add 1 + 1"), &friday()).unwrap();

        assert_eq!(todo.project, None);
        assert_eq!(todo.text, "Add 1 + 1");
    }

    #[test]
    fn an_unknown_date_is_an_error() {
        assert_eq!(
            compose(&words("Call the plumber @someday"), &friday()),
            Err(ComposeError::Date(UnknownDate("someday".to_string())))
        );
    }

    #[test]
    fn a_todo_needs_some_text() {
        assert_eq!(
            compose(&words("@tomorrow"), &friday()),
            Err(ComposeError::Empty)
        );
    }
}
