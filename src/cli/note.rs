use std::io::{self, Write};
use std::path::Path;

use clap::{Args, Subcommand};
use crossterm::event::{self, Event, KeyEventKind};

use crate::db::Store;
use crate::theme::Theme;
use crate::tui::{
    self,
    form::{FormOutcome, TaskForm},
};
use crate::workspace;

#[derive(Args)]
pub struct NoteArgs {
    #[command(subcommand)]
    pub subcommand: NoteCommand,
}

#[derive(Subcommand)]
pub enum NoteCommand {
    /// Add a note to the current scope. Omit title to open the interactive form.
    Add {
        /// Note title.
        title: Option<String>,
        /// Note content.
        content: Option<String>,
    },
    /// List notes in the current scope.
    List,
    /// Edit a note's content.
    Edit {
        /// Display index of the note to edit (e.g. 1 for N#1).
        #[arg(short = 'n', long = "note", value_name = "ID")]
        note_id: u32,
        /// New content to apply to the note.
        content: Option<String>,
        /// Append the content to the existing note body.
        #[arg(long, conflicts_with = "replace")]
        append: bool,
        /// Replace the note body with the new content.
        #[arg(long)]
        replace: bool,
    },
    /// Delete a note (prompts for confirmation unless --yes is given).
    Delete {
        /// Display index of the note to delete (e.g. 1 for N#1).
        #[arg(short = 'n', long = "note", value_name = "ID")]
        note_id: u32,
        /// Skip the confirmation prompt and delete immediately.
        #[arg(long, short = 'y')]
        yes: bool,
    },
}

pub fn run(args: &NoteArgs, store: &mut dyn Store, cwd: &Path, theme: &Theme) -> anyhow::Result<()> {
    let project_id = workspace::resolve(store, cwd)?.map(|p| p.id);

    match &args.subcommand {
        NoteCommand::Add { title, content } => {
            let (title, content) = match title {
                Some(t) => (
                    t.trim().to_string(),
                    content.as_deref().unwrap_or("").to_string(),
                ),
                None => match run_note_form(theme)? {
                    Some(pair) => pair,
                    None => return Ok(()), // user cancelled
                },
            };

            if title.is_empty() {
                anyhow::bail!("Note title cannot be empty.");
            }

            let note = store.add_note(project_id, &title, &content)?;
            println!("Added note {}: {}", note.display_id(), note.title);
        }

        NoteCommand::List => {
            let notes = store.list_notes(project_id)?;
            if notes.is_empty() {
                println!("No notes.");
                return Ok(());
            }
            println!("Notes:\n");
            for note in &notes {
                let preview = note.content.lines().next().unwrap_or("").trim();
                let preview = if preview.len() > 50 {
                    format!("{}…", &preview[..50])
                } else {
                    preview.to_string()
                };
                let id = theme.paint(note.display_id(), theme.accent);
                if preview.is_empty() {
                    println!("  {}  {}", id, note.title);
                } else {
                    println!("  {}  {} — {}", id, note.title, preview);
                }
            }
        }

        NoteCommand::Edit {
            note_id,
            content,
            append,
            replace: _,
        } => {
            let mut note = store
                .get_note(project_id, *note_id)?
                .ok_or_else(|| anyhow::anyhow!("Note N#{} not found.", note_id))?;

            if let Some(new_content) = content {
                if *append {
                    if note.content.is_empty() {
                        note.content = new_content.clone();
                    } else {
                        note.content = format!("{}\n{}", note.content, new_content);
                    }
                } else {
                    // Default to replace when content is provided without --append.
                    note.content = new_content.clone();
                }
            } else {
                anyhow::bail!("Provide content to edit the note.");
            }

            note.updated_at = chrono::Utc::now();
            store.update_note(&note)?;
            println!("Updated note {}.", note.display_id());
        }

        NoteCommand::Delete { note_id, yes } => {
            let note = store
                .get_note(project_id, *note_id)?
                .ok_or_else(|| anyhow::anyhow!("Note N#{} not found.", note_id))?;

            if !yes {
                print!("Delete note {}? [y/N]: ", note.display_id());
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                if input.trim().to_ascii_lowercase() != "y" {
                    println!("Cancelled.");
                    return Ok(());
                }
            }

            store.delete_note(note.id)?;
            println!("Deleted note {}.", note.display_id());
        }
    }

    Ok(())
}

/// Open a TUI form to create a note. Returns `(title, content)` or `None` if cancelled.
fn run_note_form(theme: &Theme) -> anyhow::Result<Option<(String, String)>> {
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
                        continue;
                    }
                    return Ok(Some((title, body.trim_end().to_string())));
                }
                FormOutcome::Cancelled => return Ok(None),
                FormOutcome::Continue => {}
            }
        }
    }
}