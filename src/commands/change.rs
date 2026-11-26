use std::error::Error;
use std::io::Write;

use chrono::NaiveDate;

use super::choose::choose;
use super::{bottom_up, open_todos, plain};
use crate::dates::{Calendar, Schedule, parse_schedule};
use crate::picker::Prompt;
use crate::recurrence::Recurrence;
use crate::todo::Todo;
use crate::vault::{Vault, VaultTodo};

const DATE_NEEDED: &str =
    "give a date, e.g. 'vt due milk @tomorrow' or '@every-friday', or '@none' to clear it";

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

    let mut reports = Vec::new();
    for (order, found) in bottom_up(chosen) {
        let checked = Todo {
            done: true,
            completed: Some(today),
            ..found.todo.clone()
        };
        let text = plain(&found.todo.text);
        let report = match repetition_of(&found.todo, today) {
            Repetition::None => {
                vault.replace_todo(found, &checked)?;
                format!("Checked off: {text}")
            }
            Repetition::Next(due) => {
                let next = Todo {
                    due: Some(due),
                    ..found.todo.clone()
                };
                vault.check_off_and_repeat(found, &checked, &next)?;
                format!("Checked off: {text}, again on {due}")
            }
            Repetition::NotUnderstood(rule) => {
                vault.replace_todo(found, &checked)?;
                format!("Checked off: {text}, but not repeated: vt does not understand '🔁 {rule}'")
            }
        };
        reports.push((order, report));
    }

    reports.sort();
    for (_, report) in reports {
        writeln!(out, "{report}")?;
    }
    Ok(())
}

enum Repetition<'a> {
    None,
    Next(NaiveDate),
    NotUnderstood(&'a str),
}

fn repetition_of(todo: &Todo, today: NaiveDate) -> Repetition<'_> {
    let Some(rule) = todo.recurrence.as_deref() else {
        return Repetition::None;
    };
    let counted_from = todo.due.unwrap_or(today);
    match Recurrence::parse(rule).and_then(|repeat| repeat.next_due(counted_from, today)) {
        Some(due) => Repetition::Next(due),
        None => Repetition::NotUnderstood(rule),
    }
}

pub fn set_due(
    vault: &Vault,
    todos: &[VaultTodo],
    query: &str,
    when: &[String],
    calendar: &Calendar,
    prompt: &mut impl Prompt,
    out: &mut impl Write,
) -> Result<(), Box<dyn Error>> {
    if when.is_empty() && !prompt.interactive() {
        return Err(DATE_NEEDED.into());
    }

    let chosen = choose(&open_todos(todos), query, prompt)?;
    let answer = match (chosen.is_empty(), when.is_empty()) {
        (true, _) => Vec::new(),
        (false, false) => when.to_vec(),
        (false, true) => prompt
            .ask_date()?
            .map(|answer| words_of_answer(&answer))
            .unwrap_or_default(),
    };
    if answer.is_empty() {
        writeln!(out, "Nothing changed")?;
        return Ok(());
    }

    let words: Vec<&str> = answer.iter().map(String::as_str).collect();
    let schedule = parse_schedule(&words, calendar)?;
    for found in chosen {
        let rescheduled = rescheduled(&found.todo, &schedule);
        vault.replace_todo(found, &rescheduled)?;
        writeln!(
            out,
            "{}: {}",
            plain(&found.todo.text),
            describe_schedule(&rescheduled)
        )?;
    }
    Ok(())
}

fn words_of_answer(answer: &str) -> Vec<String> {
    answer
        .split('@')
        .map(str::trim)
        .filter(|word| !word.is_empty())
        .map(String::from)
        .collect()
}

fn rescheduled(todo: &Todo, schedule: &Schedule) -> Todo {
    match schedule {
        Schedule::NoDate => Todo {
            due: None,
            recurrence: None,
            ..todo.clone()
        },
        Schedule::Due(due) => Todo {
            due: Some(*due),
            ..todo.clone()
        },
        Schedule::Repeating { due, recurrence } => Todo {
            due: Some(*due),
            recurrence: Some(recurrence.to_string()),
            ..todo.clone()
        },
    }
}

fn describe_schedule(todo: &Todo) -> String {
    match (todo.due, &todo.recurrence) {
        (Some(due), Some(rule)) => format!("due {due}, repeats {rule}"),
        (Some(due), None) => format!("due {due}"),
        (None, _) => "no due date".to_string(),
    }
}

pub fn split_date(words: &[String]) -> (String, Vec<String>) {
    let (dates, query): (Vec<&String>, Vec<&String>) =
        words.iter().partition(|word| word.starts_with('@'));
    let query = query
        .iter()
        .map(|word| word.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let dates = dates
        .iter()
        .map(|date| date.trim_start_matches('@').to_string())
        .collect();
    (query, dates)
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
            Self::with_inbox(INBOX)
        }

        fn with_inbox(content: &str) -> Self {
            let directory = TempDir::new().unwrap();
            fs::write(directory.path().join("Inbox.md"), content).unwrap();
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
            when: &[&str],
            prompt: &mut ScriptedPrompt,
        ) -> Result<(), Box<dyn Error>> {
            let todos = self.vault.todos().unwrap();
            let when: Vec<String> = when.iter().map(|word| word.to_string()).collect();
            set_due(
                &self.vault,
                &todos,
                query,
                &when,
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
    fn tick_puts_the_next_occurrence_above_a_recurring_todo() {
        let mut scenario =
            Scenario::with_inbox("# Inbox\n\n- [ ] Weekly report 🔁 every week 📅 2026-11-20\n");

        scenario
            .tick("report", &mut ScriptedPrompt::non_interactive())
            .unwrap();

        assert_eq!(
            scenario.inbox(),
            "# Inbox\n\n\
             - [ ] Weekly report 🔁 every week 📅 2026-11-27\n\
             - [x] Weekly report 🔁 every week 📅 2026-11-20 ✅ 2026-11-20\n"
        );
        assert_eq!(
            scenario.printed(),
            "Checked off: Weekly report, again on 2026-11-27\n"
        );
    }

    #[test]
    fn tick_repeats_the_details_with_their_checkboxes_open_again() {
        let mut scenario = Scenario::with_inbox(
            "# Inbox\n\n\
             - [ ] Month close 🔁 every month on the last 📅 2026-11-30\n\
             \t- [x] Check the invoices ✅ 2026-11-19\n\
             \t- send the hours\n\
             - [ ] Buy milk\n",
        );

        scenario
            .tick("month close", &mut ScriptedPrompt::non_interactive())
            .unwrap();

        assert_eq!(
            scenario.inbox(),
            "# Inbox\n\n\
             - [ ] Month close 🔁 every month on the last 📅 2026-12-31\n\
             \t- [ ] Check the invoices\n\
             \t- send the hours\n\
             - [x] Month close 🔁 every month on the last 📅 2026-11-30 ✅ 2026-11-20\n\
             \t- [x] Check the invoices ✅ 2026-11-19\n\
             \t- send the hours\n\
             - [ ] Buy milk\n"
        );
    }

    #[test]
    fn tick_keeps_the_indentation_of_a_nested_recurring_todo() {
        let mut scenario = Scenario::with_inbox(
            "# Inbox\n\n- [ ] Garden\n\t- [ ] Water the plants 🔁 every day 📅 2026-11-20\n",
        );

        scenario
            .tick("water", &mut ScriptedPrompt::non_interactive())
            .unwrap();

        assert_eq!(
            scenario.inbox(),
            "# Inbox\n\n- [ ] Garden\n\
             \t- [ ] Water the plants 🔁 every day 📅 2026-11-21\n\
             \t- [x] Water the plants 🔁 every day 📅 2026-11-20 ✅ 2026-11-20\n"
        );
    }

    #[test]
    fn tick_counts_a_recurring_todo_without_due_date_from_today() {
        let mut scenario =
            Scenario::with_inbox("# Inbox\n\n- [ ] Water the plants 🔁 every 3 days\n");

        scenario
            .tick("water", &mut ScriptedPrompt::non_interactive())
            .unwrap();

        assert!(
            scenario
                .inbox()
                .contains("- [ ] Water the plants 🔁 every 3 days 📅 2026-11-23\n")
        );
    }

    #[test]
    fn tick_only_checks_off_a_recurrence_it_does_not_understand() {
        let mut scenario = Scenario::with_inbox(
            "# Inbox\n\n- [ ] Go to the gym 🔁 every monday and thursday 📅 2026-11-19\n",
        );

        scenario
            .tick("gym", &mut ScriptedPrompt::non_interactive())
            .unwrap();

        assert_eq!(
            scenario.inbox(),
            "# Inbox\n\n- [x] Go to the gym 🔁 every monday and thursday 📅 2026-11-19 ✅ 2026-11-20\n"
        );
        assert_eq!(
            scenario.printed(),
            "Checked off: Go to the gym, but not repeated: vt does not understand '🔁 every monday and thursday'\n"
        );
    }

    #[test]
    fn tick_repeats_several_picked_todos_and_reports_them_in_picked_order() {
        let mut scenario = Scenario::with_inbox(
            "# Inbox\n\n\
             - [ ] Water the plants 🔁 every day 📅 2026-11-20\n\
             - [ ] Weekly report 🔁 every week 📅 2026-11-20\n",
        );

        scenario
            .tick("", &mut ScriptedPrompt::picking(vec![0, 1], None, true))
            .unwrap();

        assert_eq!(
            scenario.inbox(),
            "# Inbox\n\n\
             - [ ] Water the plants 🔁 every day 📅 2026-11-21\n\
             - [x] Water the plants 🔁 every day 📅 2026-11-20 ✅ 2026-11-20\n\
             - [ ] Weekly report 🔁 every week 📅 2026-11-27\n\
             - [x] Weekly report 🔁 every week 📅 2026-11-20 ✅ 2026-11-20\n"
        );
        assert_eq!(
            scenario.printed(),
            "Checked off: Water the plants, again on 2026-11-21\n\
             Checked off: Weekly report, again on 2026-11-27\n"
        );
    }

    #[test]
    fn due_sets_a_date() {
        let mut scenario = Scenario::new();

        scenario
            .set_due(
                "milk",
                &["nextweek"],
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
                &["morgen"],
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
            .set_due("plumber", &["geen"], &mut ScriptedPrompt::non_interactive())
            .unwrap();

        assert!(scenario.inbox().contains("- [ ] Call the plumber\n"));
    }

    #[test]
    fn due_asks_for_the_date_when_none_was_given() {
        let mut scenario = Scenario::new();
        let mut prompt = ScriptedPrompt::picking(vec![], Some("friday"), true);

        scenario.set_due("milk", &[], &mut prompt).unwrap();

        assert!(scenario.inbox().contains("- [ ] Buy milk 📅 2026-11-27\n"));
    }

    #[test]
    fn due_without_a_date_outside_a_terminal_is_an_error() {
        let mut scenario = Scenario::new();

        let error = scenario
            .set_due("milk", &[], &mut ScriptedPrompt::non_interactive())
            .unwrap_err();

        assert_eq!(error.to_string(), DATE_NEEDED);
        assert_eq!(scenario.inbox(), INBOX);
    }

    #[test]
    fn due_with_an_every_word_makes_the_todo_repeat() {
        let mut scenario = Scenario::new();

        scenario
            .set_due(
                "milk",
                &["elke-vrijdag"],
                &mut ScriptedPrompt::non_interactive(),
            )
            .unwrap();

        assert!(
            scenario
                .inbox()
                .contains("- [ ] Buy milk 🔁 every week 📅 2026-11-27\n")
        );
        assert_eq!(
            scenario.printed(),
            "Buy milk: due 2026-11-27, repeats every week\n"
        );
    }

    #[test]
    fn due_with_only_a_date_keeps_the_repetition() {
        let mut scenario =
            Scenario::with_inbox("# Inbox\n\n- [ ] Pay the rent 🔁 every month 📅 2026-11-30\n");

        scenario
            .set_due("rent", &["1/12"], &mut ScriptedPrompt::non_interactive())
            .unwrap();

        assert_eq!(
            scenario.inbox(),
            "# Inbox\n\n- [ ] Pay the rent 🔁 every month 📅 2026-12-01\n"
        );
    }

    #[test]
    fn due_none_also_clears_the_repetition() {
        let mut scenario =
            Scenario::with_inbox("# Inbox\n\n- [ ] Pay the rent 🔁 every month 📅 2026-11-30\n");

        scenario
            .set_due("rent", &["none"], &mut ScriptedPrompt::non_interactive())
            .unwrap();

        assert_eq!(scenario.inbox(), "# Inbox\n\n- [ ] Pay the rent\n");
        assert_eq!(scenario.printed(), "Pay the rent: no due date\n");
    }

    #[test]
    fn due_takes_a_date_and_an_every_word_together() {
        let mut scenario = Scenario::new();

        scenario
            .set_due(
                "milk",
                &["1/12", "every-month"],
                &mut ScriptedPrompt::non_interactive(),
            )
            .unwrap();

        assert!(
            scenario
                .inbox()
                .contains("- [ ] Buy milk 🔁 every month on the 1st 📅 2026-12-01\n")
        );
    }

    #[test]
    fn due_asks_for_a_repetition_in_words_or_with_at_signs() {
        for (answer, line) in [
            (
                "every friday",
                "- [ ] Buy milk 🔁 every week 📅 2026-11-27\n",
            ),
            (
                "@1/12 @every-month",
                "- [ ] Buy milk 🔁 every month on the 1st 📅 2026-12-01\n",
            ),
        ] {
            let mut scenario = Scenario::new();
            let mut prompt = ScriptedPrompt::picking(vec![], Some(answer), true);

            scenario.set_due("milk", &[], &mut prompt).unwrap();

            assert!(scenario.inbox().contains(line), "{answer}");
        }
    }

    #[test]
    fn due_changes_nothing_when_the_answer_is_empty() {
        let mut scenario = Scenario::new();
        let mut prompt = ScriptedPrompt::picking(vec![], Some("  "), true);

        scenario.set_due("milk", &[], &mut prompt).unwrap();

        assert_eq!(scenario.inbox(), INBOX);
        assert_eq!(scenario.printed(), "Nothing changed\n");
    }

    #[test]
    fn split_date_takes_the_at_words_out_of_the_query() {
        let words: Vec<String> = ["call", "@1/12", "the", "@every-month", "plumber"]
            .iter()
            .map(|word| word.to_string())
            .collect();

        assert_eq!(
            split_date(&words),
            (
                "call the plumber".to_string(),
                vec!["1/12".to_string(), "every-month".to_string()]
            )
        );
    }
}
