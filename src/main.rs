mod cli;
mod commands;
mod output;
mod select;
mod todo;
mod vault;

use std::env;
use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

use cli::{Cli, Command};
use output::Style;
use vault::Vault;

const MISSING_VAULT: &str =
    "VAULT is not set. Point it to your notes, e.g. export VAULT=\"$HOME/Documents/notes\"";

fn vault_root() -> io::Result<PathBuf> {
    env::var_os("VAULT")
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::other(MISSING_VAULT))
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("vt: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> io::Result<()> {
    let vault = Vault::new(vault_root()?);
    let mut out = io::stdout().lock();
    let style = Style {
        terminal: out.is_terminal(),
    };

    match cli.command {
        Command::Todo { project } => {
            commands::list_todos(&vault.todos()?, project.as_deref(), style, &mut out)
        }
    }
}
