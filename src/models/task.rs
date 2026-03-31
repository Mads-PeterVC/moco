use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The lifecycle state of a task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Open,
    Complete,
    Defer,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Open => write!(f, "open"),
            TaskStatus::Complete => write!(f, "complete"),
            TaskStatus::Defer => write!(f, "defer"),
        }
    }
}

/// A single task (or subtask) within a project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    /// `None` means the global task list.
    pub project_id: Option<Uuid>,
    pub content: String,
    pub status: TaskStatus,
    /// Progress percentage (0–100). 100 ⇒ Complete.
    pub progress: u8,
    /// Display index among open top-level tasks: 1, 2, 3, …
    /// Always 0 for subtasks (which use `sub_index` instead).
    pub display_index: u32,
    /// Display index among completed tasks: C#1, C#2, …
    pub completed_index: Option<u32>,
    /// Display index among deferred tasks: D#1, D#2, …
    pub deferred_index: Option<u32>,
    /// Parent task UUID for subtasks.
    pub parent_id: Option<Uuid>,
    /// 1-based position within the parent's subtask list.
    /// `None` for top-level tasks, `Some(n)` for subtasks.
    #[serde(default)]
    pub sub_index: Option<u32>,
    /// Free-form keyword tags for filtering (e.g. "urgent", "frontend").
    #[serde(default)]
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Task {
    pub fn new(
        project_id: Option<Uuid>,
        content: impl Into<String>,
        display_index: u32,
        parent_id: Option<Uuid>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            project_id,
            content: content.into(),
            status: TaskStatus::Open,
            progress: 0,
            display_index,
            completed_index: None,
            deferred_index: None,
            parent_id,
            sub_index: None,
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Human-readable ID for a top-level task: `"#3"`, `"C#1"`, `"D#2"`.
    /// For a subtask without parent context, prefer `display_id_in_context`.
    pub fn display_id(&self) -> String {
        match self.status {
            TaskStatus::Open => format!("#{}", self.display_index),
            TaskStatus::Complete => format!(
                "C#{}",
                self.completed_index
                    .expect("completed task must have completed_index")
            ),
            TaskStatus::Defer => format!(
                "D#{}",
                self.deferred_index
                    .expect("deferred task must have deferred_index")
            ),
        }
    }

    /// Human-readable ID with optional parent context for subtasks.
    ///
    /// - Top-level task: identical to `display_id()`.
    /// - Subtask with parent: `"S#P.N"`, `"CS#P.N"`, `"DS#P.N"` where `P` is
    ///   the parent's current display reference and `N` is `sub_index`.
    pub fn display_id_in_context(&self, parent: Option<&Task>) -> String {
        let sub = match self.sub_index {
            Some(s) => s,
            None => return self.display_id(),
        };

        let parent_ref = match parent {
            Some(p) => match p.status {
                TaskStatus::Open => format!("{}", p.display_index),
                TaskStatus::Complete => format!(
                    "C{}",
                    p.completed_index.expect("completed parent has completed_index")
                ),
                TaskStatus::Defer => format!(
                    "D{}",
                    p.deferred_index.expect("deferred parent has deferred_index")
                ),
            },
            // Parent not available — show a placeholder.
            None => "?".to_string(),
        };

        match self.status {
            TaskStatus::Open => format!("S#{parent_ref}.{sub}"),
            TaskStatus::Complete => format!("CS#{parent_ref}.{sub}"),
            TaskStatus::Defer => format!("DS#{parent_ref}.{sub}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_task(status: TaskStatus, display_index: u32) -> Task {
        let mut t = Task::new(None, "test task", display_index, None);
        t.status = status;
        t
    }

    #[test]
    fn new_task_is_open_with_zero_progress() {
        let t = Task::new(None, "do something", 1, None);
        assert_eq!(t.status, TaskStatus::Open);
        assert_eq!(t.progress, 0);
        assert_eq!(t.display_index, 1);
    }

    #[test]
    fn display_id_open() {
        let t = make_task(TaskStatus::Open, 3);
        assert_eq!(t.display_id(), "#3");
    }

    #[test]
    fn display_id_complete() {
        let mut t = make_task(TaskStatus::Complete, 0);
        t.completed_index = Some(2);
        assert_eq!(t.display_id(), "C#2");
    }

    #[test]
    fn display_id_defer() {
        let mut t = make_task(TaskStatus::Defer, 0);
        t.deferred_index = Some(1);
        assert_eq!(t.display_id(), "D#1");
    }

    #[test]
    fn display_id_in_context_top_level_delegates_to_display_id() {
        let t = make_task(TaskStatus::Open, 5);
        assert_eq!(t.display_id_in_context(None), "#5");
    }

    #[test]
    fn display_id_in_context_open_subtask_of_open_parent() {
        let parent = make_task(TaskStatus::Open, 2);
        let mut child = Task::new(None, "sub", 0, Some(parent.id));
        child.sub_index = Some(1);
        assert_eq!(child.display_id_in_context(Some(&parent)), "S#2.1");
    }

    #[test]
    fn display_id_in_context_completed_subtask_of_open_parent() {
        let parent = make_task(TaskStatus::Open, 1);
        let mut child = Task::new(None, "sub", 0, Some(parent.id));
        child.sub_index = Some(1);
        child.status = TaskStatus::Complete;
        child.completed_index = Some(1);
        assert_eq!(child.display_id_in_context(Some(&parent)), "CS#1.1");
    }

    #[test]
    fn display_id_in_context_subtask_of_completed_parent() {
        let mut parent = make_task(TaskStatus::Complete, 0);
        parent.completed_index = Some(3);
        let mut child = Task::new(None, "sub", 0, Some(parent.id));
        child.sub_index = Some(2);
        assert_eq!(child.display_id_in_context(Some(&parent)), "S#C3.2");
    }

    #[test]
    fn display_id_in_context_no_parent_shows_placeholder() {
        let mut child = Task::new(None, "sub", 0, Some(Uuid::new_v4()));
        child.sub_index = Some(1);
        assert_eq!(child.display_id_in_context(None), "S#?.1");
    }
}
