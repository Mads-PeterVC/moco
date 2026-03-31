use std::path::Path;

use clap::Args;

use crate::db::Store;
use crate::models::{Task, TaskStatus};
use crate::theme::Theme;
use crate::workspace;

#[derive(Args)]
pub struct ListArgs {
    /// List global tasks instead of the current project's tasks.
    #[arg(short, long)]
    pub global: bool,

    /// Cross-project view: list tasks from all projects with this label.
    #[arg(long, value_name = "NAME", conflicts_with = "global")]
    pub label: Option<String>,

    /// Filter tasks in the current scope to only those with this tag.
    #[arg(long, value_name = "NAME")]
    pub tag: Option<String>,
}

pub fn run(args: &ListArgs, store: &dyn Store, cwd: &Path, theme: &Theme) -> anyhow::Result<()> {
    // Cross-project label view.
    if let Some(label) = &args.label {
        let groups = store.list_tasks_by_label(label)?;
        if groups.is_empty() {
            println!("No tasks found for label '{}'.", label);
            return Ok(());
        }
        println!("Tasks with label '{}':\n", label);
        for (project, tasks) in &groups {
            println!("{}", theme.paint(&project.name, theme.accent));
            println!("{}", "─".repeat(60));
            let open: Vec<&Task> = tasks.iter().filter(|t| t.status == TaskStatus::Open).collect();
            let completed: Vec<&Task> =
                tasks.iter().filter(|t| t.status == TaskStatus::Complete).collect();
            let deferred: Vec<&Task> =
                tasks.iter().filter(|t| t.status == TaskStatus::Defer).collect();
            if !open.is_empty() {
                print_section(&open, tasks, theme);
            }
            if !completed.is_empty() {
                print_section(&completed, tasks, theme);
            }
            if !deferred.is_empty() {
                print_section(&deferred, tasks, theme);
            }
            println!();
        }
        return Ok(());
    }

    let (project_id, scope_label) = if args.global {
        (None, "global".to_string())
    } else {
        match workspace::resolve(store, cwd)? {
            Some(p) => (Some(p.id), format!("project '{}'", p.name)),
            None => (None, "global".to_string()),
        }
    };

    // Tag filter view.
    if let Some(tag) = &args.tag {
        let tasks = store.list_tasks_by_tag(project_id, tag)?;
        if tasks.is_empty() {
            println!("No tasks tagged '{}' for {}.", tag, scope_label);
            return Ok(());
        }
        println!("Tasks tagged '{}' for {}:\n", tag, scope_label);
        print_status_sections(&tasks, theme);
        return Ok(());
    }

    let tasks = store.list_tasks(project_id)?;

    if tasks.is_empty() {
        println!("No tasks for {}.", scope_label);
        return Ok(());
    }

    println!("Tasks for {}:\n", scope_label);
    print_status_sections(&tasks, theme);

    Ok(())
}

fn print_status_sections(tasks: &[Task], theme: &Theme) {
    let open: Vec<&Task> = tasks.iter().filter(|t| t.status == TaskStatus::Open).collect();
    let completed: Vec<&Task> = tasks.iter().filter(|t| t.status == TaskStatus::Complete).collect();
    let deferred: Vec<&Task> = tasks.iter().filter(|t| t.status == TaskStatus::Defer).collect();

    if !open.is_empty() {
        println!("{}", theme.paint("Open", theme.open));
        println!("{}", "─".repeat(60));
        print_section(&open, tasks, theme);
        println!();
    }
    if !completed.is_empty() {
        println!("{}", theme.paint("Completed", theme.complete));
        println!("{}", "─".repeat(60));
        print_section(&completed, tasks, theme);
        println!();
    }
    if !deferred.is_empty() {
        println!("{}", theme.paint("Deferred", theme.defer));
        println!("{}", "─".repeat(60));
        print_section(&deferred, tasks, theme);
        println!();
    }
}

/// Print a section of tasks, rendering subtasks indented beneath their parent.
fn print_section(section_tasks: &[&Task], all_tasks: &[Task], theme: &Theme) {
    for task in section_tasks {
        if task.parent_id.is_some() {
            continue;
        }
        print_task(task, None, false, theme);
        print_subtasks(task, all_tasks, theme);
    }
}

/// Recursively print subtasks of `parent_id`, indented with a tree connector.
fn print_subtasks(parent: &Task, all_tasks: &[Task], theme: &Theme) {
    let mut children: Vec<&Task> = all_tasks
        .iter()
        .filter(|t| t.parent_id == Some(parent.id))
        .collect();

    children.sort_by_key(|t| t.sub_index.unwrap_or(0));

    let last_idx = children.len().saturating_sub(1);
    for (i, child) in children.iter().enumerate() {
        print_task(child, Some(parent), i == last_idx, theme);
        print_subtasks(child, all_tasks, theme);
    }
}

fn print_task(task: &Task, parent: Option<&Task>, is_last_sibling: bool, theme: &Theme) {
    let id_color = match task.status {
        TaskStatus::Open => theme.open,
        TaskStatus::Complete => theme.complete,
        TaskStatus::Defer => theme.defer,
    };
    let is_subtask = parent.is_some();
    // Pad the plain ID string before colorizing — ANSI escape codes inflate byte
    // length, which would make Rust's {:>N} format specifier skip padding entirely.
    let raw_id = task.display_id_in_context(parent);
    let bar = progress_bar(task.progress, 20, theme);

    let preview = task.content.lines().next().unwrap_or("").trim();
    let preview = if preview.len() > 50 {
        format!("{}…", &preview[..50])
    } else {
        preview.to_string()
    };

    let tags = if task.tags.is_empty() {
        String::new()
    } else {
        let tag_str = task.tags.join(" ");
        format!("  {}", theme.paint(format!("[{tag_str}]"), theme.accent))
    };

    if is_subtask {
        // Right-align subtask IDs in a 6-char field to accommodate the longest
        // prefixes (e.g. "CS#1.1", "DS#1.1"); pad before colorizing.
        let id = theme.paint(format!("{:>6}", raw_id), id_color);
        let connector = if is_last_sibling { "└─" } else { "├─" };
        println!("   {} {}  {}  {}{}", connector, id, bar, preview, tags);
    } else {
        let id = theme.paint(raw_id, id_color);
        println!("  {}  {}  {}{}", id, bar, preview, tags);
    }
}

/// Render a compact progress bar with theme colors applied via ANSI.
pub fn progress_bar(progress: u8, width: usize, theme: &Theme) -> String {
    let filled = ((progress as usize) * width) / 100;
    let empty = width - filled;
    format!(
        "[{}{}]",
        theme.paint("█".repeat(filled), theme.progress_filled),
        theme.paint("░".repeat(empty), theme.progress_empty),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ThemeConfig;

    fn default_theme() -> Theme {
        Theme::resolve(&ThemeConfig::default())
    }

    #[test]
    fn progress_bar_empty() {
        let theme = default_theme();
        // Strip ANSI codes for comparison by checking raw character counts
        let bar = progress_bar(0, 10, &theme);
        assert!(bar.contains('░'));
        assert!(!bar.contains('█'));
    }

    #[test]
    fn progress_bar_full() {
        let theme = default_theme();
        let bar = progress_bar(100, 10, &theme);
        assert!(bar.contains('█'));
        assert!(!bar.contains('░'));
    }

    #[test]
    fn progress_bar_half() {
        let theme = default_theme();
        let bar = progress_bar(50, 10, &theme);
        assert!(bar.contains('█'));
        assert!(bar.contains('░'));
    }
}
