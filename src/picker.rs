use std::io;

use inquire::list_option::ListOption;
use inquire::validator::Validation;
use inquire::{CustomUserError, InquireError, MultiSelect, Select, Text};

const PICK_MANY_HELP: &str = "type to filter, space to select, enter to confirm, esc to cancel";
const NOTHING_SELECTED: &str = "press space to select a todo first, or esc to cancel";
const PICK_ONE_HELP: &str = "type to filter, enter to choose, esc to cancel";
const DATE_HELP: &str = "tomorrow, friday, nextweek, thismonth, 25/11, 2026-12-01, every-friday, 1/12 @every-month or none (Dutch works too)";

pub trait Prompt {
    fn interactive(&self) -> bool;
    fn pick_many(&mut self, question: &str, options: Vec<String>) -> io::Result<Vec<usize>>;
    fn pick_one(&mut self, question: &str, options: Vec<String>) -> io::Result<Option<usize>>;
    fn ask_date(&mut self) -> io::Result<Option<String>>;
}

pub struct TerminalPrompt {
    pub interactive: bool,
}

impl Prompt for TerminalPrompt {
    fn interactive(&self) -> bool {
        self.interactive
    }

    fn pick_many(&mut self, question: &str, options: Vec<String>) -> io::Result<Vec<usize>> {
        let picked = MultiSelect::new(question, options)
            .with_help_message(PICK_MANY_HELP)
            .with_validator(at_least_one_selected::<String>)
            .raw_prompt()
            .map(|chosen| chosen.into_iter().map(|option| option.index).collect());
        empty_when_cancelled(picked)
    }

    fn pick_one(&mut self, question: &str, options: Vec<String>) -> io::Result<Option<usize>> {
        let picked = Select::new(question, options)
            .with_help_message(PICK_ONE_HELP)
            .raw_prompt()
            .map(|chosen| Some(chosen.index));
        empty_when_cancelled(picked)
    }

    fn ask_date(&mut self) -> io::Result<Option<String>> {
        let answer = Text::new("New date:")
            .with_help_message(DATE_HELP)
            .prompt()
            .map(Some);
        empty_when_cancelled(answer)
    }
}

fn at_least_one_selected<T>(selected: &[ListOption<&T>]) -> Result<Validation, CustomUserError> {
    if selected.is_empty() {
        Ok(Validation::Invalid(NOTHING_SELECTED.into()))
    } else {
        Ok(Validation::Valid)
    }
}

fn empty_when_cancelled<T: Default>(result: Result<T, InquireError>) -> io::Result<T> {
    match result {
        Ok(value) => Ok(value),
        Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => {
            Ok(T::default())
        }
        Err(error) => Err(io::Error::other(error)),
    }
}

#[cfg(test)]
pub struct ScriptedPrompt {
    pub interactive: bool,
    pub picks: Vec<usize>,
    pub choice: Option<usize>,
    pub date: Option<String>,
    pub shown_options: Vec<String>,
}

#[cfg(test)]
impl ScriptedPrompt {
    pub fn non_interactive() -> Self {
        Self::picking(Vec::new(), None, false)
    }

    pub fn picking(picks: Vec<usize>, date: Option<&str>, interactive: bool) -> Self {
        Self {
            interactive,
            picks,
            choice: None,
            date: date.map(String::from),
            shown_options: Vec::new(),
        }
    }

    pub fn choosing(mut self, choice: usize) -> Self {
        self.choice = Some(choice);
        self
    }
}

#[cfg(test)]
impl Prompt for ScriptedPrompt {
    fn interactive(&self) -> bool {
        self.interactive
    }

    fn pick_many(&mut self, _question: &str, options: Vec<String>) -> io::Result<Vec<usize>> {
        self.shown_options = options;
        Ok(self.picks.clone())
    }

    fn pick_one(&mut self, _question: &str, options: Vec<String>) -> io::Result<Option<usize>> {
        self.shown_options = options;
        Ok(self.choice)
    }

    fn ask_date(&mut self) -> io::Result<Option<String>> {
        Ok(self.date.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inquire::validator::ErrorMessage;

    #[test]
    fn confirming_without_a_selection_is_refused() {
        assert_eq!(
            at_least_one_selected::<String>(&[]).unwrap(),
            Validation::Invalid(ErrorMessage::Custom(NOTHING_SELECTED.to_string()))
        );
    }

    #[test]
    fn confirming_a_selection_is_accepted() {
        let milk = "Buy milk".to_string();

        assert_eq!(
            at_least_one_selected(&[ListOption::new(0, &milk)]).unwrap(),
            Validation::Valid
        );
    }

    #[test]
    fn an_answer_is_passed_on() {
        assert_eq!(
            empty_when_cancelled(Ok(Some("friday".to_string()))).unwrap(),
            Some("friday".to_string())
        );
    }

    #[test]
    fn escape_or_ctrl_c_answers_nothing() {
        assert_eq!(
            empty_when_cancelled::<Vec<usize>>(Err(InquireError::OperationCanceled)).unwrap(),
            Vec::<usize>::new()
        );
        assert_eq!(
            empty_when_cancelled::<Option<String>>(Err(InquireError::OperationInterrupted))
                .unwrap(),
            None
        );
    }

    #[test]
    fn other_prompt_failures_are_errors() {
        let error = empty_when_cancelled::<Option<String>>(Err(InquireError::NotTTY)).unwrap_err();

        assert_eq!(error.to_string(), InquireError::NotTTY.to_string());
    }
}
