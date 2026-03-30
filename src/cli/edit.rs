use std::path::Path;

use clap::Args;
use crossterm::event::{self, Event, KeyEventKind};

use crate::db::Store;
use crate::error::MocoError;
use crate::tui::{
    self,
    browser::{BrowserOutcome, TaskBrowser},
    form::{FormOutcome, TaskForm},
};
use crate::workspace;

#[derive(Args)]
pub struct EditArgs {
    /// Display index of the open task to edit. If omitted, opens the task browser.
    #[arg(short = 't', long = "task", value_name = "TASK_ID")]
    pub task_id: Option<u32>,

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

pub fn run(args: &EditArgs, store: &mut dyn Store, cwd: &Path) -> anyhow::Result<()> {
    let project_id = if args.global {
        None
    } else {
        workspace::resolve(store, cwd)?.map(|p| p.id)
    };

    // Non-interactive path: content + --append or --replace flag provided.
    if let Some(content) = &args.content {
        let task_id = args
            .task_id
            .ok_or_else(|| anyhow::anyhow!("-t <TASK_ID> is required for non-interactive edit"))?;

        let mut task = store
            .get_open_task(project_id, task_id)?
            .ok_or(MocoError::TaskNotFound(task_id))?;

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

        task.updated_at = chrono::Utc::now();
        store.update_task(&task)?;
        println!("Task {} updated.", task.display_id());
        return Ok(());
    }

    // TUI path.
    let task_id = if let Some(id) = args.task_id {
        id
    } else {
        // Open browser to let the user pick a task.
        let tasks = store.list_tasks(project_id)?;
        if tasks.is_empty() {
            println!("No tasks to edit.");
            return Ok(());
        }
        let selected_index = run_browser(&tasks)?;
        match selected_index {
            None => return Ok(()), // cancelled
            Some(i) => {
                let selected_task = &tasks[i];
                match selected_task.status {
                    crate::models::TaskStatus::Open => selected_task.display_index,
                    _ => {
                        // For non-open tasks, we'd need a different lookup — out of scope for now.
                        anyhow::bail!(
                            "Only open tasks can be edited via the TUI. Use {} directly with -t.",
                            selected_task.display_id()
                        );
                    }
                }
            }
        }
    };

    let mut task = store
        .get_open_task(project_id, task_id)?
        .ok_or(MocoError::TaskNotFound(task_id))?;

    // Split existing content into title (first line) and body (rest).
    let mut lines = task.content.splitn(2, '\n');
    let existing_title = lines.next().unwrap_or("").trim().to_string();
    let existing_body = lines.next().unwrap_or("").to_string();

    let (new_title, new_body) = run_form(existing_title, existing_body)?;

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
    task.updated_at = chrono::Utc::now();
    store.update_task(&task)?;
    println!("Task {} updated.", task.display_id());

    Ok(())
}

/// Run the task browser TUI, returning the selected task index or None if cancelled.
fn run_browser(tasks: &[crate::models::Task]) -> anyhow::Result<Option<usize>> {
    let mut guard = tui::enter()?;
    let mut browser = TaskBrowser::new(tasks.len());

    loop {
        let tasks_ref = tasks;
        guard.terminal.draw(|frame| {
            browser.render(frame, frame.area(), tasks_ref);
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
) -> anyhow::Result<(Option<String>, Option<String>)> {
    let mut guard = tui::enter()?;
    let mut form = TaskForm::with_values(&title, &body);

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
