mod cli;
mod commands;
mod config;
mod dates;
mod output;
mod select;
mod todo;
mod vault;

use std::io::{self, IsTerminal};
use std::process::ExitCode;

use clap::Parser;

use cli::{Cli, Command};
use output::Style;
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

fn run(cli: Cli) -> io::Result<()> {
    let vault = Vault::new(config::vault_root()?);
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
