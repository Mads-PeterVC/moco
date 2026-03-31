use std::path::Path;
use std::process::Command;

use gix::bstr::ByteSlice;
use gix::remote::Direction;

use super::backend::{GitBackend, GitInfo};

/// [`GitBackend`] implementation powered by the [`gix`] crate.
///
/// Network operations (`fetch`) delegate to the `git` subprocess so that we
/// avoid the complexity of gix's async-only network client while still keeping
/// the operation behind the swappable `GitBackend` trait.
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

        // ── Local divergence (no network) ────────────────────────────────────
        let (local_ahead, local_behind) =
            Self::local_divergence(path)
                .map(|(a, b)| (Some(a), Some(b)))
                .unwrap_or((None, None));

        Some(GitInfo {
            branch: branch_name,
            remote_url,
            remote_name,
            local_ahead,
            local_behind,
        })
    }

    fn local_divergence(path: &Path) -> Option<(u32, u32)> {
        let repo = gix::discover(path).ok()?;

        // Find the remote tracking ref for the current branch.
        let head = repo.head().ok()?;
        let branch_name = match &head.kind {
            gix::head::Kind::Symbolic(r) => r.name.shorten().to_str_lossy().into_owned(),
            gix::head::Kind::Unborn(_) => return None, // no commits yet
            gix::head::Kind::Detached { .. } => return None,
        };

        // Build the tracking ref name: refs/remotes/<remote>/<branch>.
        let remote_name = repo
            .branch_remote_name(branch_name.as_bytes().as_bstr(), Direction::Fetch)
            .map(|n| n.as_bstr().to_str_lossy().into_owned())
            .unwrap_or_else(|| "origin".to_string());

        let tracking_ref = format!("refs/remotes/{}/{}", remote_name, branch_name);

        // Verify the tracking ref exists.
        if repo.find_reference(tracking_ref.as_str()).is_err() {
            return None;
        }

        // Use `git rev-list --count` for simplicity and reliability.
        // These are two fast local object-store reads — typically < 1 ms.
        let ahead = rev_list_count(path, &format!("{}..HEAD", tracking_ref))?;
        let behind = rev_list_count(path, &format!("HEAD..{}", tracking_ref))?;

        Some((ahead, behind))
    }

    fn fetch(path: &Path) -> Result<(), String> {
        let output = Command::new("git")
            .args(["fetch", "--quiet"])
            .current_dir(path)
            .output()
            .map_err(|e| format!("failed to run git: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(stderr.trim().to_string())
        }
    }
}

/// Run `git rev-list --count <range>` and parse the result.
fn rev_list_count(path: &Path, range: &str) -> Option<u32> {
    let output = Command::new("git")
        .args(["rev-list", "--count", range])
        .current_dir(path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let s = String::from_utf8_lossy(&output.stdout);
    s.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    /// Create a minimal git repo with one commit and return its TempDir.
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
        assert!(info.remote_url.is_none());
        assert!(info.remote_name.is_none());
    }

    #[test]
    fn local_divergence_none_without_tracking_branch() {
        let dir = make_git_repo();
        let divergence = GixBackend::local_divergence(dir.path());
        assert!(divergence.is_none(), "no tracking branch → should return None");
    }

    #[test]
    fn local_divergence_ahead_after_local_commit() {
        // Create a "remote" bare repo, clone it, add a commit — should be 1 ahead.
        let remote_dir = TempDir::new().unwrap();
        Command::new("git")
            .args(["init", "--bare", "-b", "main"])
            .current_dir(remote_dir.path())
            .status()
            .unwrap();

        let clone_dir = TempDir::new().unwrap();
        // Clone the empty bare repo (need an initial commit first).
        // Strategy: init bare with a seed commit via a temporary repo.
        let seed_dir = TempDir::new().unwrap();
        let run = |args: &[&str], d: &std::path::Path| {
            Command::new("git").args(args).current_dir(d).status().unwrap();
        };
        run(&["init", "-b", "main"], seed_dir.path());
        run(&["config", "user.email", "t@t.com"], seed_dir.path());
        run(&["config", "user.name", "T"], seed_dir.path());
        std::fs::write(seed_dir.path().join("f"), "seed").unwrap();
        run(&["add", "."], seed_dir.path());
        run(&["commit", "-m", "seed"], seed_dir.path());
        run(
            &[
                "clone",
                seed_dir.path().to_str().unwrap(),
                clone_dir.path().to_str().unwrap(),
            ],
            seed_dir.path(),
        );
        run(&["config", "user.email", "t@t.com"], clone_dir.path());
        run(&["config", "user.name", "T"], clone_dir.path());

        // Add a commit on the clone — now 1 ahead of origin/main.
        std::fs::write(clone_dir.path().join("new.txt"), "new").unwrap();
        run(&["add", "."], clone_dir.path());
        run(&["commit", "-m", "local commit"], clone_dir.path());

        let (ahead, behind) =
            GixBackend::local_divergence(clone_dir.path()).expect("should compute divergence");
        assert_eq!(ahead, 1, "should be 1 ahead");
        assert_eq!(behind, 0, "should be 0 behind");
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
    fn git_info_has_no_divergence_without_tracking_branch() {
        let dir = make_git_repo();
        let info = GixBackend::discover(dir.path()).unwrap();
        assert!(info.local_ahead.is_none());
        assert!(info.local_behind.is_none());
    }

    #[test]
    fn returns_none_outside_git_repo() {
        let dir = TempDir::new().expect("temp dir");
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
