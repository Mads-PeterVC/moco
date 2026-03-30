pub mod add;
pub mod config;
pub mod delete;
pub mod edit;
pub mod export;
pub mod init;
pub mod list;
pub mod open;
pub mod status;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "moco", about = "A lightweight CLI task manager", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Initialize a project for the current directory.
    Init(init::InitArgs),

    /// Add a new task to the current project (or global list).
    Add(add::AddArgs),

    /// Edit an existing task.
    Edit(edit::EditArgs),

    /// Update the status or progress of a task.
    Status(status::StatusArgs),

    /// List tasks for the current project (or global list).
    List(list::ListArgs),

    /// Export tasks to a Markdown file.
    Export(export::ExportArgs),

    /// Open a registered project in your configured editor.
    Open(open::OpenArgs),

    /// Delete a registered project and all its tasks.
    Delete(delete::DeleteArgs),

    /// Manage moco configuration.
    Config(config::ConfigArgs),
}
