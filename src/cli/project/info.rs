use std::path::Path;

use clap::Args;

use crate::config::AppConfig;
use crate::db::Store;
use crate::git;
use crate::models::TaskStatus;
use crate::theme::Theme;
use crate::workspace;

use super::list::format_last_active;

#[derive(Args)]
pub struct InfoArgs {
    /// Show info for this named project rather than the project for the current directory.
    #[arg(short, long, value_name = "NAME")]
    pub name: Option<String>,
}

pub fn run(args: &InfoArgs, store: &mut dyn Store, cwd: &Path, theme: &Theme, config: &AppConfig) -> anyhow::Result<()> {
    let project = if let Some(name) = &args.name {
        let all = store.list_projects()?;
        all.into_iter()
            .find(|p| p.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| anyhow::anyhow!("No project named '{}'.", name))?
    } else {
        workspace::resolve(store, cwd)?
            .ok_or_else(|| anyhow::anyhow!("No project found for the current directory. Run `moco project init <name>` to register one."))?
    };

    let ttl = config.moco_config.git_status_ttl_hours;
    let label = |s: &str| theme.paint(format!("{:>12}:", s), theme.label);

    // ── Identity ─────────────────────────────────────────────────────────────
    println!(
        "\n{} {}",
        label("Project"),
        theme.paint(&project.name, theme.accent),
    );
    println!("{} {}", label("ID"), project.id);

    // ── Location ─────────────────────────────────────────────────────────────
    println!("{} {}", label("Directory"), project.path.display());

    // ── Timestamps ───────────────────────────────────────────────────────────
    println!(
        "{} {}",
        label("Created"),
        project.created_at.format("%Y-%m-%d %H:%M UTC"),
    );
    println!(
        "{} {}",
        label("Last active"),
        format_last_active(&project.last_active),
    );

    // ── Organisation ─────────────────────────────────────────────────────────
    if let Some(cat) = &project.category {
        println!("{} {}", label("Category"), cat);
    } else {
        println!("{} (none)", label("Category"));
    }

    if project.labels.is_empty() {
        println!("{} (none)", label("Labels"));
    } else {
        let formatted: Vec<String> = project
            .labels
            .iter()
            .map(|l| theme.paint(format!("[{}]", l), theme.accent))
            .collect();
        println!("{} {}", label("Labels"), formatted.join("  "));
    }

    // ── Task summary ─────────────────────────────────────────────────────────
    let tasks = store.list_tasks(Some(project.id))?;
    let open = tasks.iter().filter(|t| t.status == TaskStatus::Open).count();
    let complete = tasks.iter().filter(|t| t.status == TaskStatus::Complete).count();
    let deferred = tasks.iter().filter(|t| t.status == TaskStatus::Defer).count();

    if tasks.is_empty() {
        println!("{} (none)", label("Tasks"));
    } else {
        let mut parts: Vec<String> = Vec::new();
        if open > 0 {
            parts.push(theme.paint(format!("{} open", open), theme.open));
        }
        if complete > 0 {
            parts.push(theme.paint(format!("{} complete", complete), theme.complete));
        }
        if deferred > 0 {
            parts.push(theme.paint(format!("{} deferred", deferred), theme.defer));
        }
        println!("{} {}", label("Tasks"), parts.join("  "));
    }

    // ── Git ──────────────────────────────────────────────────────────────────
    let live_info = git::git_info(&project.path);
    match &live_info {
        Some(info) => {
            let dirty_suffix = if info.dirty == Some(true) { "*" } else { "" };
            let branch_str = info
                .branch
                .as_deref()
                .map(|b| theme.paint(format!("{}{}", b, dirty_suffix), theme.accent))
                .unwrap_or_else(|| "(detached HEAD)".to_string());
            println!("{} {}", label("Git branch"), branch_str);

            if let Some(url) = &info.remote_url {
                let remote_name = info.remote_name.as_deref().unwrap_or("remote");
                println!(
                    "{} {} ({})",
                    label("Git remote"),
                    url,
                    remote_name,
                );
            } else if let Some(cached_url) = &project.git_remote {
                println!(
                    "{} {} (cached)",
                    label("Git remote"),
                    cached_url,
                );
            } else {
                println!("{} (none configured)", label("Git remote"));
            }

            // Local divergence (no network).
            if let (Some(ahead), Some(behind)) = (info.local_ahead, info.local_behind) {
                let div_str = git::format_local_divergence(ahead, behind);
                let color = if behind > 0 { theme.defer } else { theme.complete };
                println!(
                    "{} {}",
                    label("Local"),
                    theme.paint(&div_str, color),
                );
            }

            // Opportunistically cache the remote if it has changed.
            if let Some(url) = &info.remote_url {
                if project.git_remote.as_deref() != Some(url.as_str()) {
                    let mut updated = project.clone();
                    updated.git_remote = Some(url.clone());
                    let _ = store.update_project(&updated);
                }
            }
        }
        None => {
            // No live git repo — show cached remote if we have it.
            if let Some(cached_url) = &project.git_remote {
                println!("{} (no local repo)", label("Git branch"));
                println!(
                    "{} {} (cached)",
                    label("Git remote"),
                    cached_url,
                );
            }
            // If no live repo and no cache, show nothing — the project simply isn't git-tracked.
        }
    }

    // ── Cached remote divergence (from last moco sync status) ────────────────
    if project.git_sync_enabled {
        match git::format_cached_divergence(
            project.remote_ahead,
            project.remote_behind,
            project.last_remote_check,
            ttl,
        ) {
            Some(cached) => {
                let color = if project.remote_behind.unwrap_or(0) > 0 {
                    theme.defer
                } else {
                    theme.complete
                };
                println!(
                    "{} {}",
                    label("Remote"),
                    theme.paint(&cached, color),
                );
            }
            None if project.last_remote_check.is_some() => {
                println!(
                    "{} {} — run `moco sync status` to refresh",
                    label("Remote"),
                    theme.paint("(stale)", theme.defer),
                );
            }
            None => {}
        }
    } else {
        println!("{} (sync disabled)", label("Remote"));
    }

    println!();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::RedbStore;
    use tempfile::TempDir;

    fn setup() -> (TempDir, RedbStore) {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("moco.db");
        let store = RedbStore::open(&db_path).unwrap();
        (dir, store)
    }

    fn default_config() -> AppConfig {
        AppConfig {
            moco_dir: std::path::PathBuf::from("/tmp"),
            db_path: std::path::PathBuf::from("/tmp/moco.db"),
            moco_config: Default::default(),
        }
    }

    #[test]
    fn info_errors_when_not_in_project_and_no_name() {
        let (tmp, mut store) = setup();
        let args = InfoArgs { name: None };
        let result = run(&args, &mut store, tmp.path(), &crate::theme::Theme::resolve(&Default::default()), &default_config());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No project found"));
    }

    #[test]
    fn info_errors_for_unknown_name() {
        let (tmp, mut store) = setup();
        let args = InfoArgs { name: Some("nonexistent".to_string()) };
        let result = run(&args, &mut store, tmp.path(), &crate::theme::Theme::resolve(&Default::default()), &default_config());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No project named"));
    }

    #[test]
    fn info_shows_project_by_name() {
        let (tmp, mut store) = setup();
        let project_dir = tmp.path().join("myproj");
        std::fs::create_dir_all(&project_dir).unwrap();
        store.create_project("myproj", &project_dir).unwrap();

        let args = InfoArgs { name: Some("myproj".to_string()) };
        let theme = crate::theme::Theme::resolve(&Default::default());
        // Should not error.
        run(&args, &mut store, tmp.path(), &theme, &default_config()).unwrap();
    }

    #[test]
    fn info_shows_project_by_cwd() {
        let (tmp, mut store) = setup();
        let project_dir = tmp.path().join("myproj");
        std::fs::create_dir_all(&project_dir).unwrap();
        store.create_project("myproj", &project_dir).unwrap();

        let args = InfoArgs { name: None };
        let theme = crate::theme::Theme::resolve(&Default::default());
        run(&args, &mut store, &project_dir, &theme, &default_config()).unwrap();
    }
}
