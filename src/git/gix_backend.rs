use std::path::Path;

use gix::bstr::ByteSlice;
use gix::remote::Direction;

use super::backend::{GitBackend, GitInfo};

/// [`GitBackend`] implementation powered by the [`gix`] crate.
pub struct GixBackend;

impl GitBackend for GixBackend {
    fn discover(path: &Path) -> Option<GitInfo> {
        let repo = gix::discover(path).ok()?;

        // ── Branch name ──────────────────────────────────────────────────────
        let head = repo.head().ok()?;
        let branch_name: Option<String> = match &head.kind {
            gix::head::Kind::Symbolic(r) => Some(r.name.shorten().to_str_lossy().into_owned()),
            gix::head::Kind::Unborn(name) => {
                Some(name.shorten().to_str_lossy().into_owned())
            }
            gix::head::Kind::Detached { .. } => None,
        };

        // ── Remote name: upstream of current branch, then fall back to origin ─
        let remote_name: Option<String> = branch_name
            .as_deref()
            .and_then(|branch| {
                repo.branch_remote_name(branch.as_bytes().as_bstr(), Direction::Fetch)
                    .map(|n| n.as_bstr().to_str_lossy().into_owned())
            })
            .or_else(|| {
                // Fall back to "origin" if it exists.
                repo.try_find_remote("origin".as_bytes().as_bstr())
                    .and_then(|r| r.ok())
                    .map(|_| "origin".to_string())
            });

        // ── Remote URL ───────────────────────────────────────────────────────
        let remote_url: Option<String> = remote_name.as_deref().and_then(|name| {
            repo.try_find_remote(name.as_bytes().as_bstr())
                .and_then(|r| r.ok())
                .and_then(|r| r.url(Direction::Fetch).map(|u| u.to_string()))
        });

        Some(GitInfo {
            branch: branch_name,
            remote_url,
            remote_name,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    /// Create a minimal git repo and return its TempDir (keeps it alive).
    fn make_git_repo() -> TempDir {
        let dir = TempDir::new().expect("temp dir");
        let run = |args: &[&str]| {
            let status = Command::new("git")
                .args(args)
                .current_dir(dir.path())
                .status()
                .expect("git command");
            assert!(status.success(), "git {:?} failed", args);
        };
        run(&["init", "-b", "main"]);
        run(&["config", "user.email", "test@example.com"]);
        run(&["config", "user.name", "Test"]);
        // Need at least one commit for HEAD to resolve properly.
        std::fs::write(dir.path().join("README.md"), "# test").unwrap();
        run(&["add", "."]);
        run(&["commit", "-m", "init"]);
        dir
    }

    #[test]
    fn detects_branch_in_git_repo() {
        let dir = make_git_repo();
        let info = GixBackend::discover(dir.path()).expect("should detect git repo");
        assert_eq!(info.branch.as_deref(), Some("main"));
    }

    #[test]
    fn no_remote_when_none_configured() {
        let dir = make_git_repo();
        let info = GixBackend::discover(dir.path()).expect("should detect git repo");
        assert!(info.remote_url.is_none(), "expected no remote URL");
        assert!(info.remote_name.is_none(), "expected no remote name");
    }

    #[test]
    fn detects_origin_remote() {
        let dir = make_git_repo();
        Command::new("git")
            .args([
                "remote",
                "add",
                "origin",
                "https://github.com/example/repo.git",
            ])
            .current_dir(dir.path())
            .status()
            .expect("git remote add");

        let info = GixBackend::discover(dir.path()).expect("should detect git repo");
        assert_eq!(info.remote_name.as_deref(), Some("origin"));
        assert_eq!(
            info.remote_url.as_deref(),
            Some("https://github.com/example/repo.git")
        );
    }

    #[test]
    fn returns_none_outside_git_repo() {
        let dir = TempDir::new().expect("temp dir");
        // Plain directory – no git repo.
        let info = GixBackend::discover(dir.path());
        assert!(info.is_none());
    }

    #[test]
    fn discovers_from_subdirectory() {
        let dir = make_git_repo();
        let subdir = dir.path().join("src");
        std::fs::create_dir_all(&subdir).unwrap();
        let info = GixBackend::discover(&subdir).expect("should detect git repo from subdir");
        assert_eq!(info.branch.as_deref(), Some("main"));
    }
}
