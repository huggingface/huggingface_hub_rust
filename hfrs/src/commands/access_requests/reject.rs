use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{HandleAccessRequestParams, HfApi};

use crate::cli::RepoTypeArg;
use crate::output::CommandResult;

/// Reject an access request
#[derive(ClapArgs)]
pub struct Args {
    /// Repository ID (e.g. username/my-model)
    pub repo_id: String,

    /// Username to reject
    pub user: String,

    /// Repository type
    #[arg(long = "type", value_enum)]
    pub repo_type: Option<RepoTypeArg>,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let params = HandleAccessRequestParams {
        repo_id: args.repo_id,
        user: args.user,
        repo_type: args.repo_type.map(Into::into),
    };
    api.reject_access_request(&params).await?;
    Ok(CommandResult::Silent)
}
