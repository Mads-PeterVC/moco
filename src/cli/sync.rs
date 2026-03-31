use clap::{Args, Subcommand};

use crate::config::AppConfig;
use crate::db::Store;
use crate::git;
use crate::theme::Theme;

#[derive(Args)]
pub struct SyncArgs {
    #[command(subcommand)]
    pub command: SyncCommand,
}

#[derive(Subcommand)]
pub enum SyncCommand {
    /// Pull the latest changes from the sync remote into ~/.moco/.
    Pull,
    /// Push local ~/.moco/ changes to the sync remote.
    Push,
    /// Fetch all sync-enabled projects and show their remote divergence status.
    Status,
}

pub fn run(
    args: &SyncArgs,
    store: &mut dyn Store,
    config: &AppConfig,
    theme: &Theme,
) -> anyhow::Result<()> {
    match &args.command {
        SyncCommand::Pull => run_pull(config, theme),
        SyncCommand::Push => run_push(config, theme),
        SyncCommand::Status => run_status(store, config, theme),
    }
}

fn run_pull(config: &AppConfig, theme: &Theme) -> anyhow::Result<()> {
    print_not_yet_implemented("pull", config, theme)
}

fn run_push(config: &AppConfig, theme: &Theme) -> anyhow::Result<()> {
    print_not_yet_implemented("push", config, theme)
}

fn run_status(store: &mut dyn Store, config: &AppConfig, theme: &Theme) -> anyhow::Result<()> {
    let projects = store.list_projects()?;
    let sync_projects: Vec<_> = projects.iter().filter(|p| p.git_sync_enabled).collect();

    if sync_projects.is_empty() {
        println!("No sync-enabled projects. Use `moco project set-sync --enable` to opt in.");
        return Ok(());
    }

    let ttl = config.moco_config.git_status_ttl_hours;
    println!(
        "{} {} project(s)…\n",
        theme.paint("moco sync status —", theme.accent),
        sync_projects.len(),
    );

    for project in &sync_projects {
        let name_str = theme.paint(&project.name, theme.accent);

        // Fetch from remote.
        match git::fetch(&project.path) {
            Ok(()) => {
                // Fetch succeeded — compute divergence from the now-updated tracking ref.
                let (ahead, behind) = git::local_divergence(&project.path).unwrap_or((0, 0));

                let mut updated = (*project).clone();
                updated.remote_ahead = Some(ahead);
                updated.remote_behind = Some(behind);
                updated.last_remote_check = Some(chrono::Utc::now());
                let _ = store.update_project(&updated);

                let div_str = git::format_local_divergence(ahead, behind);
                let branch_str = git::git_info(&project.path)
                    .and_then(|i| i.branch)
                    .unwrap_or_else(|| "(detached)".to_string());

                println!(
                    "  {}  ⎇ {}  {}",
                    name_str,
                    branch_str,
                    theme.paint(&div_str, if behind > 0 { theme.defer } else { theme.complete }),
                );
            }
            Err(err) => {
                println!(
                    "  {}  {} {}",
                    name_str,
                    theme.paint("⚠", theme.defer),
                    err,
                );
            }
        }
    }

    // Summary line for disabled projects (if any).
    let disabled: Vec<_> = projects.iter().filter(|p| !p.git_sync_enabled).collect();
    if !disabled.is_empty() {
        println!(
            "\n  {} project(s) excluded from sync. Use `moco project set-sync --enable --name <NAME>` to include them.",
            disabled.len(),
        );
    }

    Ok(())
}

fn print_not_yet_implemented(direction: &str, config: &AppConfig, theme: &Theme) -> anyhow::Result<()> {
    println!(
        "{} git {} for ~/.moco/ is not yet implemented.",
        theme.paint("moco sync:", theme.accent),
        direction,
    );
    println!();
    println!("Coming soon: moco will be able to {} your database and", direction);
    println!("configuration using a configured git remote.");
    println!();
    if config.moco_config.sync_remote.is_some() {
        println!(
            "{} sync_remote is already configured — you're ready for when this lands.",
            theme.paint("✓", theme.complete),
        );
    } else {
        println!(
            "To prepare, add `sync_remote = \"<url>\"` to {}.",
            theme.paint("~/.moco/config.toml", theme.accent),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AppConfig, MocoConfig};
    use crate::db::RedbStore;
    use crate::theme::{Theme, ThemeConfig};
    use tempfile::TempDir;

    fn make_config(sync_remote: Option<&str>) -> (TempDir, AppConfig) {
        let tmp = TempDir::new().unwrap();
        let moco_dir = tmp.path().to_path_buf();
        let db_path = moco_dir.join("moco.db");
        let config = AppConfig {
            moco_dir,
            db_path,
            moco_config: MocoConfig {
                open_with: None,
                theme: ThemeConfig::default(),
                sync_remote: sync_remote.map(str::to_owned),
                git_status_ttl_hours: 12,
            },
        };
        (tmp, config)
    }

    fn theme() -> Theme {
        Theme::resolve(&ThemeConfig::default())
    }

    fn make_store(tmp: &TempDir) -> RedbStore {
        RedbStore::open(&tmp.path().join("moco.db")).unwrap()
    }

    #[test]
    fn pull_prints_not_implemented_without_remote() {
        let (_tmp, config) = make_config(None);
        let args = SyncArgs { command: SyncCommand::Pull };
        let (_tmp2, mut store) = {
            let t = TempDir::new().unwrap();
            let s = make_store(&t);
            (t, s)
        };
        run(&args, &mut store, &config, &theme()).unwrap();
    }

    #[test]
    fn push_prints_not_implemented_with_remote() {
        let (_tmp, config) = make_config(Some("git@github.com:user/moco.git"));
        let args = SyncArgs { command: SyncCommand::Push };
        let (_tmp2, mut store) = {
            let t = TempDir::new().unwrap();
            let s = make_store(&t);
            (t, s)
        };
        run(&args, &mut store, &config, &theme()).unwrap();
    }

    #[test]
    fn status_shows_no_projects_message_when_all_disabled() {
        let (_tmp, config) = make_config(None);
        let args = SyncArgs { command: SyncCommand::Status };
        let (_tmp2, mut store) = {
            let t = TempDir::new().unwrap();
            let s = make_store(&t);
            (t, s)
        };
        // No projects registered — should print a message and not error.
        run(&args, &mut store, &config, &theme()).unwrap();
    }

    #[test]
    fn status_warns_on_fetch_failure_and_continues() {
        let (_tmp_cfg, config) = make_config(None);
        let tmp_db = TempDir::new().unwrap();
        let mut store = make_store(&tmp_db);

        // Register a project in a non-git directory (fetch will fail).
        let proj_dir = tmp_db.path().join("notgit");
        std::fs::create_dir_all(&proj_dir).unwrap();
        store.create_project("notgit", &proj_dir).unwrap();

        let args = SyncArgs { command: SyncCommand::Status };
        // Should succeed (warn inline) even though fetch will fail.
        run(&args, &mut store, &config, &theme()).unwrap();
    }
}

