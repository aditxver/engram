use anyhow::Result;
use clap::Parser;
use engram::cli::{Cli, Commands};
use engram::index;

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Add {
            paths,
            recursive,
            no_progress,
        } => index::add(&paths, recursive, no_progress)?,
        Commands::Search {
            query,
            limit,
            show_path,
        } => index::search(&query, limit, show_path)?,
        Commands::Remove { paths } => index::remove(&paths)?,
        Commands::Rebuild => index::rebuild()?,
        Commands::Status => index::status()?,
    }
    Ok(())
}
