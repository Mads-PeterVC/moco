use std::path::Path;

use clap::Args;
use crossterm::event::{self, Event, KeyEventKind};

use crate::cli::task_ref;
use crate::db::Store;
use crate::theme::Theme;
use crate::tui::{
    self,
    browser::{BrowserOutcome, TaskBrowser},
    form::{FormOutcome, TaskForm},
};
use crate::workspace;

#[derive(Args)]
pub struct EditArgs {
    /// Task reference to edit: open (`1`), subtask (`1.2`), completed (`C1`), deferred (`D1`).
    /// If omitted, opens the task browser.
    #[arg(short = 't', long = "task", value_name = "TASK_ID")]
    pub task_id: Option<String>,

    /// New content for the task (non-interactive mode).
    pub content: Option<String>,

    /// Append content to the existing task body (non-interactive).
    #[arg(long, conflicts_with = "replace")]
    pub append: bool,

    /// Replace the task body with new content (non-interactive).
    #[arg(long, conflicts_with = "append")]
    pub replace: bool,

    /// Operate on the global task list instead of the current project.
    #[arg(short, long)]
    pub global: bool,
}

pub fn run(args: &EditArgs, store: &mut dyn Store, cwd: &Path, theme: &Theme) -> anyhow::Result<()> {
    let project_id = if args.global {
        None
    } else {
        workspace::resolve(store, cwd)?.map(|p| p.id)
    };

    // Non-interactive path: content + --append or --replace flag provided.
    if let Some(content) = &args.content {
        let task_id_str = args
            .task_id
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("-t <TASK_ID> is required for non-interactive edit"))?;

        let task_ref = task_ref::parse(task_id_str).map_err(|e| anyhow::anyhow!(e))?;
        let mut task = task_ref::resolve(store, project_id, &task_ref)?
            .ok_or_else(|| anyhow::anyhow!("Task '{}' not found.", task_id_str))?;

        if args.append {
            if !task.content.is_empty() && !task.content.ends_with('\n') {
                task.content.push('\n');
            }
            task.content.push_str(content);
        } else if args.replace {
            task.content = content.clone();
        } else {
            anyhow::bail!("Provide --append or --replace with content.");
        }

        let parent = if let Some(pid) = task.parent_id {
            store.get_task_by_id(pid)?
        } else {
            None
        };
        let display = task.display_id_in_context(parent.as_ref());
        task.updated_at = chrono::Utc::now();
        store.update_task(&task)?;
        println!("Task {} updated.", display);
        return Ok(());
    }

    // TUI path.
    let task = if let Some(ref id_str) = args.task_id {
        let task_ref = task_ref::parse(id_str).map_err(|e| anyhow::anyhow!(e))?;
        task_ref::resolve(store, project_id, &task_ref)?
            .ok_or_else(|| anyhow::anyhow!("Task '{}' not found.", id_str))?
    } else {
        // Open browser to let the user pick a task.
        let tasks = store.list_tasks(project_id)?;
        if tasks.is_empty() {
            println!("No tasks to edit.");
            return Ok(());
        }
        let selected_index = run_browser(&tasks, theme)?;
        match selected_index {
            None => return Ok(()), // cancelled
            Some(i) => tasks[i].clone(),
        }
    };

    let mut task = task;

    // Split existing content into title (first line) and body (rest).
    let mut lines = task.content.splitn(2, '\n');
    let existing_title = lines.next().unwrap_or("").trim().to_string();
    let existing_body = lines.next().unwrap_or("").to_string();

    let (new_title, new_body) = run_form(existing_title, existing_body, theme)?;

    if new_title.is_none() {
        // User cancelled.
        return Ok(());
    }

    let (title, body) = (new_title.unwrap(), new_body.unwrap_or_default());
    task.content = if body.trim().is_empty() {
        title.clone()
    } else {
        format!("{}\n{}", title, body)
    };

    let parent = if let Some(pid) = task.parent_id {
        store.get_task_by_id(pid)?
    } else {
        None
    };
    let display = task.display_id_in_context(parent.as_ref());
    task.updated_at = chrono::Utc::now();
    store.update_task(&task)?;
    println!("Task {} updated.", display);

    Ok(())
}

/// Run the task browser TUI, returning the selected task index or None if cancelled.
fn run_browser(tasks: &[crate::models::Task], theme: &Theme) -> anyhow::Result<Option<usize>> {
    let mut guard = tui::enter()?;
    let mut browser = TaskBrowser::new(tasks.len());

    loop {
        let tasks_ref = tasks;
        guard.terminal.draw(|frame| {
            browser.render(frame, frame.area(), tasks_ref, theme);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match browser.handle_key(key) {
                BrowserOutcome::Selected(i) => return Ok(Some(i)),
                BrowserOutcome::Cancelled => return Ok(None),
                BrowserOutcome::Continue => {}
            }
        }
    }
}

/// Run the task edit form TUI. Returns `(Some(title), Some(body))` on submit,
/// or `(None, None)` on cancel.
fn run_form(
    title: String,
    body: String,
    theme: &Theme,
) -> anyhow::Result<(Option<String>, Option<String>)> {
    let mut guard = tui::enter()?;
    let mut form = TaskForm::with_values(&title, &body, theme.clone());

    loop {
        guard.terminal.draw(|frame| {
            form.render(frame, frame.area());
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match form.handle_key(key) {
                FormOutcome::Submitted => {
                    let (t, b) = form.values();
                    return Ok((Some(t), Some(b)));
                }
                FormOutcome::Cancelled => return Ok((None, None)),
                FormOutcome::Continue => {}
            }
        }
    }
}