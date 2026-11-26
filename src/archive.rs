use std::path::{Path, PathBuf};

use chrono::NaiveDate;

use crate::markdown;
use crate::todo::{self, Todo};

const SUFFIX: &str = " archive";

pub fn archive_of(note: &Path) -> PathBuf {
    note.with_file_name(format!("{}{SUFFIX}.md", stem_of(note)))
}

pub fn is_archive(note: &Path) -> bool {
    stem_of(note).ends_with(SUFFIX)
}

pub fn source_of(note: &Path) -> PathBuf {
    match stem_of(note).strip_suffix(SUFFIX) {
        Some(source) => note.with_file_name(format!("{source}.md")),
        None => note.to_path_buf(),
    }
}

pub fn title_for(note: &Path) -> String {
    format!("# {}{SUFFIX}", stem_of(note))
}

fn stem_of(note: &Path) -> String {
    note.file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default()
}

pub struct Filed {
    pub headings: Vec<String>,
    pub block: Vec<String>,
}

#[derive(Default)]
pub struct Sorted {
    pub kept: Vec<String>,
    pub filed: Vec<Filed>,
    pub dated: usize,
}

pub fn sort_out(lines: &[String], today: NaiveDate, done_before: NaiveDate) -> Sorted {
    let mut sorted = Sorted::default();
    let mut trail = markdown::Trail::default();
    let mut index = 0;

    while index < lines.len() {
        let line = &lines[index];
        let done = Some(line)
            .filter(|line| trail.is_content(line) && !line.starts_with([' ', '\t']))
            .and_then(|line| Todo::parse(line))
            .filter(|todo| todo.done);

        match done {
            Some(todo) if todo.completed.is_none() => {
                sorted.kept.push(todo::with_completion(line, today));
                sorted.dated += 1;
                index += 1;
            }
            Some(todo)
                if todo
                    .completed
                    .is_some_and(|completed| completed < done_before) =>
            {
                let end = markdown::block_end(lines, index);
                sorted.filed.push(Filed {
                    headings: trail.headings(),
                    block: lines[index..end].to_vec(),
                });
                index = end;
            }
            _ => {
                sorted.kept.push(line.clone());
                index += 1;
            }
        }
    }
    sorted
}

pub fn file_into(mut archive: Vec<String>, filed: Vec<Filed>) -> Vec<String> {
    for Filed { headings, block } in filed {
        markdown::insert_under_headings(&mut archive, &headings, block);
    }
    archive
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap()
    }

    fn lines(text: &str) -> Vec<String> {
        text.lines().map(String::from).collect()
    }

    fn sorted(text: &str) -> Sorted {
        sort_out(&lines(text), day("2026-11-26"), day("2026-10-27"))
    }

    const GARDEN: &str = "# Garden\n## Todo\n- [ ] Rake the leaves\n- [x] Buy seeds ✅ 2026-10-01\n\t- the early ones\n- [x] Water the plants ✅ 2026-11-20\n## Agent\n### Branch 2\n- [x] Add tests ✅ 2026-09-30\n";

    #[test]
    fn todos_checked_off_before_the_cutoff_move_out_with_their_details() {
        let sorted = sorted(GARDEN);

        assert_eq!(
            sorted.kept.join("\n"),
            "# Garden\n## Todo\n- [ ] Rake the leaves\n- [x] Water the plants ✅ 2026-11-20\n## Agent\n### Branch 2"
        );
        assert_eq!(sorted.filed.len(), 2);
        assert_eq!(sorted.filed[0].headings, vec!["## Todo"]);
        assert_eq!(
            sorted.filed[0].block,
            vec!["- [x] Buy seeds ✅ 2026-10-01", "\t- the early ones"]
        );
        assert_eq!(sorted.filed[1].headings, vec!["## Agent", "### Branch 2"]);
        assert_eq!(sorted.dated, 0);
    }

    #[test]
    fn a_todo_checked_off_on_the_cutoff_stays() {
        let sorted = sorted("- [x] Buy seeds ✅ 2026-10-27\n");

        assert!(sorted.filed.is_empty());
        assert_eq!(sorted.kept, vec!["- [x] Buy seeds ✅ 2026-10-27"]);
    }

    #[test]
    fn a_checked_off_todo_without_a_date_gets_today_and_stays() {
        let sorted = sorted("- [x] Buy seeds\n");

        assert_eq!(sorted.kept, vec!["- [x] Buy seeds ✅ 2026-11-26"]);
        assert_eq!(sorted.dated, 1);
        assert!(sorted.filed.is_empty());
    }

    #[test]
    fn checked_off_subtasks_stay_with_their_open_todo() {
        let note = "- [ ] Plant the garden\n\t- [x] Buy seeds ✅ 2026-10-01\n\t- [x] Dig\n";

        let sorted = sorted(note);

        assert_eq!(sorted.kept, lines(note));
        assert_eq!(sorted.dated, 0);
    }

    #[test]
    fn todos_inside_code_are_left_alone() {
        let note = "```\n- [x] Buy seeds ✅ 2026-10-01\n- [x] Dig\n```\n";

        let sorted = sorted(note);

        assert_eq!(sorted.kept, lines(note));
        assert!(sorted.filed.is_empty());
    }

    #[test]
    fn filed_todos_go_under_their_headings_in_the_archive() {
        let archive = file_into(
            vec![title_for(Path::new("Garden.md"))],
            sorted(GARDEN).filed,
        );

        assert_eq!(
            archive.join("\n"),
            "# Garden archive\n\n## Todo\n- [x] Buy seeds ✅ 2026-10-01\n\t- the early ones\n\n## Agent\n\n### Branch 2\n- [x] Add tests ✅ 2026-09-30"
        );
    }

    #[test]
    fn the_archive_sits_next_to_its_note() {
        assert_eq!(
            archive_of(Path::new("Work/shop/shop.md")),
            PathBuf::from("Work/shop/shop archive.md")
        );
        assert!(is_archive(Path::new("Work/shop/shop archive.md")));
        assert!(!is_archive(Path::new("Work/shop/shop.md")));
    }

    #[test]
    fn an_archive_belongs_to_the_note_beside_it() {
        assert_eq!(
            source_of(Path::new("Work/shop/shop archive.md")),
            PathBuf::from("Work/shop/shop.md")
        );
        assert_eq!(
            source_of(Path::new("Work/shop/shop.md")),
            PathBuf::from("Work/shop/shop.md")
        );
    }
}
