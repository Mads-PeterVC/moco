use std::path::Path;

use clap::{Args, Subcommand};

use crate::db::Store;
use crate::theme::Theme;
use crate::workspace;

#[derive(Args)]
pub struct LabelArgs {
    #[command(subcommand)]
    pub subcommand: LabelCommand,
}

#[derive(Subcommand)]
pub enum LabelCommand {
    /// Add a label to the current project.
    Add {
        /// Label name to add.
        label: String,
    },
    /// Remove a label from the current project.
    Remove {
        /// Label name to remove.
        label: String,
    },
    /// List all labels on the current project.
    List,
}

pub fn run(args: &LabelArgs, store: &mut dyn Store, cwd: &Path, theme: &Theme) -> anyhow::Result<()> {
    let mut project = workspace::resolve(store, cwd)?
        .ok_or_else(|| anyhow::anyhow!("Not in a registered project. Run 'moco init' first."))?;

    match &args.subcommand {
        LabelCommand::Add { label } => {
            if project.labels.contains(label) {
                println!("Label '{}' already exists on project '{}'.", label, project.name);
            } else {
                project.labels.push(label.clone());
                store.update_project(&project)?;
                println!("Added label '{}' to project '{}'.", label, project.name);
            }
        }
        LabelCommand::Remove { label } => {
            if let Some(pos) = project.labels.iter().position(|l| l == label) {
                project.labels.remove(pos);
                store.update_project(&project)?;
                println!("Removed label '{}' from project '{}'.", label, project.name);
            } else {
                println!("Label '{}' not found on project '{}'.", label, project.name);
            }
        }
        LabelCommand::List => {
            if project.labels.is_empty() {
                println!("No labels for project '{}'.", project.name);
            } else {
                println!("Labels for project '{}':", project.name);
                for label in &project.labels {
                    println!("  {}", theme.paint(label, theme.accent));
                }
            }
        }
    }

    Ok(())
}