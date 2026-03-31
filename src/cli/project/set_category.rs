use clap::Args;

use crate::db::Store;
use crate::theme::Theme;

#[derive(Args)]
pub struct SetCategoryArgs {
    /// Name of the project to assign.
    pub project_name: String,
    /// Category to assign the project to.
    #[arg(conflicts_with = "unset")]
    pub category: Option<String>,
    /// Remove the project's category assignment.
    #[arg(long, conflicts_with = "category")]
    pub unset: bool,
}

pub fn run(args: &SetCategoryArgs, store: &mut dyn Store, theme: &Theme) -> anyhow::Result<()> {
    // Resolve the project by name (case-insensitive).
    let all_projects = store.list_projects()?;
    let name_lower = args.project_name.to_lowercase();
    let matches: Vec<_> = all_projects
        .into_iter()
        .filter(|p| p.name.to_lowercase() == name_lower)
        .collect();

    let mut project = match matches.len() {
        0 => anyhow::bail!("No project named '{}' found.", args.project_name),
        1 => matches.into_iter().next().unwrap(),
        _ => anyhow::bail!(
            "Multiple projects named '{}' found. Rename them to disambiguate.",
            args.project_name
        ),
    };

    if args.unset {
        let old = project.category.take();
        store.update_project(&project)?;
        match old {
            Some(cat) => println!(
                "Removed category '{}' from project {}.",
                cat,
                theme.paint(&project.name, theme.accent),
            ),
            None => println!(
                "Project {} had no category assigned.",
                theme.paint(&project.name, theme.accent),
            ),
        }
        return Ok(());
    }

    let category_name = args
        .category
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("Provide a category name or use --unset."))?;

    // Validate the category exists.
    if store.get_category(category_name)?.is_none() {
        anyhow::bail!(
            "Category '{}' does not exist. Register it first with \
             `moco project category add {}`.",
            category_name,
            category_name,
        );
    }

    project.category = Some(category_name.to_string());
    store.update_project(&project)?;

    println!(
        "Assigned project {} to category {}.",
        theme.paint(&project.name, theme.accent),
        theme.paint(category_name, theme.accent),
    );

    Ok(())
}
