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
}
