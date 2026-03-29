use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{CommentDiscussionParams, HfApi};

use crate::cli::RepoTypeArg;
use crate::output::CommandResult;

/// Add a comment to a discussion
#[derive(ClapArgs)]
pub struct Args {
    /// Repository ID (e.g. username/my-model)
    pub repo_id: String,

    /// Discussion number
    pub num: u64,

    /// Comment body
    #[arg(long, required = true)]
    pub body: String,

    /// Repository type
    #[arg(long, value_enum)]
    pub r#type: Option<RepoTypeArg>,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let params = CommentDiscussionParams {
        repo_id: args.repo_id,
        discussion_num: args.num,
        comment: args.body,
        repo_type: args.r#type.map(Into::into),
    };
    api.comment_discussion(&params).await?;
    Ok(CommandResult::Silent)
}
