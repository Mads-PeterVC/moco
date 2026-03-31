use std::path::Path;

use clap::Args;
use crossterm::event::{self, Event, KeyEventKind};

use crate::db::Store;
use crate::theme::Theme;
use crate::tui::{
    self,
    form::{FormOutcome, TaskForm},
};
use crate::workspace;

#[derive(Args)]
pub struct AddArgs {
    /// Task description (Markdown supported). Omit to open the interactive form.
    pub description: Option<String>,

    /// Add as a subtask of the open task with this display index.
    #[arg(long, value_name = "TASK_ID")]
    pub sub: Option<u32>,

    /// Add to the global task list instead of the current project.
    #[arg(short, long)]
    pub global: bool,

    /// Add one or more tags to the task (can be repeated).
    #[arg(long = "tag", value_name = "TAG")]
    pub tags: Vec<String>,
}

pub fn run(args: &AddArgs, store: &mut dyn Store, cwd: &Path, theme: &Theme) -> anyhow::Result<()> {
    let project_id = if args.global {
        None
    } else {
        workspace::resolve(store, cwd)?.map(|p| p.id)
    };

    // Resolve parent task if --sub was given.
    let parent_id = if let Some(sub_index) = args.sub {
        let parent = store
            .get_open_task(project_id, sub_index)?
            .ok_or_else(|| crate::error::MocoError::TaskNotFound(sub_index))?;
        Some(parent.id)
    } else {
        None
    };

    let content = match &args.description {
        Some(d) => {
            let d = d.trim().to_string();
            if d.is_empty() {
                anyhow::bail!("Task description cannot be empty.");
            }
            d
        }
        None => {
            // Open the TUI form.
            match run_form(theme)? {
                Some(c) => c,
                None => return Ok(()), // user cancelled
            }
        }
    };

    let mut task = store.add_task(project_id, &content, parent_id)?;

    if !args.tags.is_empty() {
        task.tags = args.tags.clone();
        store.update_task(&task)?;
    }

    let scope = match project_id {
        Some(_) => "project".to_string(),
        None => "global".to_string(),
    };
    let preview = task.content.lines().next().unwrap_or("").trim().to_string();

    // For subtasks, display using the parent's context.
    let display = if let Some(pid) = task.parent_id {
        let parent = store.get_task_by_id(pid)?;
        task.display_id_in_context(parent.as_ref())
    } else {
        task.display_id()
    };

    println!("Added {} task {}: {}", scope, display, preview);

    Ok(())
}

/// Open the two-field TUI form. Returns the composed content string on submit,
/// or `None` if the user cancelled.
fn run_form(theme: &Theme) -> anyhow::Result<Option<String>> {
    let mut guard = tui::enter()?;
    let mut form = TaskForm::new(theme.clone());

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
                    let (title, body) = form.values();
                    let title = title.trim().to_string();
                    if title.is_empty() {
                        // Leave the form open so the user can fill in the title.
                        continue;
                    }
                    let content = if body.trim().is_empty() {
                        title
                    } else {
                        format!("{}\n{}", title, body.trim_end())
                    };
                    return Ok(Some(content));
                }
                FormOutcome::Cancelled => return Ok(None),
                FormOutcome::Continue => {}
            }
        }
    }
}