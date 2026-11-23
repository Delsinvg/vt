use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

pub fn matching<'a>(candidates: &[&'a str], query: &str) -> Vec<&'a str> {
    let wanted = query.to_lowercase();
    let containing: Vec<&str> = candidates
        .iter()
        .copied()
        .filter(|candidate| candidate.to_lowercase().contains(&wanted))
        .collect();

    if !containing.is_empty() {
        return containing;
    }

    let matcher = SkimMatcherV2::default();
    candidates
        .iter()
        .copied()
        .filter(|candidate| matcher.fuzzy_match(candidate, query).is_some())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOTES: [&str; 3] = [
        "Inbox.md",
        "Projects/Garden/Garden.md",
        "Projects/Kitchen/Kitchen.md",
    ];

    #[test]
    fn matches_part_of_the_path_ignoring_case() {
        assert_eq!(
            matching(&NOTES, "garden"),
            vec!["Projects/Garden/Garden.md"]
        );
    }

    #[test]
    fn falls_back_to_fuzzy_matching_for_a_typo() {
        assert_eq!(
            matching(&NOTES, "kitchn"),
            vec!["Projects/Kitchen/Kitchen.md"]
        );
    }

    #[test]
    fn prefers_exact_parts_over_fuzzy_matches() {
        assert_eq!(matching(&NOTES, "inbox"), vec!["Inbox.md"]);
    }

    #[test]
    fn returns_nothing_when_nothing_resembles_the_query() {
        assert!(matching(&NOTES, "zzz").is_empty());
    }
}
