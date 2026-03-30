use clap::Args;

use crate::config::AppConfig;
use crate::db::Store;
use crate::tui::{self, project_browser::{ProjectBrowser, ProjectBrowserOutcome}};

#[derive(Args)]
pub struct OpenArgs {
    /// Print the command that would be run instead of launching the editor.
    /// Useful for scripting and testing.
    #[arg(long)]
    pub dry_run: bool,

    /// Directly open the project at this path, bypassing the TUI browser.
    /// Combined with `--dry-run`, this makes the full flow testable.
    #[arg(long, value_name = "PATH", hide = true)]
    pub project_path: Option<std::path::PathBuf>,
}

pub fn run(args: &OpenArgs, store: &impl Store, config: &AppConfig) -> anyhow::Result<()> {
    let projects = store.list_projects()?;

    if projects.is_empty() {
        anyhow::bail!(
            "No projects registered. Run `moco init <name>` inside a project directory first."
        );
    }

    // If --project-path is given, skip the TUI and resolve directly.
    let selected_project = if let Some(path) = &args.project_path {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
        projects
            .into_iter()
            .find(|p| p.path == canonical)
            .ok_or_else(|| anyhow::anyhow!("No project registered at {}", path.display()))?
    } else {
        // Launch the TUI project browser.
        let mut guard = tui::enter()?;
        let mut browser = ProjectBrowser::new(projects.len());

        let selected_idx = loop {
            let proj_ref = &projects;
            guard.terminal.draw(|frame| {
                browser.render(frame, frame.area(), proj_ref);
            })?;

            if let crossterm::event::Event::Key(key) =
                crossterm::event::read()?
            {
                match browser.handle_key(key) {
                    ProjectBrowserOutcome::Selected(i) => break i,
                    ProjectBrowserOutcome::Cancelled => {
                        drop(guard);
                        println!("Cancelled.");
                        return Ok(());
                    }
                    ProjectBrowserOutcome::Continue => {}
                }
            }
        };

        // Drop the TUI guard before printing to stdout.
        drop(guard);
        projects.into_iter().nth(selected_idx).expect("index in bounds")
    };

    let open_cmd = config.moco_config.resolve_open_command()?;
    let project_path = &selected_project.path;

    if args.dry_run {
        println!("{open_cmd} {}", project_path.display());
    } else {
        println!(
            "Opening '{}' with {}…",
            selected_project.name, open_cmd
        );
        std::process::Command::new(&open_cmd)
            .arg(project_path)
            .spawn()
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to launch `{open_cmd}`: {e}. \
                     Check `open_with` in ~/.moco/config.toml or set $EDITOR."
                )
            })?;
    }

    Ok(())
}
