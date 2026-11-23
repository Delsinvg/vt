use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "vt",
    version,
    about = "Todos in your Markdown notes, from the terminal"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    #[command(about = "Show open todos, grouped per note")]
    Todo {
        #[arg(help = "Only notes whose path matches this (typos allowed)")]
        project: Option<String>,
    },

    #[command(
        about = "Add a todo to your inbox, or show the inbox without text",
        after_help = "Examples:\n  vt inbox Call the plumber\n  vt inbox Call the plumber @tomorrow\n  vt inbox Review https://gitlab.com/acme/shop/-/merge_requests/12\n\nDates: today, tomorrow, monday..sunday, thisweek, nextweek, thismonth, nextmonth, 25/11, 2026-12-01 (Dutch words work too)"
    )]
    Inbox {
        #[arg(
            trailing_var_arg = true,
            allow_hyphen_values = true,
            help = "The todo, with @date and links"
        )]
        words: Vec<String>,
    },
}
