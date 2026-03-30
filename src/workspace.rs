use std::path::{Path, PathBuf};

use crate::db::Store;
use crate::error::MocoError;
use crate::models::Project;

/// Walk from `cwd` upward, returning the first registered project whose path
/// is an ancestor-or-equal of `cwd`. Returns `None` if no match is found
/// (callers should then use the global task list).
pub fn resolve(store: &dyn Store, cwd: &Path) -> Result<Option<Project>, MocoError> {
    let projects = store.list_projects()?;

    // Collect all candidates whose path is a prefix of cwd, then pick the
    // longest match (most specific workspace wins).
    let mut best: Option<&Project> = None;
    let mut best_len = 0usize;

    for project in &projects {
        let project_path = &project.path;
        if cwd.starts_with(project_path) {
            let len = project_path.components().count();
            if len > best_len {
                best_len = len;
                best = Some(project);
            }
        }
    }

    Ok(best.cloned())
}

/// Return the canonical form of `path`, falling back to the original if
/// canonicalization fails (e.g. path does not exist on this machine).
pub fn canonical(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    use crate::db::RedbStore;

    fn temp_store() -> (TempDir, RedbStore) {
        let dir = TempDir::new().unwrap();
        let store = RedbStore::open(&dir.path().join("test.db")).unwrap();
        (dir, store)
    }

    #[test]
    fn resolve_returns_none_when_no_projects() {
        let (_dir, store) = temp_store();
        let result = resolve(&store, Path::new("/some/path")).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn resolve_returns_matching_project() {
        let (_dir, mut store) = temp_store();
        store
            .create_project("myproject", Path::new("/home/user/myproject"))
            .unwrap();

        let result = resolve(&store, Path::new("/home/user/myproject/src")).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "myproject");
    }

    #[test]
    fn resolve_picks_most_specific_match() {
        let (_dir, mut store) = temp_store();
        store
            .create_project("parent", Path::new("/home/user/projects"))
            .unwrap();
        store
            .create_project("child", Path::new("/home/user/projects/child"))
            .unwrap();

        let result = resolve(&store, Path::new("/home/user/projects/child/src")).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "child");
    }

    #[test]
    fn resolve_returns_none_for_unregistered_path() {
        let (_dir, mut store) = temp_store();
        store
            .create_project("myproject", Path::new("/home/user/myproject"))
            .unwrap();

        let result = resolve(&store, Path::new("/home/user/other")).unwrap();
        assert!(result.is_none());
    }
}
