use anyhow::Result;
use clap::{Args as ClapArgs, Subcommand};
use huggingface_hub::HfApi;

use crate::output::CommandResult;

/// Manage repository branches
#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    pub command: BranchCommand,
}

/// Branch subcommands
#[derive(Subcommand)]
pub enum BranchCommand {
    /// Create a new branch
    Create(BranchCreateArgs),
    /// Delete a branch
    Delete(BranchDeleteArgs),
}

/// Create a new branch
#[derive(ClapArgs)]
pub struct BranchCreateArgs {}

/// Delete a branch
#[derive(ClapArgs)]
pub struct BranchDeleteArgs {}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    match args.command {
        BranchCommand::Create(a) => create(api, a).await,
        BranchCommand::Delete(a) => delete(api, a).await,
    }
}

async fn create(_api: &HfApi, _args: BranchCreateArgs) -> Result<CommandResult> {
    Ok(CommandResult::Silent)
}

async fn delete(_api: &HfApi, _args: BranchDeleteArgs) -> Result<CommandResult> {
    Ok(CommandResult::Silent)
}
