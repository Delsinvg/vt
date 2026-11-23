use std::error::Error;
use std::io::Write;

use super::choose::choose_project;
use super::{plain, title_of};
use crate::config::Layout;
use crate::dates::Calendar;
use crate::inbox::{self, NewTodo};
use crate::picker::Prompt;
use crate::vault::Vault;

pub fn add_todo(
    vault: &Vault,
    layout: &Layout,
    words: &[String],
    calendar: &Calendar,
    prompt: &mut impl Prompt,
    out: &mut impl Write,
) -> Result<(), Box<dyn Error>> {
    let todo = inbox::compose(words, calendar)?;

    if let Some(query) = &todo.project {
        let projects = vault.notes_with_section(&layout.section_heading)?;
        match choose_project(&projects, Some(query), prompt) {
            Ok(Some(note)) => {
                vault.add_to_section(note, &layout.section_heading, &todo.line())?;
                return report(out, &title_of(note), &todo);
            }
            Ok(None) => writeln!(out, "No project chosen, so it goes to the inbox")?,
            Err(reason) => writeln!(out, "{reason}, so it goes to the inbox")?,
        }
    }

    vault.append_line(&layout.inbox, &todo.line())?;
    report(out, "inbox", &todo)
}

fn report(out: &mut impl Write, place: &str, todo: &NewTodo) -> Result<(), Box<dyn Error>> {
    let text = plain(&todo.text);
    match todo.due {
        Some(due) => writeln!(out, "Added to {place}: {text} (due {due})")?,
        None => writeln!(out, "Added to {place}: {text}")?,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::picker::ScriptedPrompt;
    use chrono::NaiveDate;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn friday() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 20).unwrap()
    }

    fn layout() -> Layout {
        Layout {
            inbox: PathBuf::from("Inbox.md"),
            section_heading: "## Tasks".to_string(),
        }
    }

    fn add(text: &str) -> (TempDir, String) {
        let directory = TempDir::new().unwrap();
        fs::write(directory.path().join("Inbox.md"), "# Inbox\n").unwrap();
        fs::write(directory.path().join("Garden.md"), "# Garden\n## Tasks\n").unwrap();
        let words: Vec<String> = text.split_whitespace().map(String::from).collect();

        let mut out = Vec::new();
        add_todo(
            &Vault::new(directory.path()),
            &layout(),
            &words,
            &Calendar::on(friday()),
            &mut ScriptedPrompt::non_interactive(),
            &mut out,
        )
        .unwrap();
        (directory, String::from_utf8(out).unwrap())
    }

    fn read(directory: &TempDir, note: &str) -> String {
        fs::read_to_string(directory.path().join(note)).unwrap()
    }

    #[test]
    fn without_a_project_the_todo_goes_to_the_inbox() {
        let (directory, printed) = add("Call the plumber");

        assert_eq!(
            read(&directory, "Inbox.md"),
            "# Inbox\n- [ ] Call the plumber\n"
        );
        assert_eq!(printed, "Added to inbox: Call the plumber\n");
    }

    #[test]
    fn with_a_project_the_todo_goes_to_its_section() {
        let (directory, printed) = add("Rake the leaves +garden @friday");

        assert_eq!(
            read(&directory, "Garden.md"),
            "# Garden\n## Tasks\n\n- [ ] Rake the leaves 📅 2026-11-27\n"
        );
        assert_eq!(read(&directory, "Inbox.md"), "# Inbox\n");
        assert_eq!(
            printed,
            "Added to Garden: Rake the leaves (due 2026-11-27)\n"
        );
    }

    #[test]
    fn an_unknown_project_falls_back_to_the_inbox() {
        let (directory, printed) = add("Fix the roof +attic");

        assert_eq!(
            read(&directory, "Inbox.md"),
            "# Inbox\n- [ ] Fix the roof\n"
        );
        assert_eq!(
            printed,
            "no project matches 'attic', so it goes to the inbox\nAdded to inbox: Fix the roof\n"
        );
    }
}
