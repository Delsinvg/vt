mod cli;
mod commands;
mod config;
mod dates;
mod git;
mod inbox;
mod links;
mod markdown;
mod output;
mod picker;
mod project;
mod recurrence;
mod select;
mod todo;
mod vault;

use std::env;
use std::error::Error;
use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::process::ExitCode;

use chrono::Local;
use clap::Parser;

use cli::{Cli, Command};
use commands::{Owner, Scope};
use dates::Calendar;
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
    if let Command::Hook = cli.command {
        run_hook();
        return Ok(());
    }

    let settings = config::load()?;
    let vault = Vault::new(settings.vault_root()?).ignoring(settings.ignored);
    let layout = settings.layout;
    let mut out = io::stdout().lock();
    let style = Style {
        terminal: out.is_terminal(),
    };
    let mut prompt = TerminalPrompt {
        interactive: io::stdin().is_terminal(),
    };
    let today = Local::now().date_naive();
    let calendar = Calendar {
        today,
        week_ends_on: settings.week_ends_on,
        month_ends_on: settings.month_ends_on,
    };
    let todos_of =
        |owner| -> io::Result<_> { Ok(commands::owned_by(vault.todos()?, &layout, owner)) };

    match cli.command {
        Command::Hook => {}
        Command::Todo {
            project,
            all,
            agent,
        } => {
            let scope = todo_scope(&vault, project, all)?;
            let owner = if agent { Owner::Agent } else { Owner::Me };
            commands::list_todos(&todos_of(owner)?, &scope, today, style, &mut out)?
        }
        Command::Inbox { words } if words.is_empty() => {
            let scope = Scope::Matching(layout.inbox.display().to_string());
            commands::list_todos(&todos_of(Owner::Me)?, &scope, today, style, &mut out)?
        }
        Command::Inbox { words } => {
            commands::add_todo(&vault, &layout, &words, &calendar, &mut prompt, &mut out)?
        }
        Command::Move { words } => commands::move_todos(
            &vault,
            &todos_of(Owner::Me)?,
            &words,
            &layout,
            &mut prompt,
            &mut out,
        )?,
        Command::Tick { query } => commands::tick(
            &vault,
            &todos_of(Owner::Me)?,
            &query.join(" "),
            today,
            &mut prompt,
            &mut out,
        )?,
        Command::Due { words } => {
            let (query, when) = commands::split_date(&words);
            commands::set_due(
                &vault,
                &todos_of(Owner::Me)?,
                &query,
                when.as_deref(),
                &calendar,
                &mut prompt,
                &mut out,
            )?
        }
        Command::Agenda { project } => commands::agenda(
            &todos_of(Owner::Me)?,
            project.as_deref(),
            today,
            style,
            &mut out,
        )?,
        Command::Recap { period, project } => {
            let period = dates::parse_period(&period, &calendar)?;
            commands::recap(
                &todos_of(Owner::Me)?,
                &period,
                project.as_deref(),
                style,
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

fn run_hook() {
    let Ok(settings) = config::load() else {
        return;
    };
    let vault = settings
        .vault_root()
        .ok()
        .map(|root| Vault::new(root).ignoring(settings.ignored.clone()));
    let instruction = settings
        .hook_instruction
        .clone()
        .unwrap_or_else(|| commands::default_instruction(&settings.layout.agent_heading));

    let mut out = io::stdout().lock();
    commands::session_start(
        vault.as_ref(),
        &instruction,
        &config::home(),
        io::stdin(),
        &mut out,
    )
    .ok();
}
