mod cli;
mod commands;
mod config;
mod dates;
mod inbox;
mod links;
mod output;
mod picker;
mod select;
mod todo;
mod vault;

use std::error::Error;
use std::io::{self, IsTerminal};
use std::path::Path;
use std::process::ExitCode;

use chrono::Local;
use clap::Parser;

use cli::{Cli, Command};
use output::Style;
use picker::TerminalPrompt;
use vault::Vault;

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("vt: {error}");
            ExitCode::FAILURE
        }
    }
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
        Command::Todo { project } => {
            commands::list_todos(&vault.todos()?, project.as_deref(), style, &mut out)?
        }
        Command::Inbox { words } if words.is_empty() => {
            commands::list_todos(&vault.todos()?, Some(config::INBOX_NOTE), style, &mut out)?
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
