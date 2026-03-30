use std::path::Path;

use clap::Args;
use uuid::Uuid;

use crate::db::Store;
use crate::models::{Task, TaskStatus};
use crate::workspace;

#[derive(Args)]
pub struct ListArgs {
    /// List global tasks instead of the current project's tasks.
    #[arg(short, long)]
    pub global: bool,
}

pub fn run(args: &ListArgs, store: &dyn Store, cwd: &Path) -> anyhow::Result<()> {
    let (project_id, scope_label) = if args.global {
        (None, "global".to_string())
    } else {
        match workspace::resolve(store, cwd)? {
            Some(p) => (Some(p.id), format!("project '{}'", p.name)),
            None => (None, "global".to_string()),
        }
    };

    let tasks = store.list_tasks(project_id)?;

    if tasks.is_empty() {
        println!("No tasks for {}.", scope_label);
        return Ok(());
    }

    println!("Tasks for {}:\n", scope_label);

    let open: Vec<&Task> = tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Open)
        .collect();
    let completed: Vec<&Task> = tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Complete)
        .collect();
    let deferred: Vec<&Task> = tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Defer)
        .collect();

    if !open.is_empty() {
        println!("Open");
        println!("{}", "─".repeat(60));
        print_section(&open, &tasks);
        println!();
    }

    if !completed.is_empty() {
        println!("Completed");
        println!("{}", "─".repeat(60));
        print_section(&completed, &tasks);
        println!();
    }

    if !deferred.is_empty() {
        println!("Deferred");
        println!("{}", "─".repeat(60));
        print_section(&deferred, &tasks);
        println!();
    }

    Ok(())
}

/// Print a section of tasks, rendering subtasks indented beneath their parent.
fn print_section(section_tasks: &[&Task], all_tasks: &[Task]) {
    for task in section_tasks {
        // Only print top-level tasks here; subtasks are rendered below their parent.
        if task.parent_id.is_some() {
            continue;
        }
        print_task(task, false);
        print_subtasks(task.id, all_tasks);
    }
}

/// Recursively print subtasks of `parent_id`, indented with a tree connector.
fn print_subtasks(parent_id: Uuid, all_tasks: &[Task]) {
    let children: Vec<&Task> = all_tasks
        .iter()
        .filter(|t| t.parent_id == Some(parent_id))
        .collect();

    for child in children {
        print_task(child, true);
        print_subtasks(child.id, all_tasks);
    }
}

fn print_task(task: &Task, is_subtask: bool) {
    let id = task.display_id();
    let bar = progress_bar(task.progress, 20);
    let preview = task.content.lines().next().unwrap_or("").trim();
    let preview = if preview.len() > 50 {
        format!("{}…", &preview[..50])
    } else {
        preview.to_string()
    };

    if is_subtask {
        println!("        └─ {:>5}  {}  {}", id, bar, preview);
    } else {
        println!("  {:>5}  {}  {}", id, bar, preview);
    }
}

/// Render a compact ASCII progress bar, e.g. `[████████░░░░░░░░░░░░]`.
pub fn progress_bar(progress: u8, width: usize) -> String {
    let filled = ((progress as usize) * width) / 100;
    let empty = width - filled;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_bar_empty() {
        let bar = progress_bar(0, 10);
        assert_eq!(bar, "[░░░░░░░░░░]");
    }

    #[test]
    fn progress_bar_full() {
        let bar = progress_bar(100, 10);
        assert_eq!(bar, "[██████████]");
    }

    #[test]
    fn progress_bar_half() {
        let bar = progress_bar(50, 10);
        assert_eq!(bar, "[█████░░░░░]");
    }
}

