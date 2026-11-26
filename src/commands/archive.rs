use std::error::Error;
use std::io::Write;

use chrono::{Days, NaiveDate};

use super::title_of;
use crate::vault::Vault;

pub fn archive(
    vault: &Vault,
    today: NaiveDate,
    after_days: u32,
    out: &mut impl Write,
) -> Result<(), Box<dyn Error>> {
    let done_before = today - Days::new(after_days.into());
    let archived = vault.archive_done(today, done_before)?;

    if archived.is_empty() {
        writeln!(
            out,
            "Nothing to archive: todos go to the archive {after_days} days after they are checked off"
        )?;
    }
    for note in archived {
        let title = title_of(&note.note);
        if note.filed > 0 {
            let archive = note
                .archive
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy();
            writeln!(
                out,
                "Archived {} from {title} into {archive}",
                todos(note.filed)
            )?;
        }
        if note.dated > 0 {
            writeln!(
                out,
                "Dated {} in {title} without a date; they go to the archive in {after_days} days",
                todos(note.dated)
            )?;
        }
    }
    Ok(())
}

fn todos(count: usize) -> String {
    match count {
        1 => "1 checked-off todo".to_string(),
        _ => format!("{count} checked-off todos"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn archive_in(notes: &[(&str, &str)]) -> (TempDir, String) {
        let directory = TempDir::new().unwrap();
        for (name, content) in notes {
            fs::write(directory.path().join(name), content).unwrap();
        }
        let today = NaiveDate::from_ymd_opt(2026, 11, 26).unwrap();

        let mut out = Vec::new();
        archive(&Vault::new(directory.path()), today, 30, &mut out).unwrap();
        (directory, String::from_utf8(out).unwrap())
    }

    #[test]
    fn says_what_moved_and_what_got_a_date() {
        let (directory, printed) = archive_in(&[(
            "Garden.md",
            "# Garden\n## Todo\n- [x] Sow ✅ 2026-10-01\n- [x] Dig ✅ 2026-10-26\n- [x] Rake\n- [x] Mow\n",
        )]);

        assert_eq!(
            printed,
            "Archived 2 checked-off todos from Garden into Garden archive\n\
             Dated 2 checked-off todos in Garden without a date; they go to the archive in 30 days\n"
        );
        assert!(directory.path().join("Garden archive.md").exists());
    }

    #[test]
    fn a_single_todo_is_not_counted_in_the_plural() {
        let (_, printed) = archive_in(&[("Garden.md", "- [x] Sow ✅ 2026-10-01\n")]);

        assert_eq!(
            printed,
            "Archived 1 checked-off todo from Garden into Garden archive\n"
        );
    }

    #[test]
    fn says_so_when_there_is_nothing_to_archive() {
        let (directory, printed) = archive_in(&[("Garden.md", "- [x] Sow ✅ 2026-11-20\n")]);

        assert_eq!(
            printed,
            "Nothing to archive: todos go to the archive 30 days after they are checked off\n"
        );
        assert!(!directory.path().join("Garden archive.md").exists());
    }
}
