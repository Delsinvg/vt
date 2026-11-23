use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::git;
use crate::project;
use crate::vault::Vault;

const MAX_NOTE_CHARACTERS: usize = 8000;
const SHORTENED: &str = "\n\n[shortened: read the full note]";
pub const DEFAULT_INSTRUCTION: &str = "In your first reply, tell the user in two or three sentences where this branch stands: its status, what it waits for and its open todos. Keep the note current as agreed in your instructions.";

pub fn session_start(
    vault: Option<&Vault>,
    instruction: &str,
    home: &Path,
    input: impl Read,
    out: &mut impl Write,
) -> io::Result<()> {
    let Some(vault) = vault else {
        return Ok(());
    };
    let Some(context) = context_for(vault, instruction, home, &working_directory(input)) else {
        return Ok(());
    };

    let output = json!({
        "hookSpecificOutput": {
            "hookEventName": "SessionStart",
            "additionalContext": context,
        }
    });
    writeln!(out, "{output}")
}

fn working_directory(mut input: impl Read) -> PathBuf {
    let mut raw = String::new();
    let from_claude = input
        .read_to_string(&mut raw)
        .ok()
        .and_then(|_| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|event| event["cwd"].as_str().map(PathBuf::from));

    from_claude
        .or_else(|| env::current_dir().ok())
        .unwrap_or_default()
}

fn context_for(vault: &Vault, instruction: &str, home: &Path, directory: &Path) -> Option<String> {
    let repository = git::repository_root(directory)?;
    let projects = vault.projects(home).ok()?;

    let Some(project) = project::owner_of(&projects, &repository) else {
        return Some(format!(
            "No project note in the vault ({}) lists the repository {} under 'repos:'. If this looks like an active project, offer to create one.",
            vault.root().display(),
            repository.display()
        ));
    };

    let note = vault.root().join(&project.note);
    let content = fs::read_to_string(&note).ok()?;
    let worktree = git::worktree_root(directory).unwrap_or(repository);
    let branch =
        git::current_branch(directory).unwrap_or_else(|| "none (detached HEAD)".to_string());

    Some(format!(
        "Project note: {}\nWorktree: {}\nBranch: {branch}\n\n{instruction}\n\n{}",
        note.display(),
        worktree.display(),
        shortened(&content)
    ))
}

fn shortened(content: &str) -> String {
    match content.char_indices().nth(MAX_NOTE_CHARACTERS) {
        Some((cut, _)) => format!("{}{SHORTENED}", &content[..cut]),
        None => content.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    struct Setup {
        vault: TempDir,
        code: TempDir,
    }

    fn run_git(directory: &Path, arguments: &[&str]) {
        let output = Command::new("git")
            .arg("-C")
            .arg(directory)
            .args(["-c", "user.name=Test", "-c", "user.email=test@example.com"])
            .args(arguments)
            .output()
            .unwrap();
        assert!(output.status.success(), "git {arguments:?} failed");
    }

    fn setup(note_lists_the_repository: bool) -> Setup {
        let vault = TempDir::new().unwrap();
        let code = TempDir::new().unwrap();
        let repository = code.path().join("garden");
        fs::create_dir_all(&repository).unwrap();
        run_git(&repository, &["init", "--quiet", "--initial-branch=main"]);
        run_git(
            &repository,
            &["commit", "--quiet", "--allow-empty", "-m", "start"],
        );

        let frontmatter = if note_lists_the_repository {
            format!("---\nrepos:\n  - {}\n---\n", repository.display())
        } else {
            String::new()
        };
        fs::create_dir_all(vault.path().join("Projects/Garden")).unwrap();
        fs::write(
            vault.path().join("Projects/Garden/Garden.md"),
            format!("{frontmatter}# Garden\n\n- [ ] Water the plants\n"),
        )
        .unwrap();
        Setup { vault, code }
    }

    fn run(vault: Option<&Path>, directory: &Path) -> String {
        run_with(vault, DEFAULT_INSTRUCTION, directory)
    }

    fn run_with(vault: Option<&Path>, instruction: &str, directory: &Path) -> String {
        let input = json!({ "cwd": directory }).to_string();
        let vault = vault.map(Vault::new);
        let mut out = Vec::new();
        session_start(
            vault.as_ref(),
            instruction,
            Path::new("/home/someone"),
            input.as_bytes(),
            &mut out,
        )
        .unwrap();
        String::from_utf8(out).unwrap()
    }

    fn context(output: &str) -> String {
        let event: Value = serde_json::from_str(output).unwrap();
        event["hookSpecificOutput"]["additionalContext"]
            .as_str()
            .unwrap()
            .to_string()
    }

    #[test]
    fn gives_claude_the_project_note_of_the_repository() {
        let setup = setup(true);

        let output = run(Some(setup.vault.path()), &setup.code.path().join("garden"));

        let event: Value = serde_json::from_str(&output).unwrap();
        assert_eq!(event["hookSpecificOutput"]["hookEventName"], "SessionStart");
        let context = context(&output);
        assert!(context.contains("Projects/Garden/Garden.md"));
        assert!(context.contains("Branch: main"));
        assert!(context.contains("- [ ] Water the plants"));
    }

    #[test]
    fn uses_the_configured_instruction() {
        let setup = setup(true);

        let output = run_with(
            Some(setup.vault.path()),
            "Only list the open todos.",
            &setup.code.path().join("garden"),
        );

        let context = context(&output);
        assert!(context.contains("Only list the open todos."));
        assert!(!context.contains(DEFAULT_INSTRUCTION));
    }

    #[test]
    fn mentions_when_no_note_lists_the_repository() {
        let setup = setup(false);

        let output = run(Some(setup.vault.path()), &setup.code.path().join("garden"));

        assert!(context(&output).starts_with("No project note in the vault"));
    }

    #[test]
    fn says_nothing_outside_a_repository() {
        let setup = setup(true);

        assert_eq!(run(Some(setup.vault.path()), setup.code.path()), "");
    }

    #[test]
    fn says_nothing_without_a_vault() {
        let setup = setup(true);

        assert_eq!(run(None, &setup.code.path().join("garden")), "");
    }

    #[test]
    fn shortens_a_very_long_note() {
        let long = "é".repeat(MAX_NOTE_CHARACTERS + 10);

        let result = shortened(&long);

        assert!(result.ends_with(SHORTENED));
        assert_eq!(
            result.chars().count(),
            MAX_NOTE_CHARACTERS + SHORTENED.chars().count()
        );
    }
}
