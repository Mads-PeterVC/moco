use std::io::{self, Write};

use clap::Args;

use crate::db::Store;
use crate::models::TaskStatus;
use crate::theme::Theme;
use crate::tui::{self, project_browser::{ProjectBrowser, ProjectBrowserOutcome}};

#[derive(Args)]
pub struct DeleteArgs {
    /// Skip the confirmation prompt and delete immediately.
    #[arg(long, short = 'y')]
    pub yes: bool,

    /// Directly select the project at this path, bypassing the TUI browser.
    /// Combined with `--yes`, this makes the full flow testable.
    #[arg(long, value_name = "PATH", hide = true)]
    pub project_path: Option<std::path::PathBuf>,
}

pub fn run(args: &DeleteArgs, store: &mut impl Store, theme: &Theme) -> anyhow::Result<()> {
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

    // Resolve the target project — TUI browser or direct path.
    let project = if let Some(path) = &args.project_path {
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

    // Count tasks so the user knows what they're deleting.
    let task_count = store.list_tasks(Some(project.id))?.len();
    let task_label = match task_count {
        0 => "no tasks".to_string(),
        1 => "1 task".to_string(),
        n => format!("{n} tasks"),
    };

    println!(
        "About to delete project '{}' at {} ({}).",
        project.name,
        project.path.display(),
        task_label
    );

    if !args.yes {
        print!("Are you sure? [y/N]: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if input.trim().to_ascii_lowercase() != "y" {
            println!("Cancelled.");
            return Ok(());
        }
    }

    store.delete_project(&project)?;
    println!("Deleted project '{}'.", project.name);

    Ok(())
}