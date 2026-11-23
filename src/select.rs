use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

pub fn matching<'a>(candidates: &[&'a str], query: &str) -> Vec<&'a str> {
    matching_positions(candidates, query)
        .into_iter()
        .map(|position| candidates[position])
        .collect()
}

pub fn matching_positions(candidates: &[&str], query: &str) -> Vec<usize> {
    let wanted = query.to_lowercase();
    let positions = 0..candidates.len();

    let containing: Vec<usize> = positions
        .clone()
        .filter(|&position| candidates[position].to_lowercase().contains(&wanted))
        .collect();

    if !containing.is_empty() {
        return containing;
    }

    let matcher = SkimMatcherV2::default();
    positions
        .filter(|&position| matcher.fuzzy_match(candidates[position], query).is_some())
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
