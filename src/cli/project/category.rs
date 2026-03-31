use clap::{Args, Subcommand};

use crate::db::Store;
use crate::theme::Theme;

#[derive(Args)]
pub struct CategoryArgs {
    #[command(subcommand)]
    pub subcommand: CategoryCommand,
}

#[derive(Subcommand)]
pub enum CategoryCommand {
    /// Register a new category (appended last in display order).
    Add {
        /// Category name.
        name: String,
    },
    /// Remove a category. Fails if projects are assigned unless --force is given.
    Remove {
        /// Category name.
        name: String,
        /// Un-assign all projects in this category before removing it.
        #[arg(long)]
        force: bool,
    },
    /// List all registered categories in display order.
    List,
    /// Move a category to a new display position (1-based).
    Reorder {
        /// Category name.
        name: String,
        /// New 1-based position.
        position: usize,
    },
}

pub fn run(args: &CategoryArgs, store: &mut dyn Store, theme: &Theme) -> anyhow::Result<()> {
    match &args.subcommand {
        CategoryCommand::Add { name } => {
            if store.get_category(name)?.is_some() {
                anyhow::bail!("Category '{}' already exists.", name);
            }
            let category = store.create_category(name)?;
            println!(
                "Added category {} at position {}.",
                theme.paint(&category.name, theme.accent),
                category.order,
            );
        }

        CategoryCommand::Remove { name, force } => {
            if store.get_category(name)?.is_none() {
                anyhow::bail!("Category '{}' not found.", name);
            }

            // Check for assigned projects.
            let assigned: Vec<_> = store
                .list_projects()?
                .into_iter()
                .filter(|p| p.category.as_deref() == Some(name.as_str()))
                .collect();

            if !assigned.is_empty() && !force {
                anyhow::bail!(
                    "{} project(s) are assigned to category '{}'. \
                     Use --force to remove the category and un-assign them.",
                    assigned.len(),
                    name,
                );
            }

            store.delete_category(name)?;
            println!("Removed category '{}'.", name);
            if !assigned.is_empty() {
                println!("{} project(s) un-assigned.", assigned.len());
            }
        }

        CategoryCommand::List => {
            let categories = store.list_categories()?;
            if categories.is_empty() {
                println!("No categories registered. Use `moco project category add <name>` to create one.");
            } else {
                println!("Categories:");
                for cat in &categories {
                    println!(
                        "  {}. {}",
                        cat.order,
                        theme.paint(&cat.name, theme.accent),
                    );
                }
            }
        }

        CategoryCommand::Reorder { name, position } => {
            if store.get_category(name)?.is_none() {
                anyhow::bail!("Category '{}' not found.", name);
            }
            let count = store.list_categories()?.len();
            if *position == 0 || *position > count {
                anyhow::bail!(
                    "Position {} is out of range. Valid positions are 1–{}.",
                    position,
                    count
                );
            }
            store.reorder_category(name, *position)?;
            println!(
                "Moved category {} to position {}.",
                theme.paint(name, theme.accent),
                position,
            );
        }
    }

    Ok(())
}
