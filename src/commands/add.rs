use std::error::Error;
use std::io::Write;
use std::path::Path;

use chrono::NaiveDate;

use super::plain;
use crate::inbox;
use crate::vault::Vault;

pub fn add_to_inbox(
    vault: &Vault,
    inbox_note: &Path,
    words: &[String],
    today: NaiveDate,
    out: &mut impl Write,
) -> Result<(), Box<dyn Error>> {
    let todo = inbox::compose(words, today)?;
    vault.append_line(inbox_note, &todo.line())?;

    let text = plain(&todo.text);
    match todo.due {
        Some(due) => writeln!(out, "Added to inbox: {text} (due {due})")?,
        None => writeln!(out, "Added to inbox: {text}")?,
    }
    Ok(())
}
