use std::error::Error;
use std::path::{Path, PathBuf};

use super::plain;
use crate::picker::Prompt;
use crate::select;
use crate::vault::VaultTodo;

pub(super) fn choose<'a>(
    open: &[&'a VaultTodo],
    query: &str,
    prompt: &mut impl Prompt,
) -> Result<Vec<&'a VaultTodo>, Box<dyn Error>> {
    if open.is_empty() {
        return Err("there are no open todos".into());
    }

    let texts: Vec<String> = open.iter().map(|found| plain(&found.todo.text)).collect();
    let text_refs: Vec<&str> = texts.iter().map(String::as_str).collect();
    let candidates: Vec<usize> = if query.is_empty() {
        (0..open.len()).collect()
    } else {
        select::matching_positions(&text_refs, query)
    };

    if candidates.is_empty() {
        return Err(format!("no open todo matches '{query}'").into());
    }
    if candidates.len() == 1 && !query.is_empty() {
        return Ok(vec![open[candidates[0]]]);
    }
    if !prompt.interactive() {
        return Err(if query.is_empty() {
            "give a search term to pick a todo, e.g. 'milk'".into()
        } else {
            format!(
                "{} todos match '{query}'; be more specific",
                candidates.len()
            )
            .into()
        });
    }

    let options = candidates
        .iter()
        .map(|&position| format!("{}  ({})", texts[position], open[position].note.display()))
        .collect();
    let picked = prompt.pick_many("Which todo?", options)?;

    Ok(picked
        .into_iter()
        .map(|option| open[candidates[option]])
        .collect())
}

pub(super) fn choose_project<'a>(
    projects: &'a [PathBuf],
    query: Option<&str>,
    prompt: &mut impl Prompt,
) -> Result<Option<&'a PathBuf>, Box<dyn Error>> {
    if projects.is_empty() {
        return Err("no note has a section to move todos into".into());
    }

    let names: Vec<String> = projects
        .iter()
        .map(|note| note.display().to_string())
        .collect();
    let name_refs: Vec<&str> = names.iter().map(String::as_str).collect();

    let candidates: Vec<usize> = match query {
        Some(query) => {
            if let Some(exact) = projects.iter().position(|note| has_title(note, query)) {
                return Ok(Some(&projects[exact]));
            }
            select::matching_positions(&name_refs, query)
        }
        None => (0..projects.len()).collect(),
    };

    match (candidates.len(), query) {
        (0, Some(query)) => Err(format!("no project matches '{query}'").into()),
        (1, Some(_)) => Ok(Some(&projects[candidates[0]])),
        (_, None) if !prompt.interactive() => Err("name a project, e.g. '+garden'".into()),
        (count, Some(query)) if !prompt.interactive() => {
            Err(format!("{count} projects match '{query}'; be more specific").into())
        }
        _ => {
            let options = candidates
                .iter()
                .map(|&position| names[position].clone())
                .collect();
            let picked = prompt.pick_one("Which project?", options)?;
            Ok(picked.map(|option| &projects[candidates[option]]))
        }
    }
}

fn has_title(note: &Path, query: &str) -> bool {
    note.file_stem()
        .is_some_and(|stem| stem.to_string_lossy().eq_ignore_ascii_case(query))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::picker::ScriptedPrompt;
    use crate::todo::Todo;

    fn found(note: &str, text: &str) -> VaultTodo {
        VaultTodo {
            note: PathBuf::from(note),
            line: 1,
            headings: Vec::new(),
            todo: Todo::parse(&format!("- [ ] {text}")).unwrap(),
        }
    }

    fn sample() -> Vec<VaultTodo> {
        vec![
            found("Inbox.md", "Buy milk"),
            found("Inbox.md", "Buy bread"),
            found("Garden.md", "Water the plants"),
        ]
    }

    fn texts(chosen: &[&VaultTodo]) -> Vec<String> {
        chosen.iter().map(|found| found.todo.text.clone()).collect()
    }

    #[test]
    fn a_unique_match_is_chosen_without_asking() {
        let todos = sample();
        let open: Vec<&VaultTodo> = todos.iter().collect();

        let chosen = choose(&open, "milk", &mut ScriptedPrompt::non_interactive()).unwrap();

        assert_eq!(texts(&chosen), vec!["Buy milk"]);
    }

    #[test]
    fn several_matches_are_offered_in_the_picker() {
        let todos = sample();
        let open: Vec<&VaultTodo> = todos.iter().collect();
        let mut prompt = ScriptedPrompt::picking(vec![1], None, true);

        let chosen = choose(&open, "buy", &mut prompt).unwrap();

        assert_eq!(
            prompt.shown_options,
            vec!["Buy milk  (Inbox.md)", "Buy bread  (Inbox.md)"]
        );
        assert_eq!(texts(&chosen), vec!["Buy bread"]);
    }

    #[test]
    fn several_matches_without_a_terminal_is_an_error() {
        let todos = sample();
        let open: Vec<&VaultTodo> = todos.iter().collect();

        let error = choose(&open, "buy", &mut ScriptedPrompt::non_interactive()).unwrap_err();

        assert_eq!(error.to_string(), "2 todos match 'buy'; be more specific");
    }

    #[test]
    fn no_match_is_an_error() {
        let todos = sample();
        let open: Vec<&VaultTodo> = todos.iter().collect();

        let error = choose(&open, "zzz", &mut ScriptedPrompt::non_interactive()).unwrap_err();

        assert_eq!(error.to_string(), "no open todo matches 'zzz'");
    }

    #[test]
    fn cancelling_the_picker_chooses_nothing() {
        let todos = sample();
        let open: Vec<&VaultTodo> = todos.iter().collect();

        let chosen = choose(&open, "", &mut ScriptedPrompt::picking(vec![], None, true)).unwrap();

        assert!(chosen.is_empty());
    }

    fn projects() -> Vec<PathBuf> {
        [
            "Projects/Garden/Garden.md",
            "Projects/Garden/Shed.md",
            "Projects/Kitchen/Kitchen.md",
        ]
        .iter()
        .map(PathBuf::from)
        .collect()
    }

    fn project(query: Option<&str>, prompt: &mut ScriptedPrompt) -> Result<Option<String>, String> {
        let projects = projects();
        choose_project(&projects, query, prompt)
            .map(|chosen| chosen.map(|note| note.display().to_string()))
            .map_err(|error| error.to_string())
    }

    #[test]
    fn a_project_named_exactly_wins_even_when_others_match() {
        assert_eq!(
            project(Some("garden"), &mut ScriptedPrompt::non_interactive()),
            Ok(Some("Projects/Garden/Garden.md".to_string()))
        );
    }

    #[test]
    fn a_typo_still_finds_a_single_project() {
        assert_eq!(
            project(Some("kitchn"), &mut ScriptedPrompt::non_interactive()),
            Ok(Some("Projects/Kitchen/Kitchen.md".to_string()))
        );
    }

    #[test]
    fn several_projects_without_a_terminal_is_an_error() {
        assert_eq!(
            project(Some("projects"), &mut ScriptedPrompt::non_interactive()),
            Err("3 projects match 'projects'; be more specific".to_string())
        );
    }

    #[test]
    fn several_projects_are_offered_in_a_terminal() {
        let mut prompt = ScriptedPrompt::picking(vec![], None, true).choosing(1);

        assert_eq!(
            project(Some("projects"), &mut prompt),
            Ok(Some("Projects/Garden/Shed.md".to_string()))
        );
    }

    #[test]
    fn an_unknown_project_is_an_error() {
        assert_eq!(
            project(Some("attic"), &mut ScriptedPrompt::non_interactive()),
            Err("no project matches 'attic'".to_string())
        );
    }

    #[test]
    fn without_a_name_you_pick_from_all_projects() {
        let mut prompt = ScriptedPrompt::picking(vec![], None, true).choosing(2);

        assert_eq!(
            project(None, &mut prompt),
            Ok(Some("Projects/Kitchen/Kitchen.md".to_string()))
        );
        assert_eq!(prompt.shown_options.len(), 3);
    }
}
