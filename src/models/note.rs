use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A free-form note attached to a project (or the global list).
///
/// Notes are intentionally simpler than tasks — they have no status, progress, or subtasks.
/// They are meant for capturing ideas, references, and context that doesn't yet warrant a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: Uuid,
    /// `None` means the global note list.
    pub project_id: Option<Uuid>,
    pub title: String,
    /// Body text (Markdown supported). May be empty.
    pub content: String,
    /// Sequential index within the project/global scope: 1, 2, …
    pub display_index: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Note {
    pub fn new(
        project_id: Option<Uuid>,
        title: impl Into<String>,
        content: impl Into<String>,
        display_index: u32,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            project_id,
            title: title.into(),
            content: content.into(),
            display_index,
            created_at: now,
            updated_at: now,
        }
    }

    /// Human-readable ID string, e.g. `"N#1"`.
    pub fn display_id(&self) -> String {
        format!("N#{}", self.display_index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_note_has_unique_ids() {
        let n1 = Note::new(None, "first", "", 1);
        let n2 = Note::new(None, "second", "", 2);
        assert_ne!(n1.id, n2.id);
    }

    #[test]
    fn display_id_uses_n_prefix() {
        let n = Note::new(None, "my note", "", 3);
        assert_eq!(n.display_id(), "N#3");
    }

    #[test]
    fn new_note_stores_title_and_content() {
        let n = Note::new(None, "My idea", "Some details", 1);
        assert_eq!(n.title, "My idea");
        assert_eq!(n.content, "Some details");
    }
}
