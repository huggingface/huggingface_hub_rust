pub mod list;
pub mod rm;

use anyhow::Result;
use clap::{Args as ClapArgs, Subcommand};
use huggingface_hub::HfApi;

use crate::output::CommandResult;

/// Manage the local model cache
#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    pub command: CacheCommand,
}

/// Cache subcommands
#[derive(Subcommand)]
pub enum CacheCommand {
    /// List cached repositories and files
    List(list::Args),
    /// Delete cached revisions
    Rm(rm::Args),
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    match args.command {
        CacheCommand::List(a) => list::execute(api, a).await,
        CacheCommand::Rm(a) => rm::execute(api, a).await,
    }
}
