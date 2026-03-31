use std::io::{self, Write};
use std::path::PathBuf;

use clap::Args;

use crate::db::Store;
use crate::models::TaskStatus;
use crate::theme::Theme;
use crate::tui::{
    self,
    project_browser::{ProjectBrowser, ProjectBrowserOutcome},
};

/// Build browser groups from a flat summary list, respecting category order.
fn build_groups(
    summaries: &[(crate::models::Project, usize)],
    store: &impl Store,
) -> Vec<(String, Vec<(crate::models::Project, usize)>)> {
    let categories = store.list_categories().unwrap_or_default();
    let mut groups: Vec<(String, Vec<_>)> = Vec::new();
    for cat in &categories {
        let ps: Vec<_> = summaries
            .iter()
            .filter(|(p, _)| p.category.as_deref() == Some(cat.name.as_str()))
            .cloned()
            .collect();
        if !ps.is_empty() {
            groups.push((cat.name.clone(), ps));
        }
    }
    let uncategorized: Vec<_> = summaries
        .iter()
        .filter(|(p, _)| p.category.is_none())
        .cloned()
        .collect();
    if !uncategorized.is_empty() {
        groups.push(("Uncategorized".to_string(), uncategorized));
    }
    groups
}

#[derive(Args)]
pub struct MoveArgs {
    /// Directly select the project at this path (bypasses TUI — used in tests).
    #[arg(long, value_name = "PATH", hide = true)]
    pub project_path: Option<PathBuf>,

    /// New path to assign (bypasses interactive prompt — used in tests).
    #[arg(long, value_name = "PATH", hide = true)]
    pub new_path: Option<PathBuf>,

    /// Skip confirmation prompt (used in tests).
    #[arg(long, hide = true)]
    pub yes: bool,
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

    // Build grouped view for the browser.
    let groups = build_groups(&summaries, store);

    // Flatten groups into a stable ordered list for index resolution.
    let flat_summaries: Vec<(crate::models::Project, usize)> = groups
        .iter()
        .flat_map(|(_, ps)| ps.iter().cloned())
        .collect();

    // Resolve which project to move — TUI browser or direct path.
    let mut project = if let Some(path) = &args.project_path {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
        flat_summaries
            .into_iter()
            .find(|(p, _)| p.path == canonical)
            .map(|(p, _)| p)
            .ok_or_else(|| anyhow::anyhow!("No project registered at {}", path.display()))?
    } else {
        let mut guard = tui::enter()?;
        let mut browser = ProjectBrowser::new(flat_summaries.len());

        let selected_idx = loop {
            guard.terminal.draw(|frame| {
                browser.render(frame, frame.area(), &groups, theme);
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
        flat_summaries.into_iter().nth(selected_idx).map(|(p, _)| p).expect("index in bounds")
    };

    let old_path = project.path.clone();

    // Determine the new path — hidden arg (tests) or interactive prompt.
    let new_path = if let Some(p) = &args.new_path {
        p.canonicalize()
            .map_err(|_| anyhow::anyhow!("Path '{}' does not exist.", p.display()))?
    } else {
        let cwd = std::env::current_dir()?;
        println!(
            "Current path: {}",
            theme.paint(old_path.display(), theme.accent)
        );
        print!(
            "New path [{}]: ",
            theme.paint(cwd.display(), theme.accent)
        );
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let trimmed = input.trim();

        if trimmed.is_empty() {
            cwd
        } else {
            let p = PathBuf::from(trimmed);
            p.canonicalize()
                .map_err(|_| anyhow::anyhow!("Path '{}' does not exist.", p.display()))?
        }
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

    // Confirm the move unless --yes was passed.
    if !args.yes {
        print!(
            "Switch {} from {} → {}? [y/N] ",
            theme.paint(format!("'{}'", project.name), theme.accent),
            theme.paint(old_path.display(), theme.open),
            theme.paint(new_path.display(), theme.accent),
        );
        io::stdout().flush()?;

        let mut answer = String::new();
        io::stdin().read_line(&mut answer)?;
        if !answer.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(());
        }
    }

    project.path = new_path.clone();
    store.relocate_project(&old_path, &project)?;

    println!(
        "Moved project {} from {} to {}.",
        theme.paint(format!("'{}'", project.name), theme.accent),
        theme.paint(old_path.display(), theme.open),
        theme.paint(new_path.display(), theme.accent),
    );

    Ok(())
}
