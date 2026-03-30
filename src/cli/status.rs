use std::path::Path;

use clap::Args;

use crate::db::Store;
use crate::error::MocoError;
use crate::models::TaskStatus;
use crate::theme::Theme;
use crate::workspace;

#[derive(Args)]
pub struct StatusArgs {
    /// Display index of the open task to update (e.g. 1, 2, …).
    pub task_id: u32,

    /// New status: an integer 0–100, "complete", or "defer".
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
}

fn parse_status_value(raw: &str) -> Result<StatusValue, MocoError> {
    match raw.to_lowercase().as_str() {
        "complete" | "done" => Ok(StatusValue::Complete),
        "defer" | "deferred" => Ok(StatusValue::Defer),
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

    let mut task = store
        .get_open_task(project_id, args.task_id)?
        .ok_or(MocoError::TaskNotFound(args.task_id))?;

    let value = parse_status_value(&args.value)?;

    match value {
        StatusValue::Progress(pct) => {
            task.progress = pct;
            task.updated_at = chrono::Utc::now();
            store.update_task(&task)?;
            println!(
                "Task {} progress set to {}%.",
                theme.paint(task.display_id(), theme.open),
                theme.paint(pct, theme.accent),
            );
        }
        StatusValue::Complete => {
            let completed_index = store.next_completed_index(project_id)?;
            task.status = TaskStatus::Complete;
            task.progress = 100;
            task.completed_index = Some(completed_index);
            task.display_index = 0;
            task.updated_at = chrono::Utc::now();
            store.update_task(&task)?;
            store.reindex_open_tasks(project_id)?;
            println!(
                "Task marked {} → {}.",
                theme.paint("complete", theme.complete),
                theme.paint(task.display_id(), theme.complete),
            );
        }
        StatusValue::Defer => {
            let deferred_index = store.next_deferred_index(project_id)?;
            task.status = TaskStatus::Defer;
            task.deferred_index = Some(deferred_index);
            task.display_index = 0;
            task.updated_at = chrono::Utc::now();
            store.update_task(&task)?;
            store.reindex_open_tasks(project_id)?;
            println!(
                "Task {} → {}.",
                theme.paint("deferred", theme.defer),
                theme.paint(task.display_id(), theme.defer),
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