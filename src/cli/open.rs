use clap::Args;

use crate::config::AppConfig;
use crate::db::Store;
use crate::models::TaskStatus;
use crate::theme::Theme;
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

pub fn run(args: &OpenArgs, store: &mut impl Store, config: &AppConfig, theme: &Theme) -> anyhow::Result<()> {
    let projects = store.list_projects()?;

    if projects.is_empty() {
        anyhow::bail!(
            "No projects registered. Run `moco project init <name>` inside a project directory first."
        );
    }

    // Pre-compute (project, open_task_count) pairs for the browser.
    let summaries: Vec<(crate::models::Project, usize)> = projects
        .into_iter()
        .map(|p| {
            let open = store
                .list_tasks(Some(p.id))
                .unwrap_or_default()
                .into_iter()
                .filter(|t| t.status == TaskStatus::Open)
                .count();
            (p, open)
        })
        .collect();

    // If --project-path is given, skip the TUI and resolve directly.
    let selected_project = if let Some(path) = &args.project_path {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
        summaries
            .into_iter()
            .find(|(p, _)| p.path == canonical)
            .map(|(p, _)| p)
            .ok_or_else(|| anyhow::anyhow!("No project registered at {}", path.display()))?
    } else {
        // Launch the TUI project browser.
        let mut guard = tui::enter()?;
        let mut browser = ProjectBrowser::new(summaries.len());

        let selected_idx = loop {
            guard.terminal.draw(|frame| {
                browser.render(frame, frame.area(), &summaries, theme);
            })?;

            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
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

        drop(guard);
        summaries.into_iter().nth(selected_idx).map(|(p, _)| p).expect("index in bounds")
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

        store.touch_project(selected_project.id)?;
    }

    Ok(())
}
