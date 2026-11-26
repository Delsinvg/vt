const CODE_FENCE: &str = "```";
const DEEPEST_HEADING: usize = 6;
const TITLE_LEVEL: usize = 1;

#[derive(Default)]
pub struct Trail {
    headings: Vec<(usize, String)>,
    inside_code: bool,
}

impl Trail {
    pub fn is_content(&mut self, line: &str) -> bool {
        if is_code_fence(line) {
            self.inside_code = !self.inside_code;
            return false;
        }
        if self.inside_code {
            return false;
        }
        let Some(level) = heading_level(line) else {
            return true;
        };
        self.headings.retain(|(open_level, _)| *open_level < level);
        if level > TITLE_LEVEL {
            self.headings.push((level, line.trim_end().to_string()));
        }
        false
    }

    pub fn headings(&self) -> Vec<String> {
        self.headings
            .iter()
            .map(|(_, heading)| heading.clone())
            .collect()
    }
}

pub fn heading_level(line: &str) -> Option<usize> {
    let level = line
        .chars()
        .take_while(|&character| character == '#')
        .count();
    let is_heading = (1..=DEEPEST_HEADING).contains(&level) && line[level..].starts_with(' ');
    is_heading.then_some(level)
}

pub fn heading_text(heading: &str) -> &str {
    heading.trim_start_matches('#').trim()
}

pub fn is_code_fence(line: &str) -> bool {
    line.trim_start().starts_with(CODE_FENCE)
}

pub fn has_section(content: &str, heading: &str) -> bool {
    content.lines().any(|line| line.trim_end() == heading)
}

pub fn block_end(lines: &[String], index: usize) -> usize {
    let indentation = indentation_of(&lines[index]);
    (index + 1..lines.len())
        .find(|&position| {
            let line = &lines[position];
            line.trim().is_empty() || indentation_of(line) <= indentation
        })
        .unwrap_or(lines.len())
}

pub fn take_block(lines: &mut Vec<String>, index: usize) -> Vec<String> {
    let indentation = indentation_of(&lines[index]);
    let end = block_end(lines, index);

    lines
        .drain(index..end)
        .map(|line| line[indentation..].to_string())
        .collect()
}

pub fn insert_under_section(lines: &mut Vec<String>, heading: &str, block: Vec<String>) -> bool {
    let Some(heading_index) = lines.iter().position(|line| line.trim_end() == heading) else {
        return false;
    };

    let first_content = (heading_index + 1..lines.len())
        .find(|&position| !lines[position].trim().is_empty())
        .unwrap_or(lines.len());

    if first_content < lines.len() && is_list_line(&lines[first_content]) {
        let after_list = (first_content..lines.len())
            .find(|&position| !is_list_line(&lines[position]))
            .unwrap_or(lines.len());
        lines.splice(after_list..after_list, block);
    } else {
        let mut replacement = vec![String::new()];
        replacement.extend(block);
        if first_content < lines.len() {
            replacement.push(String::new());
        }
        lines.splice(heading_index + 1..first_content, replacement);
    }
    true
}

pub fn insert_under_headings(lines: &mut Vec<String>, headings: &[String], block: Vec<String>) {
    let mut start = 0;
    let mut end = lines.len();
    let mut level = TITLE_LEVEL;

    for heading in headings {
        let heading_index = match (start..end).find(|&index| lines[index].trim_end() == heading) {
            Some(index) => index,
            None => {
                let at = after_last_content(lines, start, end);
                let mut added = Vec::new();
                if at > 0 {
                    added.push(String::new());
                }
                added.push(heading.clone());
                let index = at + added.len() - 1;
                lines.splice(at..at, added);
                index
            }
        };
        level = heading_level(heading).unwrap_or(DEEPEST_HEADING);
        start = heading_index + 1;
        end = (start..lines.len())
            .find(|&index| heading_level(&lines[index]).is_some_and(|other| other <= level))
            .unwrap_or(lines.len());
    }

    let own_end = (start..end)
        .find(|&index| heading_level(&lines[index]).is_some_and(|other| other > level))
        .unwrap_or(end);
    let at = after_last_content(lines, start, own_end);
    lines.splice(at..at, block);
}

fn after_last_content(lines: &[String], start: usize, end: usize) -> usize {
    (start..end)
        .rev()
        .find(|&index| !lines[index].trim().is_empty())
        .map_or(start, |index| index + 1)
}

fn indentation_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn is_list_line(line: &str) -> bool {
    !line.trim().is_empty() && (line.starts_with("- ") || line.starts_with(['\t', ' ']))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADING: &str = "## Tasks";

    fn lines(text: &str) -> Vec<String> {
        text.lines().map(String::from).collect()
    }

    fn joined(lines: &[String]) -> String {
        lines.join("\n")
    }

    #[test]
    fn takes_a_todo_with_its_indented_details() {
        let mut note = lines("# Inbox\n- [ ] Water the plants\n\t- every morning\n- [ ] Buy milk");

        let block = take_block(&mut note, 1);

        assert_eq!(block, vec!["- [ ] Water the plants", "\t- every morning"]);
        assert_eq!(joined(&note), "# Inbox\n- [ ] Buy milk");
    }

    #[test]
    fn a_nested_todo_comes_out_at_the_top_level() {
        let mut note = lines("- [ ] Garden\n\t- [ ] Buy a hose\n\t\t- the long one\n\t- [ ] Rake");

        let block = take_block(&mut note, 1);

        assert_eq!(block, vec!["- [ ] Buy a hose", "\t- the long one"]);
        assert_eq!(joined(&note), "- [ ] Garden\n\t- [ ] Rake");
    }

    #[test]
    fn a_blank_line_ends_the_block() {
        let mut note = lines("- [ ] Water the plants\n\n\tnot a detail");

        assert_eq!(take_block(&mut note, 0), vec!["- [ ] Water the plants"]);
    }

    #[test]
    fn inserts_after_the_last_item_of_the_section() {
        let mut note = lines("# Garden\n## Tasks\n\n- [ ] Rake\n\t- leaves\n\n## Links");

        assert!(insert_under_section(
            &mut note,
            HEADING,
            vec!["- [ ] Buy seeds".into()]
        ));

        assert_eq!(
            joined(&note),
            "# Garden\n## Tasks\n\n- [ ] Rake\n\t- leaves\n- [ ] Buy seeds\n\n## Links"
        );
    }

    #[test]
    fn fills_an_empty_section_with_blank_lines_around_it() {
        let mut note = lines("# Garden\n## Tasks\n\n## Links");

        assert!(insert_under_section(
            &mut note,
            HEADING,
            vec!["- [ ] Buy seeds".into()]
        ));

        assert_eq!(
            joined(&note),
            "# Garden\n## Tasks\n\n- [ ] Buy seeds\n\n## Links"
        );
    }

    #[test]
    fn fills_a_section_at_the_end_of_the_note() {
        let mut note = lines("# Garden\n## Tasks");

        assert!(insert_under_section(
            &mut note,
            HEADING,
            vec!["- [ ] Buy seeds".into()]
        ));

        assert_eq!(joined(&note), "# Garden\n## Tasks\n\n- [ ] Buy seeds");
    }

    #[test]
    fn reports_a_missing_section() {
        let mut note = lines("# Garden\n## Links");

        assert!(!insert_under_section(
            &mut note,
            HEADING,
            vec!["- [ ] Buy seeds".into()]
        ));
        assert_eq!(joined(&note), "# Garden\n## Links");
    }

    fn filed(note: &str, headings: &[&str], todo: &str) -> String {
        let mut note = lines(note);
        let headings: Vec<String> = headings.iter().map(|heading| heading.to_string()).collect();
        insert_under_headings(&mut note, &headings, vec![todo.to_string()]);
        joined(&note)
    }

    #[test]
    fn files_under_the_existing_heading_after_its_last_line() {
        assert_eq!(
            filed(
                "# Archive\n\n## Tasks\n- [x] Rake\n\n## Agent\n- [x] Test",
                &["## Tasks"],
                "- [x] Sow"
            ),
            "# Archive\n\n## Tasks\n- [x] Rake\n- [x] Sow\n\n## Agent\n- [x] Test"
        );
    }

    #[test]
    fn adds_the_missing_headings_at_the_end() {
        assert_eq!(
            filed("# Archive", &["## Agent", "### Branch 2"], "- [x] Test"),
            "# Archive\n\n## Agent\n\n### Branch 2\n- [x] Test"
        );
    }

    #[test]
    fn files_a_subsection_inside_its_section_and_not_in_the_next_one() {
        assert_eq!(
            filed(
                "# Archive\n\n## Agent\n- [x] Test\n\n## Tasks\n- [x] Rake",
                &["## Agent", "### Branch 2"],
                "- [x] Lint"
            ),
            "# Archive\n\n## Agent\n- [x] Test\n\n### Branch 2\n- [x] Lint\n\n## Tasks\n- [x] Rake"
        );
    }

    #[test]
    fn files_above_the_subsections_of_its_heading() {
        assert_eq!(
            filed(
                "# Archive\n\n## Agent\n- [x] Test\n\n### Branch 2\n- [x] Lint",
                &["## Agent"],
                "- [x] Build"
            ),
            "# Archive\n\n## Agent\n- [x] Test\n- [x] Build\n\n### Branch 2\n- [x] Lint"
        );
    }

    #[test]
    fn files_a_todo_without_headings_under_the_title() {
        assert_eq!(
            filed(
                "# Archive\n- [x] Milk\n\n## Tasks\n- [x] Rake",
                &[],
                "- [x] Bread"
            ),
            "# Archive\n- [x] Milk\n- [x] Bread\n\n## Tasks\n- [x] Rake"
        );
    }

    #[test]
    fn a_trail_follows_the_headings_and_skips_code() {
        let mut trail = Trail::default();
        let content: Vec<&str> = "# Garden\n## Tasks\n### Spring\n- [ ] Sow\n```\n## Not a heading\n```\n## Links\n- [ ] Read"
            .lines()
            .filter(|line| trail.is_content(line))
            .collect();

        assert_eq!(content, vec!["- [ ] Sow", "- [ ] Read"]);
        assert_eq!(trail.headings(), vec!["## Links"]);
    }

    #[test]
    fn recognises_headings_and_their_level() {
        assert_eq!(heading_level("## Tasks"), Some(2));
        assert_eq!(heading_level("### MOB-1 badge"), Some(3));
        assert_eq!(heading_level("#hashtag"), None);
        assert_eq!(heading_level("####### too deep"), None);
        assert_eq!(heading_text("###  MOB-1 badge "), "MOB-1 badge");
    }

    #[test]
    fn recognises_a_section_heading() {
        assert!(has_section("# Garden\n## Tasks\n", HEADING));
        assert!(!has_section("# Garden\n### Tasks\n", HEADING));
    }
}
