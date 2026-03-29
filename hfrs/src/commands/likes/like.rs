use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{HfApi, LikeParams};

use crate::cli::RepoTypeArg;
use crate::output::CommandResult;

/// Like a repository
#[derive(ClapArgs)]
pub struct Args {
    /// Repository ID (e.g. username/my-model)
    pub repo_id: String,

    /// Repository type
    #[arg(long = "type", value_enum)]
    pub repo_type: Option<RepoTypeArg>,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let params = LikeParams {
        repo_id: args.repo_id,
        repo_type: args.repo_type.map(Into::into),
    };
    api.like(&params).await?;
    Ok(CommandResult::Silent)
}
