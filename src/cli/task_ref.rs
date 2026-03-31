use crate::db::Store;
use crate::models::Task;
use uuid::Uuid;

/// A parsed reference to a specific task used in CLI commands like `status` and `edit`.
///
/// Accepted string formats:
/// | Input    | Meaning                                           |
/// |----------|---------------------------------------------------|
/// | `1`      | Open top-level task #1                            |
/// | `1.2`    | Open subtask S#1.2 (sub_index 2 of open parent #1)|
/// | `C1`     | Completed top-level task C#1                      |
/// | `C1.2`   | Subtask with sub_index 2 of completed parent C#1  |
/// | `D1`     | Deferred top-level task D#1                       |
/// | `D1.2`   | Subtask with sub_index 2 of deferred parent D#1   |
///
/// The prefix is case-insensitive (`c1` == `C1`).
#[derive(Debug, PartialEq, Eq)]
pub enum TaskRef {
    /// Open top-level task by display_index.
    Open(u32),
    /// Open subtask: (parent open display_index, sub_index).
    OpenSub(u32, u32),
    /// Completed top-level task by completed_index.
    Completed(u32),
    /// Subtask of a completed parent: (parent completed_index, sub_index).
    CompletedSub(u32, u32),
    /// Deferred top-level task by deferred_index.
    Deferred(u32),
    /// Subtask of a deferred parent: (parent deferred_index, sub_index).
    DeferredSub(u32, u32),
}

/// Parse a task reference string into a [`TaskRef`].
///
/// Returns an error string suitable for user-facing messages on failure.
pub fn parse(s: &str) -> Result<TaskRef, String> {
    if s.is_empty() {
        return Err("Task reference cannot be empty.".to_string());
    }

    let upper = s.to_uppercase();

    // Determine prefix (C / D / none) and the remainder.
    let (prefix, rest) = if upper.starts_with('C') {
        ("C", &s[1..])
    } else if upper.starts_with('D') {
        ("D", &s[1..])
    } else {
        ("", s)
    };

    // The remainder may be "N" or "N.M".
    let (primary_str, sub_str) = if let Some((a, b)) = rest.split_once('.') {
        (a, Some(b))
    } else {
        (rest, None)
    };

    let primary: u32 = primary_str
        .parse()
        .map_err(|_| format!("Invalid task reference '{s}'."))?;

    if let Some(sub_raw) = sub_str {
        let sub: u32 = sub_raw
            .parse()
            .map_err(|_| format!("Invalid sub-index in task reference '{s}'."))?;

        if sub == 0 {
            return Err(format!("Sub-index in '{s}' must be ≥ 1."));
        }

        return Ok(match prefix {
            "C" => TaskRef::CompletedSub(primary, sub),
            "D" => TaskRef::DeferredSub(primary, sub),
            _ => TaskRef::OpenSub(primary, sub),
        });
    }

    if primary == 0 {
        return Err(format!("Task index in '{s}' must be ≥ 1."));
    }

    Ok(match prefix {
        "C" => TaskRef::Completed(primary),
        "D" => TaskRef::Deferred(primary),
        _ => TaskRef::Open(primary),
    })
}

/// Resolve a [`TaskRef`] to a concrete [`Task`] from the store.
///
/// Returns `None` when no matching task is found (the caller should surface a
/// user-friendly "not found" error).
pub fn resolve(
    store: &dyn Store,
    project_id: Option<Uuid>,
    task_ref: &TaskRef,
) -> anyhow::Result<Option<Task>> {
    use TaskRef::*;
    let task = match *task_ref {
        Open(idx) => store.get_open_task(project_id, idx)?,
        Completed(idx) => store.get_completed_task(project_id, idx)?,
        Deferred(idx) => store.get_deferred_task(project_id, idx)?,
        OpenSub(parent_idx, sub_idx) => {
            store.get_open_subtask(project_id, parent_idx, sub_idx)?
        }
        CompletedSub(parent_idx, sub_idx) => {
            store.get_completed_parent_subtask(project_id, parent_idx, sub_idx)?
        }
        DeferredSub(parent_idx, sub_idx) => {
            store.get_deferred_parent_subtask(project_id, parent_idx, sub_idx)?
        }
    };
    Ok(task)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_open_top_level() {
        assert_eq!(parse("1").unwrap(), TaskRef::Open(1));
        assert_eq!(parse("42").unwrap(), TaskRef::Open(42));
    }

    #[test]
    fn parse_open_subtask() {
        assert_eq!(parse("1.1").unwrap(), TaskRef::OpenSub(1, 1));
        assert_eq!(parse("3.2").unwrap(), TaskRef::OpenSub(3, 2));
    }

    #[test]
    fn parse_completed_top_level() {
        assert_eq!(parse("C1").unwrap(), TaskRef::Completed(1));
        assert_eq!(parse("c5").unwrap(), TaskRef::Completed(5));
    }

    #[test]
    fn parse_completed_subtask() {
        assert_eq!(parse("C1.1").unwrap(), TaskRef::CompletedSub(1, 1));
        assert_eq!(parse("c2.3").unwrap(), TaskRef::CompletedSub(2, 3));
    }

    #[test]
    fn parse_deferred_top_level() {
        assert_eq!(parse("D1").unwrap(), TaskRef::Deferred(1));
        assert_eq!(parse("d2").unwrap(), TaskRef::Deferred(2));
    }

    #[test]
    fn parse_deferred_subtask() {
        assert_eq!(parse("D1.2").unwrap(), TaskRef::DeferredSub(1, 2));
    }

    #[test]
    fn parse_zero_index_fails() {
        assert!(parse("0").is_err());
    }

    #[test]
    fn parse_zero_sub_index_fails() {
        assert!(parse("1.0").is_err());
    }

    #[test]
    fn parse_empty_fails() {
        assert!(parse("").is_err());
    }

    #[test]
    fn parse_garbage_fails() {
        assert!(parse("abc").is_err());
        assert!(parse("C").is_err());
    }
}
