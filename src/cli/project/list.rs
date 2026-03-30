use chrono::{DateTime, Utc};
use clap::Args;

use crate::db::Store;
use crate::models::TaskStatus;
use crate::theme::Theme;

#[derive(Args)]
pub struct ListArgs {
    /// Only show projects that have this label.
    #[arg(short, long, value_name = "LABEL")]
    pub label: Option<String>,
}

/// Format a `last_active` timestamp as `YYYY-MM-DD`, or `"never"` for the epoch sentinel.
pub fn format_last_active(dt: &DateTime<Utc>) -> String {
    if dt.timestamp() == 0 {
        "never".to_string()
    } else {
        dt.format("%d-%m-%Y").to_string()
    }
}

pub fn run(args: &ListArgs, store: &dyn Store, theme: &Theme) -> anyhow::Result<()> {
    let mut projects = store.list_projects()?;

    // Apply label filter when requested.
    if let Some(filter) = &args.label {
        let filter_lower = filter.to_lowercase();
        projects.retain(|p| p.labels.iter().any(|l| l.to_lowercase() == filter_lower));

        if projects.is_empty() {
            println!("No projects with label '{}'.", filter);
            return Ok(());
        }
    }

    if projects.is_empty() {
        println!("No projects registered. Run `moco project init <name>` to get started.");
        return Ok(());
    }

    println!("Projects ({}):\n", projects.len());

    for project in &projects {
        // ── Name + last active ───────────────────────────────────────────────
        let date = format_last_active(&project.last_active);
        println!(
            "  {} [{}]",
            theme.paint(&project.name, theme.accent),
            date,
        );

        // ── Directory ───────────────────────────────────────────────────────
        println!(
            "    {} {}",
            theme.paint("Directory:", theme.label),
            project.path.display(),
        );

        // ── Labels ──────────────────────────────────────────────────────────
        if project.labels.is_empty() {
            println!("    {} (none)", theme.paint("Labels:", theme.label));
        } else {
            let formatted: Vec<String> = project
                .labels
                .iter()
                .map(|l| theme.paint(format!("[{}]", l), theme.accent))
                .collect();
            println!(
                "    {} {}",
                theme.paint("Labels:", theme.label),
                formatted.join("  "),
            );
        }

        // ── Task counts ─────────────────────────────────────────────────────
        let tasks = store.list_tasks(Some(project.id))?;
        let open = tasks.iter().filter(|t| t.status == TaskStatus::Open).count();
        let complete = tasks.iter().filter(|t| t.status == TaskStatus::Complete).count();
        let deferred = tasks.iter().filter(|t| t.status == TaskStatus::Defer).count();

        if tasks.is_empty() {
            println!("    {} No tasks", theme.paint("Tasks:", theme.label));
        } else {
            let mut parts: Vec<String> = Vec::new();
            if open > 0 {
                parts.push(theme.paint(format!("{} open", open), theme.open));
            }
            if complete > 0 {
                parts.push(theme.paint(format!("{} complete", complete), theme.complete));
            }
            if deferred > 0 {
                parts.push(theme.paint(format!("{} deferred", deferred), theme.defer));
            }
            println!(
                "    {} {}",
                theme.paint("Tasks:", theme.label),
                parts.join("  "),
            );
        }

        println!();
    }

    Ok(())
}
