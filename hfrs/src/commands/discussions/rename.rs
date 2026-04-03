use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{HfApi, RepoRenameDiscussionParams};

use crate::cli::RepoTypeArg;
use crate::output::CommandResult;

/// Rename a discussion or pull request
#[derive(ClapArgs)]
pub struct Args {
    /// Repository ID (e.g. username/my-model)
    pub repo_id: String,

    /// Discussion number
    pub num: u64,

    /// New title
    pub new_title: String,

    /// Repository type
    #[arg(long, value_enum)]
    pub r#type: Option<RepoTypeArg>,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let repo_type = args.r#type.map(Into::into).unwrap_or(huggingface_hub::RepoType::Model);
    let repo = crate::util::make_repo(api, &args.repo_id, repo_type);
    let params = RepoRenameDiscussionParams {
        discussion_num: args.num,
        new_title: args.new_title,
    };
    repo.rename_discussion(&params).await?;
    Ok(CommandResult::Silent)
}
