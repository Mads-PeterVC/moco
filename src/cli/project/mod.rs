mod relocate;
pub mod list;
mod category;
mod info;
mod set_category;
mod set_sync;

use std::path::Path;

use clap::{Args, Subcommand};

use crate::config::AppConfig;
use crate::db::Store;
use crate::theme::Theme;

// Re-use the existing command modules from the parent cli module.
use super::{delete, export, init, label, open};

#[derive(Args)]
pub struct ProjectArgs {
    #[command(subcommand)]
    pub command: ProjectCommand,
}

#[derive(Subcommand)]
pub enum ProjectCommand {
    /// Initialize a project for the current directory.
    Init(init::InitArgs),
    /// Delete a registered project and all its tasks.
    Delete(delete::DeleteArgs),
    /// Open a registered project in your configured editor.
    Open(open::OpenArgs),
    /// List all registered projects with their labels and task counts.
    List(list::ListArgs),
    /// Show detailed information about a project, including git status.
    Info(info::InfoArgs),
    /// Manage labels on the current project.
    Label(label::LabelArgs),
    /// Export tasks to a Markdown file.
    Export(export::ExportArgs),
    /// Change the registered directory for a project.
    #[command(name = "move")]
    Move(relocate::MoveArgs),
    /// Manage project categories.
    Category(category::CategoryArgs),
    /// Assign a project to a category.
    #[command(name = "set-category")]
    SetCategory(set_category::SetCategoryArgs),
    /// Enable or disable `moco sync status` network checks for a project.
    #[command(name = "set-sync")]
    SetSync(set_sync::SetSyncArgs),
}

pub fn run(
    args: &ProjectArgs,
    store: &mut impl Store,
    cwd: &Path,
    config: &AppConfig,
    theme: &Theme,
) -> anyhow::Result<()> {
    match &args.command {
        ProjectCommand::Init(a) => init::run(a, store, cwd, theme),
        ProjectCommand::Delete(a) => delete::run(a, store, theme),
        ProjectCommand::Open(a) => open::run(a, store, config, theme),
        ProjectCommand::List(a) => list::run(a, store, theme, config),
        ProjectCommand::Info(a) => info::run(a, store, cwd, theme, config),
        ProjectCommand::Label(a) => label::run(a, store, cwd, theme),
        ProjectCommand::Export(a) => export::run(a, store, cwd, config, theme),
        ProjectCommand::Move(a) => relocate::run(a, store, theme),
        ProjectCommand::Category(a) => category::run(a, store, theme),
        ProjectCommand::SetCategory(a) => set_category::run(a, store, theme),
        ProjectCommand::SetSync(a) => set_sync::run(a, store, cwd, theme),
    }
}
