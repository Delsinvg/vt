mod cli;
mod commands;
mod config;
mod dates;
mod git;
mod inbox;
mod links;
mod output;
mod picker;
mod project;
mod select;
mod todo;
mod vault;

use std::env;
use std::error::Error;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chrono::Local;
use clap::Parser;

use cli::{Cli, Command};
use commands::Scope;
use output::Style;
use picker::TerminalPrompt;
use vault::Vault;

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if reader_stopped_listening(error.as_ref()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("vt: {error}");
            ExitCode::FAILURE
        }
    }
}

fn reader_stopped_listening(error: &(dyn Error + 'static)) -> bool {
    error
        .downcast_ref::<io::Error>()
        .is_some_and(|error| error.kind() == io::ErrorKind::BrokenPipe)
}

fn run(cli: Cli) -> Result<(), Box<dyn Error>> {
    let vault = Vault::new(config::vault_root()?);
    let mut out = io::stdout().lock();
    let style = Style {
        terminal: out.is_terminal(),
    };
    let mut prompt = TerminalPrompt {
        interactive: io::stdin().is_terminal(),
    };
    let today = Local::now().date_naive();

    match cli.command {
        Command::Todo { project, all } => {
            let scope = todo_scope(&vault, project, all)?;
            commands::list_todos(&vault.todos()?, &scope, style, &mut out)?
        }
        Command::Inbox { words } if words.is_empty() => {
            let scope = Scope::Matching(config::INBOX_NOTE.to_string());
            commands::list_todos(&vault.todos()?, &scope, style, &mut out)?
        }
        Command::Inbox { words } => commands::add_to_inbox(
            &vault,
            Path::new(config::INBOX_NOTE),
            &words,
            today,
            &mut out,
        )?,
        Command::Tick { query } => commands::tick(
            &vault,
            &vault.todos()?,
            &query.join(" "),
            today,
            &mut prompt,
            &mut out,
        )?,
        Command::Due { words } => {
            let (query, when) = commands::split_date(&words);
            commands::set_due(
                &vault,
                &vault.todos()?,
                &query,
                when.as_deref(),
                today,
                &mut prompt,
                &mut out,
            )?
        }
    }

    Ok(())
}

fn todo_scope(vault: &Vault, project: Option<String>, all: bool) -> io::Result<Scope> {
    if all {
        return Ok(Scope::All);
    }
    if let Some(query) = project {
        return Ok(Scope::Matching(query));
    }
    Ok(current_project_folder(vault)?.map_or(Scope::All, Scope::Folder))
}

fn current_project_folder(vault: &Vault) -> io::Result<Option<PathBuf>> {
    let Some(repository) = env::current_dir()
        .ok()
        .and_then(|directory| git::repository_root(&directory))
    else {
        return Ok(None);
    };
    let projects = vault.projects(&config::home())?;
    Ok(project::folder_for(&projects, &repository))
}
