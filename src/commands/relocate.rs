use std::error::Error;
use std::io::Write;

use super::choose::{choose, choose_project};
use super::{bottom_up, open_todos, plain, title_of};
use crate::config::Layout;
use crate::picker::Prompt;
use crate::vault::{Vault, VaultTodo};

pub fn move_todos(
    vault: &Vault,
    todos: &[VaultTodo],
    words: &[String],
    layout: &Layout,
    prompt: &mut impl Prompt,
    out: &mut impl Write,
) -> Result<(), Box<dyn Error>> {
    let (query, project) = split_project(words);
    let chosen = choose(&open_todos(todos), &query, prompt)?;
    if chosen.is_empty() {
        writeln!(out, "Nothing moved")?;
        return Ok(());
    }

    let projects = vault.notes_with_section(&layout.section_heading)?;
    let Some(destination) = choose_project(&projects, project.as_deref(), prompt)? else {
        writeln!(out, "Nothing moved")?;
        return Ok(());
    };
    let title = title_of(destination);

    let mut reports = Vec::new();
    for (order, found) in bottom_up(chosen) {
        let text = plain(&found.todo.text);
        let report = if found.note == *destination {
            format!("Already in {title}: {text}")
        } else {
            vault.move_todo(found, destination, &layout.section_heading)?;
            format!("Moved to {title}: {text}")
        };
        reports.push((order, report));
    }

    reports.sort();
    for (_, report) in reports {
        writeln!(out, "{report}")?;
    }
    Ok(())
}

pub fn split_project(words: &[String]) -> (String, Option<String>) {
    let mut query = Vec::new();
    let mut project = None;
    for word in words
        .iter()
        .flat_map(|argument| argument.split_whitespace())
    {
        match word.strip_prefix('+').filter(|name| !name.is_empty()) {
            Some(name) => project = Some(name.to_string()),
            None => query.push(word),
        }
    }
    (query.join(" "), project)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::picker::ScriptedPrompt;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    const INBOX: &str =
        "# Inbox\n\n- [ ] Buy a hose\n\t- the long one\n- [ ] Call the plumber\n- [ ] Buy seeds\n";
    const GARDEN: &str = "# Garden\n## Tasks\n\n- [ ] Rake the leaves\n";

    fn layout() -> Layout {
        Layout {
            inbox: PathBuf::from("Inbox.md"),
            section_heading: "## Tasks".to_string(),
            agent_heading: "## Agent".to_string(),
        }
    }

    fn words(text: &str) -> Vec<String> {
        text.split_whitespace().map(String::from).collect()
    }

    fn run(text: &str, prompt: &mut ScriptedPrompt) -> (TempDir, Result<String, String>) {
        let directory = TempDir::new().unwrap();
        fs::write(directory.path().join("Inbox.md"), INBOX).unwrap();
        fs::write(directory.path().join("Garden.md"), GARDEN).unwrap();
        let vault = Vault::new(directory.path());
        let todos = vault.todos().unwrap();

        let mut out = Vec::new();
        let result = move_todos(&vault, &todos, &words(text), &layout(), prompt, &mut out)
            .map(|()| String::from_utf8(out).unwrap())
            .map_err(|error| error.to_string());
        (directory, result)
    }

    fn read(directory: &TempDir, note: &str) -> String {
        fs::read_to_string(directory.path().join(note)).unwrap()
    }

    #[test]
    fn moves_a_todo_with_its_details_into_the_project() {
        let (directory, printed) = run("hose +garden", &mut ScriptedPrompt::non_interactive());

        assert_eq!(printed, Ok("Moved to Garden: Buy a hose\n".to_string()));
        assert_eq!(
            read(&directory, "Garden.md"),
            "# Garden\n## Tasks\n\n- [ ] Rake the leaves\n- [ ] Buy a hose\n\t- the long one\n"
        );
        assert_eq!(
            read(&directory, "Inbox.md"),
            "# Inbox\n\n- [ ] Call the plumber\n- [ ] Buy seeds\n"
        );
    }

    #[test]
    fn moves_several_picked_todos_from_the_same_note() {
        let mut prompt = ScriptedPrompt::picking(vec![0, 1], None, true);

        let (directory, printed) = run("buy +garden", &mut prompt);

        assert_eq!(
            printed,
            Ok("Moved to Garden: Buy a hose\nMoved to Garden: Buy seeds\n".to_string())
        );
        assert_eq!(
            read(&directory, "Inbox.md"),
            "# Inbox\n\n- [ ] Call the plumber\n"
        );
        assert!(read(&directory, "Garden.md").contains("- [ ] Buy seeds\n"));
    }

    #[test]
    fn a_todo_already_in_the_project_stays_where_it_is() {
        let (directory, printed) = run("rake +garden", &mut ScriptedPrompt::non_interactive());

        assert_eq!(
            printed,
            Ok("Already in Garden: Rake the leaves\n".to_string())
        );
        assert_eq!(read(&directory, "Garden.md"), GARDEN);
    }

    #[test]
    fn without_a_project_outside_a_terminal_is_an_error() {
        let (directory, result) = run("hose", &mut ScriptedPrompt::non_interactive());

        assert_eq!(result, Err("name a project, e.g. '+garden'".to_string()));
        assert_eq!(read(&directory, "Inbox.md"), INBOX);
    }

    #[test]
    fn split_project_separates_the_plus_word() {
        assert_eq!(
            split_project(&words("buy a hose +garden")),
            ("buy a hose".to_string(), Some("garden".to_string()))
        );
    }

    #[test]
    fn moves_nothing_when_no_todo_is_picked() {
        let (directory, printed) = run(
            "buy +garden",
            &mut ScriptedPrompt::picking(vec![], None, true),
        );

        assert_eq!(printed, Ok("Nothing moved\n".to_string()));
        assert_eq!(read(&directory, "Inbox.md"), INBOX);
    }

    #[test]
    fn moves_nothing_when_no_project_is_picked() {
        let (directory, printed) = run("hose", &mut ScriptedPrompt::picking(vec![], None, true));

        assert_eq!(printed, Ok("Nothing moved\n".to_string()));
        assert_eq!(read(&directory, "Inbox.md"), INBOX);
        assert_eq!(read(&directory, "Garden.md"), GARDEN);
    }
}
