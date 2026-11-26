use std::error::Error;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use super::choose::choose_one;
use super::{note_title, open_todos, plain};
use crate::markdown;
use crate::picker::Prompt;
use crate::vault::{Vault, VaultTodo};

pub fn open_todo(
    vault: &Vault,
    todos: &[VaultTodo],
    query: &str,
    prompt: &mut impl Prompt,
    launch: impl FnOnce(&str) -> io::Result<()>,
    out: &mut impl Write,
) -> Result<(), Box<dyn Error>> {
    let Some(found) = choose_one(&open_todos(todos), query, prompt)? else {
        writeln!(out, "Nothing opened")?;
        return Ok(());
    };

    let note = fs::canonicalize(vault.root().join(&found.note))?;
    let nearest_heading = found
        .headings
        .last()
        .map(|heading| markdown::heading_text(heading));
    launch(&obsidian_uri(&note, nearest_heading))?;
    writeln!(
        out,
        "Opened {}: {}",
        note_title(found),
        plain(&found.todo.text)
    )?;
    Ok(())
}

/// @see https://obsidian.md/help/Extending+Obsidian/Obsidian+URI
fn obsidian_uri(note: &Path, heading: Option<&str>) -> String {
    let mut target = note.to_string_lossy().into_owned();
    if let Some(heading) = heading {
        target.push('#');
        target.push_str(heading);
    }
    format!("obsidian://open?path={}", percent_encoded(&target))
}

fn percent_encoded(text: &str) -> String {
    text.bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                char::from(byte).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::picker::ScriptedPrompt;
    use tempfile::TempDir;

    const GARDEN: &str = "# Garden\n## Todo\n- [ ] Rake the leaves\n### Spring\n- [ ] Sow the beans\n- [x] Buy seeds ✅ 2026-11-20\n";

    fn run(query: &str, prompt: &mut ScriptedPrompt) -> (Vec<String>, Result<String, String>) {
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

        let mut launched = Vec::new();
        let mut out = Vec::new();
        let result = open_todo(
            &vault,
            &todos,
            query,
            prompt,
            |uri| {
                launched.push(uri.replace(&percent_encoded(&root.to_string_lossy()), "ROOT"));
                Ok(())
            },
            &mut out,
        )
        .map(|()| String::from_utf8(out).unwrap())
        .map_err(|error| error.to_string());
        (launched, result)
    }

    #[test]
    fn opens_the_note_at_the_heading_the_todo_sits_under() {
        let (launched, printed) = run("beans", &mut ScriptedPrompt::non_interactive());

        assert_eq!(
            launched,
            vec!["obsidian://open?path=ROOT%2FGarden.md%23Spring"]
        );
        assert_eq!(printed, Ok("Opened Garden: Sow the beans\n".to_string()));
    }

    #[test]
    fn a_todo_without_a_heading_opens_the_top_of_the_note() {
        let (launched, _) = run("plumber", &mut ScriptedPrompt::non_interactive());

        assert_eq!(launched, vec!["obsidian://open?path=ROOT%2FInbox.md"]);
    }

    #[test]
    fn several_matches_let_you_pick_one() {
        let mut prompt = ScriptedPrompt::picking(vec![], None, true).choosing(1);

        let (launched, printed) = run("", &mut prompt);

        assert_eq!(prompt.shown_options.len(), 3);
        assert_eq!(
            launched,
            vec!["obsidian://open?path=ROOT%2FGarden.md%23Spring"]
        );
        assert_eq!(printed, Ok("Opened Garden: Sow the beans\n".to_string()));
    }

    #[test]
    fn opens_nothing_when_no_todo_is_picked() {
        let (launched, printed) = run("", &mut ScriptedPrompt::picking(vec![], None, true));

        assert!(launched.is_empty());
        assert_eq!(printed, Ok("Nothing opened\n".to_string()));
    }

    #[test]
    fn checked_off_todos_are_not_offered() {
        let (launched, result) = run("seeds", &mut ScriptedPrompt::non_interactive());

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
            &mut ScriptedPrompt::non_interactive(),
            |_| Err(io::Error::other("Obsidian is not installed")),
            &mut Vec::new(),
        )
        .unwrap_err();

        assert_eq!(error.to_string(), "Obsidian is not installed");
    }

    #[test]
    fn spaces_and_accents_are_encoded() {
        assert_eq!(
            obsidian_uri(Path::new("/notes/Tuin plan.md"), Some("Lente: zaaien")),
            "obsidian://open?path=%2Fnotes%2FTuin%20plan.md%23Lente%3A%20zaaien"
        );
        assert_eq!(percent_encoded("é"), "%C3%A9");
    }
}
