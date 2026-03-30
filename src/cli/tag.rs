use std::path::Path;

use clap::{Args, Subcommand};

use crate::db::Store;
use crate::error::MocoError;
use crate::theme::Theme;
use crate::workspace;

#[derive(Args)]
pub struct TagArgs {
    #[command(subcommand)]
    pub subcommand: TagCommand,
}

#[derive(Subcommand)]
pub enum TagCommand {
    /// Add a tag to a task.
    Add {
        /// Display index of the open task (e.g. 1 for #1).
        task_id: u32,
        /// Tag name to add.
        tag: String,
    },
    /// Remove a tag from a task.
    Remove {
        /// Display index of the open task (e.g. 1 for #1).
        task_id: u32,
        /// Tag name to remove.
        tag: String,
    },
    /// List tags on a task, or all tagged tasks if no task_id is given.
    List {
        /// Display index of the open task (e.g. 1 for #1). If omitted, lists all tagged tasks.
        task_id: Option<u32>,
    },
}

pub fn run(args: &TagArgs, store: &mut dyn Store, cwd: &Path, theme: &Theme) -> anyhow::Result<()> {
    let project_id = workspace::resolve(store, cwd)?.map(|p| p.id);

    match &args.subcommand {
        TagCommand::Add { task_id, tag } => {
            let mut task = store
                .get_open_task(project_id, *task_id)?
                .ok_or(MocoError::TaskNotFound(*task_id))?;

            if task.tags.contains(tag) {
                println!("Tag '{}' already exists on task {}.", tag, task.display_id());
            } else {
                task.tags.push(tag.clone());
                store.update_task(&task)?;
                println!("Added tag '{}' to task {}.", tag, task.display_id());
            }
        }
        TagCommand::Remove { task_id, tag } => {
            let mut task = store
                .get_open_task(project_id, *task_id)?
                .ok_or(MocoError::TaskNotFound(*task_id))?;

            if let Some(pos) = task.tags.iter().position(|t| t == tag) {
                task.tags.remove(pos);
                store.update_task(&task)?;
                println!("Removed tag '{}' from task {}.", tag, task.display_id());
            } else {
                println!("Tag '{}' not found on task {}.", tag, task.display_id());
            }
        }
        TagCommand::List { task_id } => {
            if let Some(id) = task_id {
                let task = store
                    .get_open_task(project_id, *id)?
                    .ok_or(MocoError::TaskNotFound(*id))?;

                if task.tags.is_empty() {
                    println!("No tags on task {}.", task.display_id());
                } else {
                    println!("Tags on task {}:", task.display_id());
                    for tag in &task.tags {
                        println!("  {}", theme.paint(tag, theme.accent));
                    }
                }
            } else {
                let tasks = store.list_tasks(project_id)?;
                let tagged: Vec<_> = tasks.iter().filter(|t| !t.tags.is_empty()).collect();
                if tagged.is_empty() {
                    println!("No tags found.");
                } else {
                    for task in tagged {
                        let tag_str = task.tags.join(" ");
                        println!(
                            "  {}  {}",
                            task.display_id(),
                            theme.paint(format!("[{tag_str}]"), theme.accent)
                        );
                    }
                }
            }
        }
    }

    Ok(())
}
