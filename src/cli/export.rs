use std::path::Path;

use clap::Args;

use crate::config::AppConfig;
use crate::db::Store;
use crate::error::MocoError;
use crate::models::{Task, TaskStatus};
use crate::workspace;

#[derive(Args)]
pub struct ExportArgs {
    /// Export the global task list instead of the current project.
    #[arg(short, long)]
    pub global: bool,
}

pub fn run(
    args: &ExportArgs,
    store: &dyn Store,
    cwd: &Path,
    config: &AppConfig,
) -> anyhow::Result<()> {
    let (project_id, output_path, title) = if args.global {
        let path = config.moco_dir.join("global.md");
        (None, path, "Global Tasks".to_string())
    } else {
        let project = workspace::resolve(store, cwd)?
            .ok_or(MocoError::ProjectNotFound)?;
        let filename = format!("{}.md", sanitize_filename(&project.name));
        let path = project.path.join(filename);
        (Some(project.id), path, project.name.clone())
    };

    let tasks = store.list_tasks(project_id)?;
    let markdown = render_markdown(&title, &tasks);

    std::fs::write(&output_path, &markdown)?;
    println!("Exported to {}", output_path.display());

    Ok(())
}

/// Render tasks as a Markdown document.
pub fn render_markdown(title: &str, tasks: &[Task]) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", title));

    let open: Vec<&Task> = tasks.iter().filter(|t| t.status == TaskStatus::Open).collect();
    let completed: Vec<&Task> = tasks.iter().filter(|t| t.status == TaskStatus::Complete).collect();
    let deferred: Vec<&Task> = tasks.iter().filter(|t| t.status == TaskStatus::Defer).collect();

    if !open.is_empty() {
        out.push_str("## Open\n\n");
        render_task_list(&mut out, &open, tasks, 0);
        out.push('\n');
    }

    if !completed.is_empty() {
        out.push_str("## Completed\n\n");
        render_task_list(&mut out, &completed, tasks, 0);
        out.push('\n');
    }

    if !deferred.is_empty() {
        out.push_str("## Deferred\n\n");
        render_task_list(&mut out, &deferred, tasks, 0);
        out.push('\n');
    }

    out
}

fn render_task_list(out: &mut String, section: &[&Task], all_tasks: &[Task], depth: usize) {
    let indent = "  ".repeat(depth);
    for task in section {
        if task.parent_id.is_some() && depth == 0 {
            continue; // subtasks rendered recursively
        }
        let checkbox = match task.status {
            TaskStatus::Open => "[ ]",
            TaskStatus::Complete => "[x]",
            TaskStatus::Defer => "[-]",
        };
        let id = task.display_id();
        let title = task.content.lines().next().unwrap_or("").trim();
        let body_lines: Vec<&str> = task.content.lines().skip(1).collect();

        out.push_str(&format!(
            "{}- {} {} {} ({}%)\n",
            indent,
            checkbox,
            id,
            title,
            task.progress
        ));

        // Append body lines as a blockquote continuation.
        for line in &body_lines {
            if !line.trim().is_empty() {
                out.push_str(&format!("{}  > {}\n", indent, line));
            }
        }

        // Recurse for subtasks.
        let children: Vec<&Task> = all_tasks
            .iter()
            .filter(|t| t.parent_id == Some(task.id))
            .collect();
        if !children.is_empty() {
            render_task_list(out, &children, all_tasks, depth + 1);
        }
    }
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Task, TaskStatus};

    fn open_task(content: &str, index: u32) -> Task {
        Task::new(None, content, index, None)
    }

    fn completed_task(content: &str, completed_index: u32) -> Task {
        let mut t = Task::new(None, content, 0, None);
        t.status = TaskStatus::Complete;
        t.completed_index = Some(completed_index);
        t
    }

    fn deferred_task(content: &str, deferred_index: u32) -> Task {
        let mut t = Task::new(None, content, 0, None);
        t.status = TaskStatus::Defer;
        t.deferred_index = Some(deferred_index);
        t
    }

    #[test]
    fn render_markdown_has_title() {
        let md = render_markdown("My Project", &[]);
        assert!(md.starts_with("# My Project\n"));
    }

    #[test]
    fn render_markdown_open_section() {
        let tasks = vec![open_task("Fix the bug", 1)];
        let md = render_markdown("Proj", &tasks);
        assert!(md.contains("## Open"));
        assert!(md.contains("[ ]"));
        assert!(md.contains("Fix the bug"));
    }

    #[test]
    fn render_markdown_completed_section() {
        let tasks = vec![completed_task("Done task", 1)];
        let md = render_markdown("Proj", &tasks);
        assert!(md.contains("## Completed"));
        assert!(md.contains("[x]"));
    }

    #[test]
    fn render_markdown_deferred_section() {
        let tasks = vec![deferred_task("Later task", 1)];
        let md = render_markdown("Proj", &tasks);
        assert!(md.contains("## Deferred"));
        assert!(md.contains("[-]"));
    }

    #[test]
    fn render_markdown_subtask_indented() {
        let parent = open_task("Parent", 1);
        let mut child = Task::new(None, "Child", 2, None);
        child.parent_id = Some(parent.id);
        let tasks = vec![parent.clone(), child];
        let md = render_markdown("Proj", &tasks);
        // Child should appear indented (two spaces before the list marker).
        assert!(md.contains("  - "));
        assert!(md.contains("Child"));
    }

    #[test]
    fn sanitize_filename_replaces_spaces() {
        assert_eq!(sanitize_filename("my project"), "my-project");
    }

    #[test]
    fn sanitize_filename_keeps_alphanumeric() {
        assert_eq!(sanitize_filename("my-proj_1"), "my-proj_1");
    }
}
