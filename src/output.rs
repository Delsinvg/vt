use std::sync::LazyLock;

use regex::Regex;

static WIKILINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[\[(?:[^\]|]*\|)?([^\]]*)\]\]").unwrap());
static MARKDOWN_LINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[([^\[\]]+)\]\((https?://[^)\s]+)\)").unwrap());

const RESET: &str = "\x1b[0m";
const MAGENTA: &str = "\x1b[35m";
const BOLD: &str = "\x1b[1m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const DIM: &str = "\x1b[2m";

#[derive(Clone, Copy)]
pub struct Style {
    pub terminal: bool,
}

impl Style {
    pub fn overdue(&self, text: &str) -> String {
        self.paint(RED, text)
    }

    pub fn due_today(&self, text: &str) -> String {
        self.paint(YELLOW, text)
    }

    pub fn dim(&self, text: &str) -> String {
        self.paint(DIM, text)
    }

    pub fn heading(&self, text: &str) -> String {
        self.paint(MAGENTA, text)
    }

    pub fn subheading(&self, text: &str) -> String {
        self.paint(BOLD, text)
    }

    pub fn links(&self, text: &str) -> String {
        let text = WIKILINK.replace_all(text, "$1");
        let replacement = if self.terminal {
            "\x1b]8;;$2\x1b\\$1\x1b]8;;\x1b\\"
        } else {
            "$1"
        };
        MARKDOWN_LINK.replace_all(&text, replacement).into_owned()
    }

    fn paint(&self, color: &str, text: &str) -> String {
        if self.terminal {
            format!("{color}{text}{RESET}")
        } else {
            text.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAIN: Style = Style { terminal: false };
    const TERMINAL: Style = Style { terminal: true };

    #[test]
    fn a_wikilink_shows_its_label() {
        assert_eq!(
            PLAIN.links("see [[Projects/Garden|the garden]]"),
            "see the garden"
        );
    }

    #[test]
    fn a_wikilink_without_label_shows_its_target() {
        assert_eq!(PLAIN.links("see [[Garden]]"), "see Garden");
    }

    #[test]
    fn a_markdown_link_shows_only_its_label_outside_a_terminal() {
        assert_eq!(
            PLAIN.links("review [shop!12](https://example.com/mr/12)"),
            "review shop!12"
        );
    }

    #[test]
    fn a_markdown_link_becomes_clickable_in_a_terminal() {
        assert_eq!(
            TERMINAL.links("[shop!12](https://example.com/mr/12)"),
            "\x1b]8;;https://example.com/mr/12\x1b\\shop!12\x1b]8;;\x1b\\"
        );
    }

    #[test]
    fn colors_only_appear_in_a_terminal() {
        assert_eq!(PLAIN.heading("Inbox.md"), "Inbox.md");
        assert_eq!(TERMINAL.heading("Inbox.md"), "\x1b[35mInbox.md\x1b[0m");
    }
}
