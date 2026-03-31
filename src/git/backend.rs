use std::path::Path;

/// Snapshot of git metadata resolved for a project directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitInfo {
    /// Name of the currently checked-out branch (e.g. `"main"`).
    /// `None` when HEAD is detached.
    pub branch: Option<String>,
    /// URL of the upstream remote (tracked upstream for the current branch, with
    /// fallback to `origin`).  `None` when no suitable remote is configured.
    pub remote_url: Option<String>,
    /// Short name of the resolved remote (e.g. `"origin"`, `"upstream"`).
    pub remote_name: Option<String>,
    /// Commits in HEAD that are not yet in the remote tracking branch (unpushed).
    /// `None` when there is no tracking branch or the count could not be determined.
    pub local_ahead: Option<u32>,
    /// Commits in the remote tracking branch that are not yet in HEAD (need to pull).
    /// `None` when there is no tracking branch or the count could not be determined.
    pub local_behind: Option<u32>,
}

/// Abstraction over a git backend so the underlying implementation (currently
/// `gix`) can be swapped without touching the rest of the codebase.
pub trait GitBackend {
    /// Attempt to discover a git repository at or above `path` and return its
    /// metadata.  Returns `None` when `path` is not inside a git repository or
    /// when discovery fails for any reason (errors are silently ignored so that
    /// non-git projects are handled gracefully).
    ///
    /// The returned `GitInfo` includes local divergence counts (ahead/behind the
    /// remote tracking branch) when a tracking branch is configured.
    fn discover(path: &Path) -> Option<GitInfo>
    where
        Self: Sized;

    /// Count commits ahead/behind the remote tracking branch locally (no network).
    ///
    /// Returns `Some((ahead, behind))` when a tracking ref exists, or `None`
    /// when no tracking branch is configured or the counts cannot be determined.
    fn local_divergence(path: &Path) -> Option<(u32, u32)>
    where
        Self: Sized;

    /// Run `git fetch` for the configured upstream remote (network operation).
    ///
    /// Returns `Err` with a human-readable description on failure (no network,
    /// auth error, etc.).
    fn fetch(path: &Path) -> Result<(), String>
    where
        Self: Sized;
}
