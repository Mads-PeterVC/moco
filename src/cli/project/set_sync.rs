use std::path::Path;

use clap::Args;

use crate::db::Store;
use crate::theme::Theme;
use crate::workspace;

#[derive(Args)]
#[command(group = clap::ArgGroup::new("action").required(true))]
pub struct SetSyncArgs {
    /// Enable `moco sync status` network checks for this project.
    #[arg(long, group = "action")]
    pub enable: bool,
    /// Disable `moco sync status` network checks for this project.
    #[arg(long, group = "action")]
    pub disable: bool,
    /// Target project by name instead of resolving from the current directory.
    #[arg(short, long, value_name = "NAME")]
    pub name: Option<String>,
}

pub fn run(args: &SetSyncArgs, store: &mut dyn Store, cwd: &Path, theme: &Theme) -> anyhow::Result<()> {
    let mut project = if let Some(name) = &args.name {
        let all = store.list_projects()?;
        all.into_iter()
            .find(|p| p.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| anyhow::anyhow!("No project named '{}'.", name))?
    } else {
        workspace::resolve(store, cwd)?
            .ok_or_else(|| anyhow::anyhow!("No project found for the current directory. Use --name to specify one."))?
    };

    let enabled = args.enable;
    project.git_sync_enabled = enabled;
    store.update_project(&project)?;

    let status = if enabled {
        theme.paint("enabled", theme.complete)
    } else {
        theme.paint("disabled", theme.defer)
    };

    println!(
        "Git sync {} for project '{}'.",
        status,
        theme.paint(&project.name, theme.accent),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::RedbStore;
    use tempfile::TempDir;

    fn setup() -> (TempDir, RedbStore) {
        let dir = TempDir::new().unwrap();
        let store = RedbStore::open(&dir.path().join("moco.db")).unwrap();
        (dir, store)
    }

    fn theme() -> crate::theme::Theme {
        crate::theme::Theme::resolve(&Default::default())
    }

    #[test]
    fn disable_sync_for_project_by_cwd() {
        let (tmp, mut store) = setup();
        let proj_dir = tmp.path().join("myproj");
        std::fs::create_dir_all(&proj_dir).unwrap();
        store.create_project("myproj", &proj_dir).unwrap();

        let args = SetSyncArgs { enable: false, disable: true, name: None };
        run(&args, &mut store, &proj_dir, &theme()).unwrap();

        let p = store.get_project_by_path(&proj_dir).unwrap().unwrap();
        assert!(!p.git_sync_enabled);
    }

    #[test]
    fn enable_sync_for_project_by_name() {
        let (tmp, mut store) = setup();
        let proj_dir = tmp.path().join("myproj");
        std::fs::create_dir_all(&proj_dir).unwrap();
        let mut p = store.create_project("myproj", &proj_dir).unwrap();
        p.git_sync_enabled = false;
        store.update_project(&p).unwrap();

        let args = SetSyncArgs { enable: true, disable: false, name: Some("myproj".to_string()) };
        run(&args, &mut store, tmp.path(), &theme()).unwrap();

        let updated = store.get_project_by_path(&proj_dir).unwrap().unwrap();
        assert!(updated.git_sync_enabled);
    }

    #[test]
    fn errors_for_unknown_project_name() {
        let (tmp, mut store) = setup();
        let args = SetSyncArgs { enable: true, disable: false, name: Some("ghost".to_string()) };
        let result = run(&args, &mut store, tmp.path(), &theme());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No project named"));
    }

    #[test]
    fn errors_when_not_in_project_and_no_name() {
        let (tmp, mut store) = setup();
        let args = SetSyncArgs { enable: true, disable: false, name: None };
        let result = run(&args, &mut store, tmp.path(), &theme());
        assert!(result.is_err());
    }
}
