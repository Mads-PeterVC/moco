use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// A registered workspace project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    /// Canonical absolute path to the workspace root.
    pub path: PathBuf,
    /// User-defined labels for cross-project categorisation (e.g. "work", "personal").
    #[serde(default)]
    pub labels: Vec<String>,
    /// The name of the category this project belongs to, if any.
    #[serde(default)]
    pub category: Option<String>,
    pub created_at: DateTime<Utc>,
    /// Updated whenever a task is added/modified/completed or the project is opened.
    /// Existing records without this field default to the Unix epoch (sort last).
    #[serde(default = "epoch")]
    pub last_active: DateTime<Utc>,
    /// Cached git remote URL for this project's repository.
    ///
    /// Populated by live git discovery and persisted so moco can display it even when the
    /// repository is temporarily absent (and will enable "remote-only" projects in a future phase).
    #[serde(default)]
    pub git_remote: Option<String>,
}

fn epoch() -> DateTime<Utc> {
    DateTime::from_timestamp(0, 0).unwrap_or_default()
}

impl Project {
    pub fn new(name: impl Into<String>, path: PathBuf) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            path,
            labels: Vec::new(),
            category: None,
            created_at: now,
            last_active: now,
            git_remote: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_project_has_unique_ids() {
        let p1 = Project::new("alpha", PathBuf::from("/tmp/alpha"));
        let p2 = Project::new("beta", PathBuf::from("/tmp/beta"));
        assert_ne!(p1.id, p2.id);
    }

    #[test]
    fn new_project_stores_name_and_path() {
        let path = PathBuf::from("/home/user/myproject");
        let p = Project::new("myproject", path.clone());
        assert_eq!(p.name, "myproject");
        assert_eq!(p.path, path);
    }

    #[test]
    fn new_project_git_remote_is_none() {
        let p = Project::new("test", PathBuf::from("/tmp/test"));
        assert!(p.git_remote.is_none());
    }

    #[test]
    fn git_remote_round_trips_through_serde() {
        let mut p = Project::new("test", PathBuf::from("/tmp/test"));
        p.git_remote = Some("https://github.com/user/repo.git".to_string());
        let json = serde_json::to_string(&p).unwrap();
        let restored: Project = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.git_remote.as_deref(), Some("https://github.com/user/repo.git"));
    }

    #[test]
    fn git_remote_defaults_to_none_in_old_records() {
        // Simulate a JSON blob that does not have the git_remote field (old record).
        let json = r#"{"id":"00000000-0000-0000-0000-000000000001","name":"old","path":"/tmp/old","labels":[],"created_at":"2024-01-01T00:00:00Z","last_active":"2024-01-01T00:00:00Z"}"#;
        let p: Project = serde_json::from_str(json).unwrap();
        assert!(p.git_remote.is_none(), "git_remote should default to None for old records");
    }
}
