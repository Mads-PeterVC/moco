use clap::{Args, Subcommand};

use crate::config::AppConfig;
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
    /// Show the current sync configuration and status.
    Status,
}

pub fn run(args: &SyncArgs, config: &AppConfig, theme: &Theme) -> anyhow::Result<()> {
    match &args.command {
        SyncCommand::Pull => run_pull(config, theme),
        SyncCommand::Push => run_push(config, theme),
        SyncCommand::Status => run_status(config, theme),
    }
}

fn run_pull(config: &AppConfig, theme: &Theme) -> anyhow::Result<()> {
    print_not_yet_implemented("pull", config, theme)
}

fn run_push(config: &AppConfig, theme: &Theme) -> anyhow::Result<()> {
    print_not_yet_implemented("push", config, theme)
}

fn run_status(config: &AppConfig, theme: &Theme) -> anyhow::Result<()> {
    println!("{}", theme.paint("moco sync status", theme.accent));
    println!("{}", "─".repeat(40));
    println!(
        "  {:>12}: {}",
        theme.paint("moco dir", theme.label),
        config.moco_dir.display(),
    );
    match &config.moco_config.sync_remote {
        Some(remote) => {
            println!(
                "  {:>12}: {}",
                theme.paint("sync remote", theme.label),
                remote,
            );
        }
        None => {
            println!(
                "  {:>12}: {}",
                theme.paint("sync remote", theme.label),
                "(not configured)",
            );
            println!();
            println!(
                "To enable sync, add the following to {}:",
                theme.paint("~/.moco/config.toml", theme.accent),
            );
            println!();
            println!("    sync_remote = \"git@github.com:youruser/moco-sync.git\"");
        }
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
            },
        };
        (tmp, config)
    }

    fn theme() -> Theme {
        Theme::resolve(&ThemeConfig::default())
    }

    #[test]
    fn pull_prints_not_implemented_without_remote() {
        let (_tmp, config) = make_config(None);
        let args = SyncArgs { command: SyncCommand::Pull };
        run(&args, &config, &theme()).unwrap();
    }

    #[test]
    fn push_prints_not_implemented_with_remote() {
        let (_tmp, config) = make_config(Some("git@github.com:user/moco.git"));
        let args = SyncArgs { command: SyncCommand::Push };
        run(&args, &config, &theme()).unwrap();
    }

    #[test]
    fn status_shows_unconfigured_when_no_remote() {
        let (_tmp, config) = make_config(None);
        let args = SyncArgs { command: SyncCommand::Status };
        run(&args, &config, &theme()).unwrap();
    }

    #[test]
    fn status_shows_remote_when_configured() {
        let (_tmp, config) = make_config(Some("https://github.com/user/moco-sync.git"));
        let args = SyncArgs { command: SyncCommand::Status };
        run(&args, &config, &theme()).unwrap();
    }
}
