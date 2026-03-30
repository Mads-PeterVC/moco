use std::path::Path;

use clap::Args;

use crate::db::Store;
use crate::error::MocoError;
use crate::workspace;

#[derive(Args)]
pub struct InitArgs {
    /// Name for this project/workspace.
    pub name: String,

    /// Re-initialize even if this directory is already registered.
    #[arg(long)]
    pub force: bool,
}

pub fn run(args: &InitArgs, store: &mut dyn Store, cwd: &Path) -> anyhow::Result<()> {
    let canonical = workspace::canonical(cwd);

    // Check if this exact path is already registered.
    if let Some(existing) = store.get_project_by_path(&canonical)? {
        if !args.force {
            return Err(MocoError::AlreadyInitialized.into());
        }
        eprintln!(
            "Warning: overwriting existing project '{}' at this path.",
            existing.name
        );
    }

    let project = store.create_project(&args.name, &canonical)?;
    println!(
        "Initialized project '{}' at {}",
        project.name,
        canonical.display()
    );

    Ok(())
}
