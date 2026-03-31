use std::path::Path;

use clap::Args;

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

pub fn run(args: &InfoArgs, store: &mut dyn Store, cwd: &Path, theme: &Theme) -> anyhow::Result<()> {
    let project = if let Some(name) = &args.name {
        let all = store.list_projects()?;
        all.into_iter()
            .find(|p| p.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| anyhow::anyhow!("No project named '{}'.", name))?
    } else {
        workspace::resolve(store, cwd)?
            .ok_or_else(|| anyhow::anyhow!("No project found for the current directory. Run `moco project init <name>` to register one."))?
    };

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
            let branch_str = info
                .branch
                .as_deref()
                .map(|b| theme.paint(b, theme.accent))
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

    #[test]
    fn info_errors_when_not_in_project_and_no_name() {
        let (tmp, mut store) = setup();
        let args = InfoArgs { name: None };
        let result = run(&args, &mut store, tmp.path(), &crate::theme::Theme::resolve(&Default::default()));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No project found"));
    }

    #[test]
    fn info_errors_for_unknown_name() {
        let (tmp, mut store) = setup();
        let args = InfoArgs { name: Some("nonexistent".to_string()) };
        let result = run(&args, &mut store, tmp.path(), &crate::theme::Theme::resolve(&Default::default()));
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
        run(&args, &mut store, tmp.path(), &theme).unwrap();
    }

    #[test]
    fn info_shows_project_by_cwd() {
        let (tmp, mut store) = setup();
        let project_dir = tmp.path().join("myproj");
        std::fs::create_dir_all(&project_dir).unwrap();
        store.create_project("myproj", &project_dir).unwrap();

        let args = InfoArgs { name: None };
        let theme = crate::theme::Theme::resolve(&Default::default());
        run(&args, &mut store, &project_dir, &theme).unwrap();
    }
}
