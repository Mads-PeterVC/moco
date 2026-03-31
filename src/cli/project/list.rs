use chrono::{DateTime, Utc};
use clap::Args;

use crate::config::AppConfig;
use crate::db::Store;
use crate::git;
use crate::models::TaskStatus;
use crate::theme::Theme;

#[derive(Args)]
pub struct ListArgs {
    /// Only show projects that have this label.
    #[arg(short, long, value_name = "LABEL")]
    pub label: Option<String>,
    /// Only show projects in this category.
    #[arg(short, long, value_name = "CATEGORY")]
    pub category: Option<String>,
}

/// Format a `last_active` timestamp as `YYYY-MM-DD`, or `"never"` for the epoch sentinel.
pub fn format_last_active(dt: &DateTime<Utc>) -> String {
    if dt.timestamp() == 0 {
        "never".to_string()
    } else {
        dt.format("%d-%m-%Y").to_string()
    }
}

pub fn run(args: &ListArgs, store: &dyn Store, theme: &Theme, config: &AppConfig) -> anyhow::Result<()> {
    // Validate --category filter early so we can error even when no projects exist.
    if let Some(filter) = &args.category {
        if store.get_category(filter)?.is_none() {
            anyhow::bail!("Category '{}' not found.", filter);
        }
    }

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

    // Apply category filter or group by category.
    if let Some(filter) = &args.category {
        // Category existence already validated above.
        projects.retain(|p| p.category.as_deref().map(|c| c.eq_ignore_ascii_case(filter)).unwrap_or(false));
        if projects.is_empty() {
            println!("No projects in category '{}'.", filter);
            return Ok(());
        }
        print_project_group(filter, &projects, store, theme, config)?;
    } else {
        // Group all projects by category (categories in order, Uncategorized last).
        let categories = store.list_categories()?;

        // Build groups: (header, projects_in_group).
        let mut groups: Vec<(String, Vec<_>)> = Vec::new();
        for cat in &categories {
            let cat_projects: Vec<_> = projects
                .iter()
                .filter(|p| p.category.as_deref() == Some(cat.name.as_str()))
                .cloned()
                .collect();
            if !cat_projects.is_empty() {
                groups.push((cat.name.clone(), cat_projects));
            }
        }
        let uncategorized: Vec<_> = projects
            .iter()
            .filter(|p| p.category.is_none())
            .cloned()
            .collect();
        if !uncategorized.is_empty() {
            groups.push(("Uncategorized".to_string(), uncategorized));
        }

        let total = projects.len();
        println!("Projects ({total}):\n");

        for (header, group_projects) in &groups {
            // Print the category header.
            println!(
                "{}",
                theme.paint(format!("── {header} ──"), theme.label),
            );
            print_project_group_items(group_projects, store, theme, config)?;
        }
    }

    Ok(())
}

/// Print projects under a single named group (used for --category filter).
fn print_project_group(
    header: &str,
    projects: &[crate::models::Project],
    store: &dyn Store,
    theme: &Theme,
    config: &AppConfig,
) -> anyhow::Result<()> {
    println!("Projects in category '{}' ({}):\n", header, projects.len());
    print_project_group_items(projects, store, theme, config)
}

/// Print a list of project entries without a group header.
fn print_project_group_items(
    projects: &[crate::models::Project],
    store: &dyn Store,
    theme: &Theme,
    config: &AppConfig,
) -> anyhow::Result<()> {
    let ttl = config.moco_config.git_status_ttl_hours;

    for project in projects {
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

        // ── Git info (branch + remote URL) ───────────────────────────────────
        // Try a live query first; fall back to the cached remote from the DB.
        let live_info = git::git_info(&project.path);
        let git_line = if let Some(ref info) = live_info {
            let mut s = git::format_git_info(info);
            if !s.is_empty() {
                // Append dirty marker when tracked files have uncommitted changes.
                if info.dirty == Some(true) {
                    s.push_str("  *");
                }
                Some(s)
            } else {
                None
            }
        } else {
            project.git_remote.as_deref().map(|url| format!("↑ {} (cached)", url))
        };
        if let Some(line) = git_line {
            println!(
                "    {} {}",
                theme.paint("Git:", theme.label),
                line,
            );
        }

        // ── Local divergence (free, no network) ──────────────────────────────
        if let Some(ref info) = live_info {
            if let (Some(ahead), Some(behind)) = (info.local_ahead, info.local_behind) {
                let div_str = git::format_local_divergence(ahead, behind);
                let color = if behind > 0 { theme.defer } else { theme.complete };
                println!(
                    "    {} {}",
                    theme.paint("Local:", theme.label),
                    theme.paint(&div_str, color),
                );
            }
        }

        // ── Cached remote divergence (from last moco sync status) ────────────
        if project.git_sync_enabled {
            match git::format_cached_divergence(
                project.remote_ahead,
                project.remote_behind,
                project.last_remote_check,
                ttl,
            ) {
                Some(cached) => {
                    let color = if project.remote_behind.unwrap_or(0) > 0 {
                        theme.defer
                    } else {
                        theme.complete
                    };
                    println!(
                        "    {} {}",
                        theme.paint("Remote:", theme.label),
                        theme.paint(&cached, color),
                    );
                }
                None if project.last_remote_check.is_some() => {
                    // Was checked before but TTL expired.
                    println!(
                        "    {} {} — run `moco sync status` to refresh",
                        theme.paint("Remote:", theme.label),
                        theme.paint("(stale)", theme.defer),
                    );
                }
                None => {
                    // Never checked — omit silently.
                }
            }
        } else {
            println!(
                "    {} (sync disabled)",
                theme.paint("Remote:", theme.label),
            );
        }

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

