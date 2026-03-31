pub mod add;
pub mod config;
pub mod delete;
pub mod edit;
pub mod export;
pub mod init;
pub mod label;
pub mod list;
pub mod note;
pub mod open;
pub mod project;
pub mod remove;
pub mod status;
pub mod sync;
pub mod tag;
pub mod task_ref;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "moco", about = "A lightweight CLI task manager", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Manage projects: init, delete, open, label, export, move.
    Project(project::ProjectArgs),
    /// Add a new task to the current project (or global list).
    Add(add::AddArgs),
    /// Edit an existing task.
    Edit(edit::EditArgs),
    /// Update the status or progress of a task.
    Status(status::StatusArgs),
    /// List tasks for the current project (or global list).
    List(list::ListArgs),
    /// Manage tags on tasks.
    Tag(tag::TagArgs),
    /// Manage notes for the current project (or global list).
    Note(note::NoteArgs),
    /// Remove a task (and its subtasks) from the current project or global list.
    Remove(remove::RemoveArgs),
    /// Manage moco configuration.
    Config(config::ConfigArgs),
    /// Sync the ~/.moco/ directory with a remote git repository.
    Sync(sync::SyncArgs),
}
