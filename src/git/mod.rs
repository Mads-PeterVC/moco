mod backend;
mod gix_backend;

pub use backend::{GitBackend, GitInfo};
pub use gix_backend::GixBackend;

use std::path::Path;

/// Discover git information for the repository at or above `path`.
///
/// This is the single entry point used throughout the codebase.  It delegates
/// to the active backend ([`GixBackend`]) and returns `None` silently when
/// `path` is not inside a git repository.
pub fn git_info(path: &Path) -> Option<GitInfo> {
    GixBackend::discover(path)
}

/// Format git branch and remote for display in the terminal UI.
///
/// Returns a string like `"⎇ main  ↑ git@github.com:user/repo.git"` or just
/// `"⎇ main"` when no remote is configured.  Returns an empty string when
/// `info` has neither branch nor remote.
pub fn format_git_info(info: &GitInfo) -> String {
    match (&info.branch, &info.remote_url) {
        (Some(branch), Some(url)) => format!("⎇ {}  ↑ {}", branch, url),
        (Some(branch), None) => format!("⎇ {}", branch),
        (None, Some(url)) => format!("↑ {}", url),
        (None, None) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(branch: Option<&str>, remote_url: Option<&str>) -> GitInfo {
        GitInfo {
            branch: branch.map(str::to_owned),
            remote_url: remote_url.map(str::to_owned),
            remote_name: None,
        }
    }

    #[test]
    fn format_branch_and_remote() {
        let s = format_git_info(&info(Some("main"), Some("https://github.com/u/r.git")));
        assert_eq!(s, "⎇ main  ↑ https://github.com/u/r.git");
    }

    #[test]
    fn format_branch_only() {
        let s = format_git_info(&info(Some("feature/x"), None));
        assert_eq!(s, "⎇ feature/x");
    }

    #[test]
    fn format_remote_only() {
        let s = format_git_info(&info(None, Some("https://example.com/r.git")));
        assert_eq!(s, "↑ https://example.com/r.git");
    }

    #[test]
    fn format_empty_when_neither() {
        let s = format_git_info(&info(None, None));
        assert!(s.is_empty());
    }
}
