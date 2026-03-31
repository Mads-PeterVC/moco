use std::path::Path;

use clap::Args;

use crate::cli::task_ref;
use crate::db::Store;
use crate::error::MocoError;
use crate::models::TaskStatus;
use crate::theme::Theme;
use crate::workspace;

#[derive(Args)]
pub struct StatusArgs {
    /// Task reference: open index (e.g. `1`), subtask (`1.2`), completed (`C1`, `C1.2`),
    /// or deferred (`D1`, `D1.2`). Also accepts `open`/`reopen` to re-open a task.
    pub task_id: String,

    /// New status: an integer 0–100, "complete", "defer", or "open"/"reopen".
    pub value: String,

    /// Operate on the global task list instead of the current project.
    #[arg(short, long)]
    pub global: bool,
}

/// Parsed representation of the status value argument.
enum StatusValue {
    Progress(u8),
    Complete,
    Defer,
    /// Re-open a completed or deferred task.
    Open,
}

fn parse_status_value(raw: &str) -> Result<StatusValue, MocoError> {
    match raw.to_lowercase().as_str() {
        "complete" | "done" => Ok(StatusValue::Complete),
        "defer" | "deferred" => Ok(StatusValue::Defer),
        "open" | "reopen" => Ok(StatusValue::Open),
        _ => {
            let n: u8 = raw
                .parse()
                .map_err(|_| MocoError::InvalidStatus(raw.to_string()))?;
            if n > 100 {
                return Err(MocoError::InvalidStatus(raw.to_string()));
            }
            if n == 100 {
                Ok(StatusValue::Complete)
            } else {
                Ok(StatusValue::Progress(n))
            }
        }
    }
}

pub fn run(args: &StatusArgs, store: &mut dyn Store, cwd: &Path, theme: &Theme) -> anyhow::Result<()> {
    let project_id = if args.global {
        None
    } else {
        workspace::resolve(store, cwd)?.map(|p| p.id)
    };

    let task_ref = task_ref::parse(&args.task_id)
        .map_err(|e| anyhow::anyhow!(e))?;

    let mut task = task_ref::resolve(store, project_id, &task_ref)?
        .ok_or_else(|| anyhow::anyhow!("Task '{}' not found.", args.task_id))?;

    // Fetch parent for display purposes (subtasks).
    let parent = if let Some(pid) = task.parent_id {
        store.get_task_by_id(pid)?
    } else {
        None
    };

    let display = task.display_id_in_context(parent.as_ref());

    let value = parse_status_value(&args.value)?;

    match value {
        StatusValue::Progress(pct) => {
            task.progress = pct;
            task.updated_at = chrono::Utc::now();
            store.update_task(&task)?;
            println!(
                "Task {} progress set to {}%.",
                theme.paint(&display, theme.open),
                theme.paint(pct, theme.accent),
            );
        }
        StatusValue::Complete => {
            let _old_display = display.clone();
            let completed_index = store.next_completed_index(project_id)?;
            let was_open = task.status == TaskStatus::Open;
            // Clear any deferred index if re-completing a deferred task.
            task.deferred_index = None;
            task.status = TaskStatus::Complete;
            task.progress = 100;
            task.completed_index = Some(completed_index);
            if was_open && task.parent_id.is_none() {
                task.display_index = 0;
            }
            task.updated_at = chrono::Utc::now();
            store.update_task(&task)?;
            if was_open && task.parent_id.is_none() {
                store.reindex_open_tasks(project_id)?;
            }
            let new_display = task.display_id_in_context(parent.as_ref());
            println!(
                "Task marked {} → {}.",
                theme.paint("complete", theme.complete),
                theme.paint(new_display, theme.complete),
            );
        }
        StatusValue::Defer => {
            let was_open = task.status == TaskStatus::Open;
            let deferred_index = store.next_deferred_index(project_id)?;
            // Clear completed index if was previously completed.
            task.completed_index = None;
            task.status = TaskStatus::Defer;
            task.deferred_index = Some(deferred_index);
            if was_open && task.parent_id.is_none() {
                task.display_index = 0;
            }
            task.updated_at = chrono::Utc::now();
            store.update_task(&task)?;
            if was_open && task.parent_id.is_none() {
                store.reindex_open_tasks(project_id)?;
            }
            let new_display = task.display_id_in_context(parent.as_ref());
            println!(
                "Task {} → {}.",
                theme.paint("deferred", theme.defer),
                theme.paint(new_display, theme.defer),
            );
        }
        StatusValue::Open => {
            if task.status == TaskStatus::Open {
                println!("Task {} is already open.", theme.paint(&display, theme.open));
                return Ok(());
            }
            // Re-open: assign a new display_index (top-level tasks only).
            task.completed_index = None;
            task.deferred_index = None;
            task.status = TaskStatus::Open;
            if task.parent_id.is_none() {
                task.display_index = store.next_open_display_index(project_id)?;
            }
            task.updated_at = chrono::Utc::now();
            store.update_task(&task)?;
            let new_display = task.display_id_in_context(parent.as_ref());
            println!(
                "Task re-opened → {}.",
                theme.paint(new_display, theme.open),
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_complete_variants() {
        assert!(matches!(
            parse_status_value("complete").unwrap(),
            StatusValue::Complete
        ));
        assert!(matches!(
            parse_status_value("done").unwrap(),
            StatusValue::Complete
        ));
        assert!(matches!(
            parse_status_value("100").unwrap(),
            StatusValue::Complete
        ));
    }

    #[test]
    fn parse_defer_variants() {
        assert!(matches!(
            parse_status_value("defer").unwrap(),
            StatusValue::Defer
        ));
        assert!(matches!(
            parse_status_value("deferred").unwrap(),
            StatusValue::Defer
        ));
    }

    #[test]
    fn parse_open_variants() {
        assert!(matches!(
            parse_status_value("open").unwrap(),
            StatusValue::Open
        ));
        assert!(matches!(
            parse_status_value("reopen").unwrap(),
            StatusValue::Open
        ));
    }

    #[test]
    fn parse_progress_values() {
        assert!(matches!(
            parse_status_value("0").unwrap(),
            StatusValue::Progress(0)
        ));
        assert!(matches!(
            parse_status_value("50").unwrap(),
            StatusValue::Progress(50)
        ));
        assert!(matches!(
            parse_status_value("99").unwrap(),
            StatusValue::Progress(99)
        ));
    }

    #[test]
    fn parse_invalid_value_errors() {
        assert!(parse_status_value("blah").is_err());
        assert!(parse_status_value("101").is_err());
        assert!(parse_status_value("-1").is_err());
    }
}