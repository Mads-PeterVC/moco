use clap::{Args, Subcommand};

use crate::config::{AppConfig, MocoConfig};
use crate::theme::Theme;

#[derive(Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommand,
}

#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Check the configuration file for errors.
    Check,
}

pub fn run(args: &ConfigArgs, config: &AppConfig, theme: &Theme) -> anyhow::Result<()> {
    match &args.command {
        ConfigCommand::Check => run_check(config, theme),
    }
}

fn run_check(config: &AppConfig, theme: &Theme) -> anyhow::Result<()> {
    let config_path = MocoConfig::config_path(&config.moco_dir);
    let mut issues: Vec<String> = Vec::new();

    // Validate the config file can be parsed.
    if config_path.exists() {
        let contents = std::fs::read_to_string(&config_path)?;
        match toml::from_str::<MocoConfig>(&contents) {
            Err(e) => {
                issues.push(format!("Parse error in config.toml: {e}"));
            }
            Ok(parsed) => {
                // If open_with is set, verify the command exists on PATH.
                if let Some(cmd) = &parsed.open_with {
                    if which(cmd).is_none() {
                        issues.push(format!(
                            "`open_with = \"{cmd}\"` — command not found on PATH."
                        ));
                    }
                }
            }
        }
    } else {
        // File absent is fine (defaults apply), but let the user know.
        println!(
            "Config file not found at {}. Using defaults.",
            theme.paint(config_path.display(), theme.accent),
        );
        return Ok(());
    }

    if issues.is_empty() {
        println!(
            "{} ({})",
            theme.paint("Configuration OK", theme.complete),
            theme.paint(config_path.display(), theme.accent),
        );
    } else {
        for issue in &issues {
            eprintln!("  {} {issue}", theme.paint("✗", theme.defer));
        }
        anyhow::bail!("Configuration has {} issue(s).", issues.len());
    }

    Ok(())
}

/// Resolve a command name to its full path by searching `$PATH`, returning `None` if not found.
fn which(cmd: &str) -> Option<std::path::PathBuf> {
    std::env::var_os("PATH").and_then(|path_var| {
        std::env::split_paths(&path_var).find_map(|dir| {
            let candidate = dir.join(cmd);
            if candidate.is_file() { Some(candidate) } else { None }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ThemeConfig;
    use tempfile::TempDir;

    fn make_config(tmp: &TempDir) -> AppConfig {
        let moco_dir = tmp.path().join(".moco");
        std::fs::create_dir_all(&moco_dir).unwrap();
        AppConfig {
            moco_dir: moco_dir.clone(),
            db_path: moco_dir.join("moco.db"),
            moco_config: MocoConfig::default(),
        }
    }

    fn default_theme() -> Theme {
        Theme::resolve(&ThemeConfig::default())
    }

    #[test]
    fn check_passes_with_absent_config() {
        let tmp = TempDir::new().unwrap();
        let config = make_config(&tmp);
        // No config.toml written — should not error.
        run_check(&config, &default_theme()).expect("check should pass with absent config");
    }

    #[test]
    fn check_passes_with_valid_empty_config() {
        let tmp = TempDir::new().unwrap();
        let config = make_config(&tmp);
        std::fs::write(MocoConfig::config_path(&config.moco_dir), "").unwrap();
        run_check(&config, &default_theme()).expect("check should pass with empty config");
    }

    #[test]
    fn check_fails_with_malformed_toml() {
        let tmp = TempDir::new().unwrap();
        let config = make_config(&tmp);
        std::fs::write(
            MocoConfig::config_path(&config.moco_dir),
            "open_with = [invalid",
        )
        .unwrap();
        assert!(run_check(&config, &default_theme()).is_err());
    }

    #[test]
    fn check_fails_when_open_with_command_not_on_path() {
        let tmp = TempDir::new().unwrap();
        let config = make_config(&tmp);
        std::fs::write(
            MocoConfig::config_path(&config.moco_dir),
            "open_with = \"__moco_nonexistent_cmd__\"\n",
        )
        .unwrap();
        assert!(run_check(&config, &default_theme()).is_err());
    }

    #[test]
    fn which_finds_common_commands() {
        // Most systems have `sh`.
        assert!(which("sh").is_some());
    }

    #[test]
    fn which_returns_none_for_unknown_command() {
        assert!(which("__moco_nonexistent_cmd__").is_none());
    }
}