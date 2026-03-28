pub mod like;
pub mod list;
pub mod unlike;

use anyhow::Result;
use clap::{Args as ClapArgs, Subcommand};
use huggingface_hub::HfApi;

use crate::output::CommandResult;

/// Manage likes
#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    pub command: LikesCommand,
}

/// Likes subcommands
#[derive(Subcommand)]
pub enum LikesCommand {
    /// Like a repository
    Like(like::Args),
    /// Remove a like from a repository
    Unlike(unlike::Args),
    /// List liked repositories
    List(list::Args),
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    match args.command {
        LikesCommand::Like(a) => like::execute(api, a).await,
        LikesCommand::Unlike(a) => unlike::execute(api, a).await,
        LikesCommand::List(a) => list::execute(api, a).await,
    }
}
