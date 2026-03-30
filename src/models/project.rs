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
    pub created_at: DateTime<Utc>,
}

impl Project {
    pub fn new(name: impl Into<String>, path: PathBuf) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            path,
            labels: Vec::new(),
            created_at: Utc::now(),
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
}
