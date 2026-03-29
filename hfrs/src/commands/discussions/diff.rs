use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{GetDiscussionDetailsParams, HfApi};

use crate::cli::RepoTypeArg;
use crate::output::CommandResult;

/// Show the diff for a pull request
#[derive(ClapArgs)]
pub struct Args {
    /// Repository ID (e.g. username/my-model)
    pub repo_id: String,

    /// Discussion number
    pub num: u64,

    /// Repository type
    #[arg(long, value_enum)]
    pub r#type: Option<RepoTypeArg>,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let params = GetDiscussionDetailsParams {
        repo_id: args.repo_id,
        discussion_num: args.num,
        repo_type: args.r#type.map(Into::into),
    };
    let d = api.get_discussion_details(&params).await?;
    Ok(CommandResult::Raw(d.diff.unwrap_or_default()))
}
