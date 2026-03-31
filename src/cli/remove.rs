use std::io::{self, Write};
use std::path::Path;

use clap::Args;

use crate::cli::task_ref;
use crate::db::Store;
use crate::theme::Theme;
use crate::workspace;

#[derive(Args)]
pub struct RemoveArgs {
    /// Task to remove: open index (`1`), subtask (`1.1`), completed (`C1`), deferred (`D1`).
    pub task_id: String,

    /// Skip confirmation prompt.
    #[arg(short, long)]
    pub yes: bool,

    /// Operate on the global task list instead of the current project.
    #[arg(short, long)]
    pub global: bool,
}

pub fn run(args: &RemoveArgs, store: &mut dyn Store, cwd: &Path, theme: &Theme) -> anyhow::Result<()> {
    let project_id = if args.global {
        None
    } else {
        workspace::resolve(store, cwd)?.map(|p| p.id)
    };

    let task_ref = task_ref::parse(&args.task_id).map_err(|e| anyhow::anyhow!(e))?;
    let task = task_ref::resolve(store, project_id, &task_ref)?
        .ok_or_else(|| anyhow::anyhow!("Task '{}' not found.", args.task_id))?;

    let parent = if let Some(pid) = task.parent_id {
        store.get_task_by_id(pid)?
    } else {
        None
    };
    let display = task.display_id_in_context(parent.as_ref());

    // Check whether this task has subtasks (only relevant for top-level tasks).
    let all_tasks = store.list_tasks(project_id)?;
    let subtask_count = all_tasks.iter().filter(|t| t.parent_id == Some(task.id)).count();

    if !args.yes {
        let extra = if subtask_count > 0 {
            format!(" and its {} subtask(s)", subtask_count)
        } else {
            String::new()
        };
        print!(
            "Delete task {}{extra}? [y/N] ",
            theme.paint(&display, match task.status {
                crate::models::TaskStatus::Open => theme.open,
                crate::models::TaskStatus::Complete => theme.complete,
                crate::models::TaskStatus::Defer => theme.defer,
            })
        );
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            println!("Aborted.");
            return Ok(());
        }
    }

    store.delete_task(task.id)?;

    if subtask_count > 0 {
        println!(
            "Deleted task {} and {} subtask(s).",
            theme.paint(&display, theme.accent),
            subtask_count,
        );
    } else {
        println!("Deleted task {}.", theme.paint(&display, theme.accent));
    }

    Ok(())
}
