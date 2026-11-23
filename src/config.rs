use std::env;
use std::io;
use std::path::PathBuf;

const INBOX_NOTE: &str = "Inbox.md";
const PROJECT_SECTION: &str = "Algemeen";

const VAULT_VARIABLE: &str = "VAULT";
const MISSING_VAULT: &str =
    "VAULT is not set. Point it to your notes, e.g. export VAULT=\"$HOME/Documents/notes\"";

pub struct Layout {
    pub inbox: PathBuf,
    pub section_heading: String,
}

pub fn layout() -> Layout {
    Layout {
        inbox: PathBuf::from(INBOX_NOTE),
        section_heading: format!("## {PROJECT_SECTION}"),
    }
}

pub fn home() -> PathBuf {
    env::var_os("HOME").map(PathBuf::from).unwrap_or_default()
}

pub fn vault_root() -> io::Result<PathBuf> {
    env::var_os(VAULT_VARIABLE)
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::other(MISSING_VAULT))
}
