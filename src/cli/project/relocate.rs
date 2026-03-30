use std::path::PathBuf;

use clap::Args;

use crate::db::Store;
use crate::models::TaskStatus;
use crate::theme::Theme;
use crate::tui::{
    self,
    project_browser::{ProjectBrowser, ProjectBrowserOutcome},
};

#[derive(Args)]
pub struct MoveArgs {
    /// New directory path for the project. Defaults to the current directory.
    pub path: Option<PathBuf>,

    /// Directly select the project at this path (bypasses TUI — used in tests).
    #[arg(long, value_name = "PATH", hide = true)]
    pub project_path: Option<PathBuf>,
}

pub fn run(args: &MoveArgs, store: &mut impl Store, theme: &Theme) -> anyhow::Result<()> {
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

    // Resolve which project to move — TUI browser or direct path.
    let mut project = if let Some(path) = &args.project_path {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
        summaries
            .into_iter()
            .find(|(p, _)| p.path == canonical)
            .map(|(p, _)| p)
            .ok_or_else(|| anyhow::anyhow!("No project registered at {}", path.display()))?
    } else {
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

    // Determine the new path — explicit arg or current directory.
    let new_path = match &args.path {
        Some(p) => p
            .canonicalize()
            .map_err(|_| anyhow::anyhow!("Path '{}' does not exist.", p.display()))?,
        None => std::env::current_dir()?,
    };

    // Guard: reject if another project is already registered at the new path.
    if let Some(existing) = store.get_project_by_path(&new_path)? {
        if existing.id != project.id {
            anyhow::bail!(
                "Project '{}' is already registered at {}.",
                existing.name,
                new_path.display()
            );
        }
        // Same project, same path — nothing to do.
        println!(
            "Project {} is already registered at {}.",
            theme.paint(format!("'{}'", project.name), theme.accent),
            theme.paint(new_path.display(), theme.accent),
        );
        return Ok(());
    }

    let old_path = project.path.clone();
    project.path = new_path.clone();
    store.update_project(&project)?;

    println!(
        "Moved project {} from {} to {}.",
        theme.paint(format!("'{}'", project.name), theme.accent),
        theme.paint(old_path.display(), theme.open),
        theme.paint(new_path.display(), theme.accent),
    );

    Ok(())
}
