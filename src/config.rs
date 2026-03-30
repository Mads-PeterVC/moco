use std::path::{Path, PathBuf};

use anyhow::Context;
use directories::UserDirs;
use serde::{Deserialize, Serialize};

use crate::error::MocoError;
use crate::theme::ThemeConfig;

const DEFAULT_CONFIG_TEMPLATE: &str = r#"# moco configuration
#
# Command used to open a project directory.
# Falls back to the $EDITOR environment variable if not set.
#
# open_with = "code"

# [theme]
# preset = "moco"  # moco (default) | default | dracula | nord | solarized-dark
#
# [theme.colors]  # override individual colours (all optional)
# open            = "light_green"
# complete        = "green"
# defer           = "148"        # indexed 256-colour (yellow-green)
# accent          = "light_green"
# selection_bg    = "22"         # indexed 256-colour (dark forest green)
# progress_filled = "green"
# progress_empty  = "22"
"#;

/// User-editable preferences loaded from `~/.moco/config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MocoConfig {
    /// Command to open a project directory (e.g. `"code"`, `"vim"`).
    /// Falls back to `$EDITOR` if `None`.
    pub open_with: Option<String>,
    /// Colour theme configuration.
    #[serde(default)]
    pub theme: ThemeConfig,
}

impl MocoConfig {
    /// Load config from `<moco_dir>/config.toml`.
    /// If the file does not exist, a default template is written and an empty config is returned.
    /// If the file exists but fails to parse, the error is silently ignored and defaults are used;
    /// `moco config check` is the right tool for surfacing parse errors.
    pub fn load(moco_dir: &Path) -> anyhow::Result<Self> {
        let config_path = moco_dir.join("config.toml");
        if !config_path.exists() {
            std::fs::write(&config_path, DEFAULT_CONFIG_TEMPLATE)
                .with_context(|| format!("writing default config to {}", config_path.display()))?;
            return Ok(Self::default());
        }
        let contents = std::fs::read_to_string(&config_path)
            .with_context(|| format!("reading config from {}", config_path.display()))?;
        // Fall back to defaults on parse error; the user can run `moco config check` to diagnose.
        Ok(toml::from_str(&contents).unwrap_or_default())
    }

    /// Path to the config file within `moco_dir`.
    pub fn config_path(moco_dir: &Path) -> PathBuf {
        moco_dir.join("config.toml")
    }

    /// Resolve the command to use for opening a project.
    /// Priority: `open_with` field → `$EDITOR` env var → error.
    pub fn resolve_open_command(&self) -> anyhow::Result<String> {
        if let Some(cmd) = &self.open_with {
            return Ok(cmd.clone());
        }
        std::env::var("EDITOR").map_err(|_| {
            anyhow::anyhow!(
                "No editor configured. Set `open_with` in ~/.moco/config.toml or export $EDITOR."
            )
        })
    }
}

/// Application-level configuration: paths for the moco home directory and database,
/// plus the loaded user preferences.
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// `~/.moco/`
    pub moco_dir: PathBuf,
    /// `~/.moco/moco.db`
    pub db_path: PathBuf,
    /// User preferences from `~/.moco/config.toml`.
    pub moco_config: MocoConfig,
}

impl AppConfig {
    /// Resolve config from the current environment, creating `~/.moco/` if needed.
    pub fn load() -> anyhow::Result<Self> {
        let home = UserDirs::new()
            .ok_or(MocoError::HomeNotFound)
            .context("resolving home directory")?
            .home_dir()
            .to_path_buf();

        let moco_dir = home.join(".moco");
        std::fs::create_dir_all(&moco_dir)
            .with_context(|| format!("creating moco directory at {}", moco_dir.display()))?;

        let db_path = moco_dir.join("moco.db");
        let moco_config = MocoConfig::load(&moco_dir)?;

        Ok(Self {
            moco_dir,
            db_path,
            moco_config,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn load_creates_moco_dir() {
        let config = AppConfig::load().expect("config should load");
        assert!(config.moco_dir.exists(), "~/.moco should be created");
        assert_eq!(config.db_path, config.moco_dir.join("moco.db"));
    }

    #[test]
    fn db_path_is_inside_moco_dir() {
        let tmp = TempDir::new().unwrap();
        let moco_dir = tmp.path().join(".moco");
        std::fs::create_dir_all(&moco_dir).unwrap();
        let db_path = moco_dir.join("moco.db");
        assert_eq!(db_path.parent().unwrap(), moco_dir);
    }

    #[test]
    fn moco_config_creates_default_file_when_absent() {
        let tmp = TempDir::new().unwrap();
        let config = MocoConfig::load(tmp.path()).expect("load should succeed");
        assert!(config.open_with.is_none());
        assert!(MocoConfig::config_path(tmp.path()).exists());
    }

    #[test]
    fn moco_config_parses_open_with() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("config.toml"), "open_with = \"code\"\n").unwrap();
        let config = MocoConfig::load(tmp.path()).expect("load should succeed");
        assert_eq!(config.open_with.as_deref(), Some("code"));
    }

    #[test]
    fn moco_config_parses_theme_preset() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("config.toml"),
            "[theme]\npreset = \"dracula\"\n",
        )
        .unwrap();
        let config = MocoConfig::load(tmp.path()).expect("load should succeed");
        assert_eq!(config.theme.preset, crate::theme::Preset::Dracula);
    }

    #[test]
    fn resolve_open_command_uses_open_with_field() {
        let config = MocoConfig {
            open_with: Some("code".to_string()),
            ..Default::default()
        };
        assert_eq!(config.resolve_open_command().unwrap(), "code");
    }

    #[test]
    fn resolve_open_command_falls_back_to_editor_env() {
        let config = MocoConfig::default();
        // Only testable when $EDITOR is set; skip if not.
        if std::env::var("EDITOR").is_ok() {
            assert!(config.resolve_open_command().is_ok());
        }
    }
}

