const CODE_FENCE: &str = "```";
const DEEPEST_HEADING: usize = 6;

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

pub fn take_block(lines: &mut Vec<String>, index: usize) -> Vec<String> {
    let indentation = indentation_of(&lines[index]);
    let end = (index + 1..lines.len())
        .find(|&position| {
            let line = &lines[position];
            line.trim().is_empty() || indentation_of(line) <= indentation
        })
        .unwrap_or(lines.len());

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
