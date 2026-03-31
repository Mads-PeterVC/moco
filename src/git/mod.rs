mod backend;
mod gix_backend;

pub use backend::{GitBackend, GitInfo};
pub use gix_backend::GixBackend;

use std::path::Path;

use chrono::{DateTime, Utc};

/// Discover git information for the repository at or above `path`.
///
/// This is the single entry point used throughout the codebase.  It delegates
/// to the active backend ([`GixBackend`]) and returns `None` silently when
/// `path` is not inside a git repository.
pub fn git_info(path: &Path) -> Option<GitInfo> {
    GixBackend::discover(path)
}

/// Count commits ahead/behind the remote tracking branch without network I/O.
///
/// Returns `Some((ahead, behind))` when a tracking ref is configured, `None`
/// otherwise.
pub fn local_divergence(path: &Path) -> Option<(u32, u32)> {
    GixBackend::local_divergence(path)
}

/// Run `git fetch` for the repository at `path` (network operation).
///
/// Returns `Err` with a human-readable message on failure.
pub fn fetch(path: &Path) -> Result<(), String> {
    GixBackend::fetch(path)
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

/// Format local divergence (HEAD vs tracking branch) as a compact string.
///
/// Examples: `"↑ 2  ↓ 3"`, `"↑ 2"`, `"↓ 1"`, `"✓"` (up to date).
pub fn format_local_divergence(ahead: u32, behind: u32) -> String {
    match (ahead, behind) {
        (0, 0) => "✓".to_string(),
        (a, 0) => format!("↑ {}", a),
        (0, b) => format!("↓ {}", b),
        (a, b) => format!("↑ {}  ↓ {}", a, b),
    }
}

/// Format cached remote divergence with TTL awareness.
///
/// Returns `None` when the cached data is absent or has expired (age >
/// `ttl_hours`).  Returns a `Some(String)` with divergence info and a
/// human-friendly age annotation otherwise.
pub fn format_cached_divergence(
    ahead: Option<u32>,
    behind: Option<u32>,
    last_check: Option<DateTime<Utc>>,
    ttl_hours: u64,
) -> Option<String> {
    let checked_at = last_check?;
    let age = Utc::now().signed_duration_since(checked_at);
    let age_hours = age.num_hours();

    // Treat negative durations (clock skew) as zero.
    let age_hours_u = age_hours.max(0) as u64;

    if age_hours_u >= ttl_hours {
        return None; // Stale — caller should show a "run moco sync status" hint.
    }

    let divergence = format_local_divergence(ahead.unwrap_or(0), behind.unwrap_or(0));
    let age_str = if age_hours_u == 0 {
        let mins = age.num_minutes().max(0);
        if mins < 2 { "just now".to_string() } else { format!("{}m ago", mins) }
    } else if age_hours_u == 1 {
        "1h ago".to_string()
    } else {
        format!("{}h ago", age_hours_u)
    };

    Some(format!("{}  (checked {})", divergence, age_str))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(branch: Option<&str>, remote_url: Option<&str>) -> GitInfo {
        GitInfo {
            branch: branch.map(str::to_owned),
            remote_url: remote_url.map(str::to_owned),
            remote_name: None,
            local_ahead: None,
            local_behind: None,
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

    // ── format_local_divergence ───────────────────────────────────────────────

    #[test]
    fn divergence_up_to_date() {
        assert_eq!(format_local_divergence(0, 0), "✓");
    }

    #[test]
    fn divergence_ahead_only() {
        assert_eq!(format_local_divergence(3, 0), "↑ 3");
    }

    #[test]
    fn divergence_behind_only() {
        assert_eq!(format_local_divergence(0, 2), "↓ 2");
    }

    #[test]
    fn divergence_both() {
        assert_eq!(format_local_divergence(2, 3), "↑ 2  ↓ 3");
    }

    // ── format_cached_divergence ─────────────────────────────────────────────

    #[test]
    fn cached_divergence_none_when_never_checked() {
        assert!(format_cached_divergence(None, None, None, 12).is_none());
    }

    #[test]
    fn cached_divergence_none_when_stale() {
        let old = Utc::now() - chrono::Duration::hours(13);
        assert!(format_cached_divergence(Some(0), Some(0), Some(old), 12).is_none());
    }

    #[test]
    fn cached_divergence_some_when_fresh() {
        let recent = Utc::now() - chrono::Duration::hours(2);
        let result = format_cached_divergence(Some(1), Some(0), Some(recent), 12);
        assert!(result.is_some());
        let s = result.unwrap();
        assert!(s.contains("↑ 1"), "expected ahead marker in: {}", s);
        assert!(s.contains("2h ago"), "expected age in: {}", s);
    }

    #[test]
    fn cached_divergence_up_to_date_when_fresh() {
        let recent = Utc::now() - chrono::Duration::hours(1);
        let result = format_cached_divergence(Some(0), Some(0), Some(recent), 12).unwrap();
        assert!(result.contains("✓"));
    }
}
