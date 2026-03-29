use anyhow::Result;
use clap::{Args as ClapArgs, Subcommand};
use huggingface_hub::{CreateBranchParams, DeleteBranchParams, HfApi};

use crate::cli::RepoTypeArg;
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
pub struct BranchCreateArgs {
    /// Repository ID
    pub repo_id: String,

    /// Branch name to create
    pub branch: String,

    /// Starting revision (branch, tag, or commit SHA)
    #[arg(long)]
    pub revision: Option<String>,

    /// Repository type
    #[arg(long, value_enum, default_value = "model")]
    pub r#type: RepoTypeArg,
}

/// Delete a branch
#[derive(ClapArgs)]
pub struct BranchDeleteArgs {
    /// Repository ID
    pub repo_id: String,

    /// Branch name to delete
    pub branch: String,

    /// Repository type
    #[arg(long, value_enum, default_value = "model")]
    pub r#type: RepoTypeArg,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    match args.command {
        BranchCommand::Create(a) => create(api, a).await,
        BranchCommand::Delete(a) => delete(api, a).await,
    }
}

async fn create(api: &HfApi, args: BranchCreateArgs) -> Result<CommandResult> {
    let repo_type: huggingface_hub::RepoType = args.r#type.into();
    let params = CreateBranchParams {
        repo_id: args.repo_id,
        branch: args.branch,
        revision: args.revision,
        repo_type: Some(repo_type),
    };
    api.create_branch(&params).await?;
    Ok(CommandResult::Raw("Branch created.".to_string()))
}

async fn delete(api: &HfApi, args: BranchDeleteArgs) -> Result<CommandResult> {
    let repo_type: huggingface_hub::RepoType = args.r#type.into();
    let params = DeleteBranchParams {
        repo_id: args.repo_id,
        branch: args.branch,
        repo_type: Some(repo_type),
    };
    api.delete_branch(&params).await?;
    Ok(CommandResult::Silent)
}
