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
}

/// Abstraction over a git backend so the underlying implementation (currently
/// `gix`) can be swapped without touching the rest of the codebase.
pub trait GitBackend {
    /// Attempt to discover a git repository at or above `path` and return its
    /// metadata.  Returns `None` when `path` is not inside a git repository or
    /// when discovery fails for any reason (errors are silently ignored so that
    /// non-git projects are handled gracefully).
    fn discover(path: &Path) -> Option<GitInfo>
    where
        Self: Sized;
}
