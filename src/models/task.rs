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
    /// Display index among open tasks: 1, 2, 3, …
    pub display_index: u32,
    /// Display index among completed tasks: C#1, C#2, …
    pub completed_index: Option<u32>,
    /// Display index among deferred tasks: D#1, D#2, …
    pub deferred_index: Option<u32>,
    /// Parent task UUID for subtasks.
    pub parent_id: Option<Uuid>,
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
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Human-readable ID string, e.g. "#3", "C#1", "D#2".
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
    fn task_status_display() {
        assert_eq!(TaskStatus::Open.to_string(), "open");
        assert_eq!(TaskStatus::Complete.to_string(), "complete");
        assert_eq!(TaskStatus::Defer.to_string(), "defer");
    }
}
