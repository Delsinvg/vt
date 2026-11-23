use std::io;

use inquire::{InquireError, MultiSelect, Select, Text};

const PICK_MANY_HELP: &str = "type to filter, space to select, enter to confirm, esc to cancel";
const PICK_ONE_HELP: &str = "type to filter, enter to choose, esc to cancel";
const DATE_HELP: &str =
    "tomorrow, friday, nextweek, thismonth, 25/11, 2026-12-01 or none (Dutch works too)";

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
