use std::error::Error;
use std::io::Write;

use chrono::NaiveDate;

use super::choose::choose;
use super::{open_todos, plain};
use crate::dates::{Calendar, parse_due};
use crate::picker::Prompt;
use crate::todo::Todo;
use crate::vault::{Vault, VaultTodo};

const DATE_NEEDED: &str = "give a date, e.g. 'vt due milk @tomorrow', or '@none' to clear it";

pub fn tick(
    vault: &Vault,
    todos: &[VaultTodo],
    query: &str,
    today: NaiveDate,
    prompt: &mut impl Prompt,
    out: &mut impl Write,
) -> Result<(), Box<dyn Error>> {
    let chosen = choose(&open_todos(todos), query, prompt)?;
    if chosen.is_empty() {
        writeln!(out, "Nothing checked off")?;
        return Ok(());
    }

    for found in chosen {
        let checked = Todo {
            done: true,
            completed: Some(today),
            ..found.todo.clone()
        };
        vault.replace_todo(found, &checked)?;
        writeln!(out, "Checked off: {}", plain(&found.todo.text))?;
    }
    Ok(())
}

pub fn set_due(
    vault: &Vault,
    todos: &[VaultTodo],
    query: &str,
    when: Option<&str>,
    calendar: &Calendar,
    prompt: &mut impl Prompt,
    out: &mut impl Write,
) -> Result<(), Box<dyn Error>> {
    if when.is_none() && !prompt.interactive() {
        return Err(DATE_NEEDED.into());
    }

    let chosen = choose(&open_todos(todos), query, prompt)?;
    let answer = match (chosen.is_empty(), when) {
        (true, _) => None,
        (false, Some(when)) => Some(when.to_string()),
        (false, None) => prompt.ask_date()?,
    };
    let Some(answer) = answer else {
        writeln!(out, "Nothing changed")?;
        return Ok(());
    };

    let due = parse_due(answer.trim_start_matches('@'), calendar)?;
    for found in chosen {
        let rescheduled = Todo {
            due,
            ..found.todo.clone()
        };
        vault.replace_todo(found, &rescheduled)?;
        match due {
            Some(due) => writeln!(out, "{}: due {due}", plain(&found.todo.text))?,
            None => writeln!(out, "{}: no due date", plain(&found.todo.text))?,
        }
    }
    Ok(())
}

pub fn split_date(words: &[String]) -> (String, Option<String>) {
    let (dates, query): (Vec<&String>, Vec<&String>) =
        words.iter().partition(|word| word.starts_with('@'));
    let query = query
        .iter()
        .map(|word| word.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    (query, dates.last().map(|date| date.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::picker::ScriptedPrompt;
    use std::fs;
    use tempfile::TempDir;

    const INBOX: &str = "# Inbox\n\n- [ ] Buy milk\n- [ ] Call the plumber 📅 2026-11-27\n";

    fn friday() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 20).unwrap()
    }

    struct Scenario {
        directory: TempDir,
        vault: Vault,
        output: Vec<u8>,
    }

    impl Scenario {
        fn new() -> Self {
            let directory = TempDir::new().unwrap();
            fs::write(directory.path().join("Inbox.md"), INBOX).unwrap();
            let vault = Vault::new(directory.path());
            Self {
                directory,
                vault,
                output: Vec::new(),
            }
        }

        fn tick(&mut self, query: &str, prompt: &mut ScriptedPrompt) -> Result<(), Box<dyn Error>> {
            let todos = self.vault.todos().unwrap();
            tick(
                &self.vault,
                &todos,
                query,
                friday(),
                prompt,
                &mut self.output,
            )
        }

        fn set_due(
            &mut self,
            query: &str,
            when: Option<&str>,
            prompt: &mut ScriptedPrompt,
        ) -> Result<(), Box<dyn Error>> {
            let todos = self.vault.todos().unwrap();
            set_due(
                &self.vault,
                &todos,
                query,
                when,
                &Calendar::on(friday()),
                prompt,
                &mut self.output,
            )
        }

        fn inbox(&self) -> String {
            fs::read_to_string(self.directory.path().join("Inbox.md")).unwrap()
        }

        fn printed(&self) -> String {
            String::from_utf8(self.output.clone()).unwrap()
        }
    }

    #[test]
    fn tick_checks_off_the_todo_with_todays_date() {
        let mut scenario = Scenario::new();

        scenario
            .tick("milk", &mut ScriptedPrompt::non_interactive())
            .unwrap();

        assert_eq!(
            scenario.inbox(),
            "# Inbox\n\n- [x] Buy milk ✅ 2026-11-20\n- [ ] Call the plumber 📅 2026-11-27\n"
        );
        assert_eq!(scenario.printed(), "Checked off: Buy milk\n");
    }

    #[test]
    fn tick_can_check_off_several_picked_todos() {
        let mut scenario = Scenario::new();

        scenario
            .tick("", &mut ScriptedPrompt::picking(vec![0, 1], None, true))
            .unwrap();

        assert_eq!(scenario.inbox().matches("- [x]").count(), 2);
    }

    #[test]
    fn due_sets_a_date() {
        let mut scenario = Scenario::new();

        scenario
            .set_due(
                "milk",
                Some("@nextweek"),
                &mut ScriptedPrompt::non_interactive(),
            )
            .unwrap();

        assert!(scenario.inbox().contains("- [ ] Buy milk 📅 2026-11-27\n"));
        assert_eq!(scenario.printed(), "Buy milk: due 2026-11-27\n");
    }

    #[test]
    fn due_replaces_an_existing_date() {
        let mut scenario = Scenario::new();

        scenario
            .set_due(
                "plumber",
                Some("@morgen"),
                &mut ScriptedPrompt::non_interactive(),
            )
            .unwrap();

        assert!(
            scenario
                .inbox()
                .contains("- [ ] Call the plumber 📅 2026-11-21\n")
        );
    }

    #[test]
    fn due_none_clears_the_date() {
        let mut scenario = Scenario::new();

        scenario
            .set_due(
                "plumber",
                Some("@geen"),
                &mut ScriptedPrompt::non_interactive(),
            )
            .unwrap();

        assert!(scenario.inbox().contains("- [ ] Call the plumber\n"));
    }

    #[test]
    fn due_asks_for_the_date_when_none_was_given() {
        let mut scenario = Scenario::new();
        let mut prompt = ScriptedPrompt::picking(vec![], Some("friday"), true);

        scenario.set_due("milk", None, &mut prompt).unwrap();

        assert!(scenario.inbox().contains("- [ ] Buy milk 📅 2026-11-27\n"));
    }

    #[test]
    fn due_without_a_date_outside_a_terminal_is_an_error() {
        let mut scenario = Scenario::new();

        let error = scenario
            .set_due("milk", None, &mut ScriptedPrompt::non_interactive())
            .unwrap_err();

        assert_eq!(error.to_string(), DATE_NEEDED);
        assert_eq!(scenario.inbox(), INBOX);
    }

    #[test]
    fn split_date_takes_the_at_word_out_of_the_query() {
        let words: Vec<String> = ["call", "the", "@friday", "plumber"]
            .iter()
            .map(|word| word.to_string())
            .collect();

        assert_eq!(
            split_date(&words),
            ("call the plumber".to_string(), Some("@friday".to_string()))
        );
    }
}
