mod cli;
mod config;
mod db;
mod error;
mod git;
mod models;
mod theme;
mod tui;
mod workspace;

use clap::{CommandFactory, FromArgMatches};

fn main() -> anyhow::Result<()> {
    // Load config and resolve the theme before parsing so that `--help` output
    // is rendered with the user's colour palette via clap's Styles API.
    let config = config::AppConfig::load()?;
    let theme = theme::Theme::resolve(&config.moco_config.theme);

    let matches = cli::Cli::command()
        .styles(theme.to_clap_styles())
        .get_matches();
    let cli = cli::Cli::from_arg_matches(&matches).unwrap_or_else(|e| e.exit());

    let mut store = db::RedbStore::open(&config.db_path)?;
    let cwd = std::env::current_dir()?;

    match &cli.command {
        cli::Command::Project(args) => cli::project::run(args, &mut store, &cwd, &config, &theme),
        cli::Command::Add(args) => cli::add::run(args, &mut store, &cwd, &theme),
        cli::Command::Edit(args) => cli::edit::run(args, &mut store, &cwd, &theme),
        cli::Command::Status(args) => cli::status::run(args, &mut store, &cwd, &theme),
        cli::Command::List(args) => cli::list::run(args, &mut store, &cwd, &theme),
        cli::Command::Tag(args) => cli::tag::run(args, &mut store, &cwd, &theme),
        cli::Command::Note(args) => cli::note::run(args, &mut store, &cwd, &theme),
        cli::Command::Remove(args) => cli::remove::run(args, &mut store, &cwd, &theme),
        cli::Command::Config(args) => cli::config::run(args, &config, &theme),
        cli::Command::Sync(args) => cli::sync::run(args, &mut store, &config, &theme),
    }
}
