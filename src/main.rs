mod cli;
mod config;
mod db;
mod error;
mod models;
mod tui;
mod workspace;

use clap::Parser;

fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    let config = config::AppConfig::load()?;
    let mut store = db::RedbStore::open(&config.db_path)?;
    let cwd = std::env::current_dir()?;

    match &cli.command {
        cli::Command::Init(args) => cli::init::run(args, &mut store, &cwd),
        cli::Command::Add(args) => cli::add::run(args, &mut store, &cwd),
        cli::Command::Edit(args) => cli::edit::run(args, &mut store, &cwd),
        cli::Command::Status(args) => cli::status::run(args, &mut store, &cwd),
        cli::Command::List(args) => cli::list::run(args, &store, &cwd),
        cli::Command::Export(args) => cli::export::run(args, &store, &cwd, &config),
        cli::Command::Open(args) => cli::open::run(args, &store, &config),
        cli::Command::Delete(args) => cli::delete::run(args, &mut store),
        cli::Command::Config(args) => cli::config::run(args, &config),
    }
}
