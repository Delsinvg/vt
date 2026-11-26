use std::path::Path;

pub const FILE_PLACEHOLDER: &str = "{file}";
pub const LINE_PLACEHOLDER: &str = "{line}";

pub const SYSTEM_OPENER: &str = if cfg!(target_os = "macos") {
    "open"
} else {
    "xdg-open"
};

#[derive(Debug, Clone, PartialEq)]
pub enum Opener {
    Obsidian,
    Command(Vec<String>),
}

pub struct Spot<'a> {
    pub note: &'a Path,
    pub line: usize,
    pub heading: Option<&'a str>,
}

impl Opener {
    pub fn from_template(template: &str) -> Option<Self> {
        let words: Vec<String> = template.split_whitespace().map(String::from).collect();
        let names_the_file = words.iter().any(|word| word.contains(FILE_PLACEHOLDER));
        names_the_file.then_some(Self::Command(words))
    }

    pub fn command_line(&self, spot: &Spot) -> Vec<String> {
        match self {
            Self::Obsidian => vec![SYSTEM_OPENER.to_string(), obsidian_uri(spot)],
            Self::Command(words) => {
                let file = spot.note.to_string_lossy();
                let line = spot.line.to_string();
                words
                    .iter()
                    .map(|word| {
                        word.replace(FILE_PLACEHOLDER, &file)
                            .replace(LINE_PLACEHOLDER, &line)
                    })
                    .collect()
            }
        }
    }
}

/// @see https://obsidian.md/help/Extending+Obsidian/Obsidian+URI
fn obsidian_uri(spot: &Spot) -> String {
    let mut target = spot.note.to_string_lossy().into_owned();
    if let Some(heading) = spot.heading {
        target.push('#');
        target.push_str(heading);
    }
    format!("obsidian://open?path={}", percent_encoded(&target))
}

pub fn percent_encoded(text: &str) -> String {
    text.bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                char::from(byte).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spot(heading: Option<&'static str>) -> Spot<'static> {
        Spot {
            note: Path::new("/notes/Tuin plan.md"),
            line: 12,
            heading,
        }
    }

    #[test]
    fn obsidian_opens_the_note_at_its_heading() {
        assert_eq!(
            Opener::Obsidian.command_line(&spot(Some("Lente: zaaien"))),
            vec![
                SYSTEM_OPENER,
                "obsidian://open?path=%2Fnotes%2FTuin%20plan.md%23Lente%3A%20zaaien"
            ]
        );
    }

    #[test]
    fn obsidian_opens_a_note_without_heading_at_the_top() {
        assert_eq!(
            Opener::Obsidian.command_line(&spot(None)),
            vec![
                SYSTEM_OPENER,
                "obsidian://open?path=%2Fnotes%2FTuin%20plan.md"
            ]
        );
    }

    #[test]
    fn a_template_fills_in_the_file_and_line_and_keeps_spaces_in_one_argument() {
        let opener = Opener::from_template("nvim +{line} {file}").unwrap();

        assert_eq!(
            opener.command_line(&spot(Some("Lente"))),
            vec!["nvim", "+12", "/notes/Tuin plan.md"]
        );
    }

    #[test]
    fn a_placeholder_can_sit_inside_a_word() {
        let opener = Opener::from_template("zed {file}:{line}").unwrap();

        assert_eq!(
            opener.command_line(&spot(None)),
            vec!["zed", "/notes/Tuin plan.md:12"]
        );
    }

    #[test]
    fn a_template_without_the_file_is_refused() {
        assert_eq!(Opener::from_template("nvim +{line}"), None);
        assert_eq!(Opener::from_template("  "), None);
    }

    #[test]
    fn accents_are_encoded_as_utf8() {
        assert_eq!(percent_encoded("é"), "%C3%A9");
    }
}
