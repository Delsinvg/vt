use std::error::Error;

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
    let picked = prompt.pick(options)?;

    Ok(picked
        .into_iter()
        .map(|option| open[candidates[option]])
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::picker::ScriptedPrompt;
    use crate::todo::Todo;
    use std::path::PathBuf;

    fn found(note: &str, text: &str) -> VaultTodo {
        VaultTodo {
            note: PathBuf::from(note),
            line: 1,
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
}
