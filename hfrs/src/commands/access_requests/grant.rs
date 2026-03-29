use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{GrantAccessParams, HfApi};

use crate::cli::RepoTypeArg;
use crate::output::CommandResult;

/// Grant access to a user for a gated repository
#[derive(ClapArgs)]
pub struct Args {
    /// Repository ID (e.g. username/my-model)
    pub repo_id: String,

    /// Username to grant access to
    pub user: String,

    /// Repository type
    #[arg(long = "type", value_enum)]
    pub repo_type: Option<RepoTypeArg>,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let params = GrantAccessParams {
        repo_id: args.repo_id,
        user: args.user,
        repo_type: args.repo_type.map(Into::into),
    };
    api.grant_access(&params).await?;
    Ok(CommandResult::Silent)
}
