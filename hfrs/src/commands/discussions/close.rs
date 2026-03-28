use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{ChangeDiscussionStatusParams, HfApi};

use crate::cli::RepoTypeArg;
use crate::output::CommandResult;

/// Close a discussion or pull request
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
    let params = ChangeDiscussionStatusParams {
        repo_id: args.repo_id,
        discussion_num: args.num,
        new_status: "closed".to_string(),
        repo_type: args.r#type.map(Into::into),
    };
    api.change_discussion_status(&params).await?;
    Ok(CommandResult::Silent)
}
