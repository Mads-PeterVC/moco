use std::path::Path;

use clap::Args;
use crossterm::event::{self, Event, KeyEventKind};

use crate::db::Store;
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
}

pub fn run(args: &AddArgs, store: &mut dyn Store, cwd: &Path) -> anyhow::Result<()> {
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
            match run_form()? {
                Some(c) => c,
                None => return Ok(()), // user cancelled
            }
        }
    };

    let task = store.add_task(project_id, &content, parent_id)?;

    let scope = match project_id {
        Some(_) => "project".to_string(),
        None => "global".to_string(),
    };
    let preview = task.content.lines().next().unwrap_or("").trim().to_string();
    println!("Added {} task {}: {}", scope, task.display_id(), preview);

    Ok(())
}

/// Open the two-field TUI form. Returns the composed content string on submit,
/// or `None` if the user cancelled.
fn run_form() -> anyhow::Result<Option<String>> {
    let mut guard = tui::enter()?;
    let mut form = TaskForm::new();

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
