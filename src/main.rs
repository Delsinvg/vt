mod todo;
mod vault;

use std::env;
use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

use vault::Vault;

const MISSING_VAULT: &str =
    "VAULT is not set. Point it to your notes, e.g. export VAULT=\"$HOME/Documents/notes\"";

fn vault_root() -> io::Result<PathBuf> {
    env::var_os("VAULT")
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::other(MISSING_VAULT))
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("vt: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> io::Result<()> {
    let vault = Vault::new(vault_root()?);

    for found in vault.todos()? {
        if !found.todo.done {
            println!(
                "{}:{}  {}",
                found.note.display(),
                found.line,
                found.todo.text
            );
        }
    }

    Ok(())
}
