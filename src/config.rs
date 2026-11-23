use std::env;
use std::io;
use std::path::PathBuf;

pub const INBOX_NOTE: &str = "Inbox.md";

const VAULT_VARIABLE: &str = "VAULT";
const MISSING_VAULT: &str =
    "VAULT is not set. Point it to your notes, e.g. export VAULT=\"$HOME/Documents/notes\"";

pub fn vault_root() -> io::Result<PathBuf> {
    env::var_os(VAULT_VARIABLE)
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::other(MISSING_VAULT))
}
