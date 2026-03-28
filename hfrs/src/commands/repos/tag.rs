use anyhow::Result;
use clap::{Args as ClapArgs, Subcommand};
use huggingface_hub::HfApi;

use crate::output::CommandResult;

/// Manage repository tags
#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    pub command: TagCommand,
}

/// Tag subcommands
#[derive(Subcommand)]
pub enum TagCommand {
    /// Create a new tag
    Create(TagCreateArgs),
    /// Delete a tag
    Delete(TagDeleteArgs),
    /// List tags
    List(TagListArgs),
}

/// Create a new tag
#[derive(ClapArgs)]
pub struct TagCreateArgs {}

/// Delete a tag
#[derive(ClapArgs)]
pub struct TagDeleteArgs {}

/// List tags
#[derive(ClapArgs)]
pub struct TagListArgs {}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    match args.command {
        TagCommand::Create(a) => create(api, a).await,
        TagCommand::Delete(a) => delete(api, a).await,
        TagCommand::List(a) => list(api, a).await,
    }
}

async fn create(_api: &HfApi, _args: TagCreateArgs) -> Result<CommandResult> {
    Ok(CommandResult::Silent)
}

async fn delete(_api: &HfApi, _args: TagDeleteArgs) -> Result<CommandResult> {
    Ok(CommandResult::Silent)
}

async fn list(_api: &HfApi, _args: TagListArgs) -> Result<CommandResult> {
    Ok(CommandResult::Silent)
}
