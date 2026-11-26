use std::error::Error;
use std::fs;
use std::io::{self, Write};

use super::choose::choose_one;
use super::{note_title, open_todos, plain};
use crate::markdown;
use crate::opener::{Opener, Spot};
use crate::picker::Prompt;
use crate::vault::{Vault, VaultTodo};

pub fn open_todo(
    vault: &Vault,
    todos: &[VaultTodo],
    query: &str,
    opener: &Opener,
    prompt: &mut impl Prompt,
    launch: impl FnOnce(&[String]) -> io::Result<()>,
    out: &mut impl Write,
) -> Result<(), Box<dyn Error>> {
    let Some(found) = choose_one(&open_todos(todos), query, prompt)? else {
        writeln!(out, "Nothing opened")?;
        return Ok(());
    };

    let note = fs::canonicalize(vault.root().join(&found.note))?;
    let spot = Spot {
        note: &note,
        line: found.line,
        heading: found
            .headings
            .last()
            .map(|heading| markdown::heading_text(heading)),
    };
    launch(&opener.command_line(&spot))?;
    writeln!(
        out,
        "Opened {}: {}",
        note_title(found),
        plain(&found.todo.text)
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opener::{SYSTEM_OPENER, percent_encoded};
    use crate::picker::ScriptedPrompt;
    use tempfile::TempDir;

    const GARDEN: &str = "# Garden\n## Todo\n- [ ] Rake the leaves\n### Spring\n- [ ] Sow the beans\n- [x] Buy seeds ✅ 2026-11-20\n";

    fn run(
        query: &str,
        opener: &Opener,
        prompt: &mut ScriptedPrompt,
    ) -> (Vec<String>, Result<String, String>) {
        let directory = TempDir::new().unwrap();
        fs::write(directory.path().join("Garden.md"), GARDEN).unwrap();
        fs::write(
            directory.path().join("Inbox.md"),
            "- [ ] Call the plumber\n",
        )
        .unwrap();
        let vault = Vault::new(directory.path());
        let todos = vault.todos().unwrap();
        let root = fs::canonicalize(directory.path()).unwrap();
        let root = root.to_string_lossy();
        let without_root = |argument: &String| {
            argument
                .replace(&percent_encoded(&root), "ROOT")
                .replace(root.as_ref(), "ROOT")
        };

        let mut launched = Vec::new();
        let mut out = Vec::new();
        let result = open_todo(
            &vault,
            &todos,
            query,
            opener,
            prompt,
            |command_line| {
                launched = command_line.iter().map(without_root).collect();
                Ok(())
            },
            &mut out,
        )
        .map(|()| String::from_utf8(out).unwrap())
        .map_err(|error| error.to_string());
        (launched, result)
    }

    fn in_obsidian(query: &str) -> (Vec<String>, Result<String, String>) {
        run(
            query,
            &Opener::Obsidian,
            &mut ScriptedPrompt::non_interactive(),
        )
    }

    #[test]
    fn opens_the_note_in_obsidian_at_the_heading_the_todo_sits_under() {
        let (launched, printed) = in_obsidian("beans");

        assert_eq!(
            launched,
            vec![
                SYSTEM_OPENER,
                "obsidian://open?path=ROOT%2FGarden.md%23Spring"
            ]
        );
        assert_eq!(printed, Ok("Opened Garden: Sow the beans\n".to_string()));
    }

    #[test]
    fn a_todo_without_a_heading_opens_the_top_of_the_note() {
        let (launched, _) = in_obsidian("plumber");

        assert_eq!(
            launched,
            vec![SYSTEM_OPENER, "obsidian://open?path=ROOT%2FInbox.md"]
        );
    }

    #[test]
    fn an_editor_opens_the_note_at_the_line_of_the_todo() {
        let editor = Opener::from_template("nvim +{line} {file}").unwrap();

        let (launched, printed) = run("beans", &editor, &mut ScriptedPrompt::non_interactive());

        assert_eq!(launched, vec!["nvim", "+5", "ROOT/Garden.md"]);
        assert_eq!(printed, Ok("Opened Garden: Sow the beans\n".to_string()));
    }

    #[test]
    fn several_matches_let_you_pick_one() {
        let mut prompt = ScriptedPrompt::picking(vec![], None, true).choosing(1);

        let (launched, printed) = run("", &Opener::Obsidian, &mut prompt);

        assert_eq!(prompt.shown_options.len(), 3);
        assert_eq!(
            launched,
            vec![
                SYSTEM_OPENER,
                "obsidian://open?path=ROOT%2FGarden.md%23Spring"
            ]
        );
        assert_eq!(printed, Ok("Opened Garden: Sow the beans\n".to_string()));
    }

    #[test]
    fn opens_nothing_when_no_todo_is_picked() {
        let (launched, printed) = run(
            "",
            &Opener::Obsidian,
            &mut ScriptedPrompt::picking(vec![], None, true),
        );

        assert!(launched.is_empty());
        assert_eq!(printed, Ok("Nothing opened\n".to_string()));
    }

    #[test]
    fn checked_off_todos_are_not_offered() {
        let (launched, result) = in_obsidian("seeds");

        assert!(launched.is_empty());
        assert_eq!(result, Err("no open todo matches 'seeds'".to_string()));
    }

    #[test]
    fn a_failing_launch_is_reported() {
        let directory = TempDir::new().unwrap();
        fs::write(
            directory.path().join("Inbox.md"),
            "- [ ] Call the plumber\n",
        )
        .unwrap();
        let vault = Vault::new(directory.path());

        let error = open_todo(
            &vault,
            &vault.todos().unwrap(),
            "plumber",
            &Opener::Obsidian,
            &mut ScriptedPrompt::non_interactive(),
            |_| Err(io::Error::other("Obsidian is not installed")),
            &mut Vec::new(),
        )
        .unwrap_err();

        assert_eq!(error.to_string(), "Obsidian is not installed");
    }
}
