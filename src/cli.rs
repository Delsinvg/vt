use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "vt",
    version,
    arg_required_else_help = true,
    about = "Todos in your Markdown notes, from the terminal",
    after_help = "Configuration (~/.config/vt/config.toml, every setting optional):\n  vault = \"~/Documents/notes\"   where your notes are; the VAULT variable wins over this\n  inbox = \"Inbox.md\"           the note new todos go to\n  section = \"Todo\"             the '## ' section that projects collect todos in\n  ignore = [\"Templates\"]       folders, relative to the vault, that vt skips\n  week_ends_on = \"friday\"      the day @thisweek and @nextweek point to\n  hook_instruction = \"...\"     what Claude Code does with the project note (vt hook)"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    #[command(
        about = "Show open todos, grouped per note",
        after_help = "Inside a repository that a project note lists under 'repos:', only that project's todos are shown.\n\nExamples:\n  vt todo            the current project, or everything outside a project\n  vt todo garden     notes whose path matches 'garden'\n  vt todo --all      everything"
    )]
    Todo {
        #[arg(help = "Only notes whose path matches this (typos allowed)")]
        project: Option<String>,

        #[arg(
            short,
            long,
            conflicts_with = "project",
            help = "Show the todos of every note"
        )]
        all: bool,
    },

    #[command(
        about = "Show every open todo with a due date, earliest first",
        after_help = "Examples:\n  vt agenda\n  vt agenda garden"
    )]
    Agenda {
        #[arg(help = "Only notes whose path matches this (typos allowed)")]
        project: Option<String>,
    },

    #[command(
        about = "Show what you checked off in a period",
        after_help = "Periods: thisweek (default), lastweek, today, yesterday, thismonth, lastmonth, thisyear, lastyear (Dutch words work too)\n\nExamples:\n  vt recap\n  vt recap lastweek\n  vt recap thismonth --project garden"
    )]
    Recap {
        #[arg(default_value = "thisweek", help = "Which period to look back on")]
        period: String,

        #[arg(
            short,
            long,
            help = "Only notes whose path matches this (typos allowed)"
        )]
        project: Option<String>,
    },

    #[command(
        about = "Add a todo to your inbox, or show the inbox without text",
        after_help = "Examples:\n  vt inbox Call the plumber\n  vt inbox Call the plumber @tomorrow\n  vt inbox Rake the leaves +garden     (straight into a project)\n  vt inbox Review https://gitlab.com/acme/shop/-/merge_requests/12\n\nDates: today, tomorrow, monday..sunday, thisweek, nextweek, thismonth, nextmonth, 25/11, 2026-12-01 (Dutch words work too)"
    )]
    Inbox {
        #[arg(
            trailing_var_arg = true,
            allow_hyphen_values = true,
            help = "The todo, with @date, +project and links"
        )]
        words: Vec<String>,
    },

    #[command(
        about = "Check off a todo; without a search term you pick from a list",
        after_help = "Examples:\n  vt tick milk\n  vt tick"
    )]
    Tick {
        #[arg(
            trailing_var_arg = true,
            allow_hyphen_values = true,
            help = "Part of the todo text (typos allowed)"
        )]
        query: Vec<String>,
    },

    #[command(
        about = "Set, change or clear the due date of a todo",
        after_help = "Examples:\n  vt due milk @friday\n  vt due plumber @none\n  vt due          (pick a todo, then type a date)"
    )]
    Due {
        #[arg(
            trailing_var_arg = true,
            allow_hyphen_values = true,
            help = "Part of the todo text, optionally followed by @date or @none"
        )]
        words: Vec<String>,
    },

    #[command(
        name = "move",
        about = "Move a todo, with its details, into a project",
        after_help = "Examples:\n  vt move hose +garden\n  vt move hose        (pick the project from a list)\n  vt move             (pick the todo, then the project)"
    )]
    Move {
        #[arg(
            trailing_var_arg = true,
            allow_hyphen_values = true,
            help = "Part of the todo text, optionally followed by +project"
        )]
        words: Vec<String>,
    },

    #[command(
        about = "Print the project note for a Claude Code SessionStart hook",
        after_help = "Reads the hook event from stdin and prints the matching project note as JSON. Prints nothing outside a repository or without VAULT, so it never blocks a session.\n\nIn ~/.claude/settings.json:\n  \"SessionStart\": [{ \"hooks\": [{ \"type\": \"command\", \"command\": \"vt hook\" }] }]"
    )]
    Hook,
}
