use std::sync::LazyLock;

use regex::Regex;

const FALLBACK_LABEL: &str = "link";

struct LinkPattern {
    pattern: LazyLock<Regex>,
    label: fn(&regex::Captures) -> String,
}

static PATTERNS: [LinkPattern; 5] = [
    LinkPattern {
        pattern: LazyLock::new(|| Regex::new(r"/([^/]+)/-/merge_requests/(\d+)").unwrap()),
        label: |found| format!("{}!{}", &found[1], &found[2]),
    },
    LinkPattern {
        pattern: LazyLock::new(|| Regex::new(r"/([^/]+)/-/issues/(\d+)").unwrap()),
        label: |found| format!("{}#{}", &found[1], &found[2]),
    },
    LinkPattern {
        pattern: LazyLock::new(|| {
            Regex::new(r"github\.com/[^/]+/([^/]+)/(?:pull|issues)/(\d+)").unwrap()
        }),
        label: |found| format!("{}#{}", &found[1], &found[2]),
    },
    LinkPattern {
        pattern: LazyLock::new(|| {
            Regex::new(r"(?:browse/|selectedIssue=)([A-Z][A-Z0-9]+-\d+)").unwrap()
        }),
        label: |found| found[1].to_string(),
    },
    LinkPattern {
        pattern: LazyLock::new(|| {
            Regex::new(r"linear\.app/[^/]+/issue/([A-Z][A-Z0-9]+-\d+)").unwrap()
        }),
        label: |found| found[1].to_string(),
    },
];

pub fn is_url(word: &str) -> bool {
    word.starts_with("https://") || word.starts_with("http://")
}

pub fn label_for(url: &str) -> String {
    PATTERNS
        .iter()
        .find_map(|link| link.pattern.captures(url).map(|found| (link.label)(&found)))
        .unwrap_or_else(|| FALLBACK_LABEL.to_string())
}

pub fn markdown_link(url: &str) -> String {
    format!("[{}]({url})", label_for(url))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_gitlab_merge_request_is_named_after_repo_and_number() {
        assert_eq!(
            label_for("https://gitlab.com/acme/shop/-/merge_requests/12"),
            "shop!12"
        );
    }

    #[test]
    fn a_gitlab_issue_uses_a_hash() {
        assert_eq!(
            label_for("https://gitlab.com/acme/shop/-/issues/3"),
            "shop#3"
        );
    }

    #[test]
    fn a_github_pull_request_uses_a_hash() {
        assert_eq!(
            label_for("https://github.com/acme/garden/pull/125"),
            "garden#125"
        );
    }

    #[test]
    fn a_jira_ticket_is_named_after_its_key() {
        assert_eq!(
            label_for("https://acme.atlassian.net/browse/SHOP-42"),
            "SHOP-42"
        );
        assert_eq!(
            label_for("https://acme.atlassian.net/jira/boards/1?selectedIssue=SHOP-43"),
            "SHOP-43"
        );
    }

    #[test]
    fn a_linear_issue_is_named_after_its_key() {
        assert_eq!(
            label_for("https://linear.app/acme/issue/GAR-7/water-the-plants"),
            "GAR-7"
        );
    }

    #[test]
    fn any_other_url_is_just_a_link() {
        assert_eq!(label_for("https://example.com/docs"), "link");
    }

    #[test]
    fn builds_a_markdown_link() {
        assert_eq!(
            markdown_link("https://gitlab.com/acme/shop/-/issues/3"),
            "[shop#3](https://gitlab.com/acme/shop/-/issues/3)"
        );
    }
}
