use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{CreateDiscussionParams, CreatePullRequestParams, HfApi};

use crate::cli::RepoTypeArg;
use crate::output::CommandResult;

/// Create a new discussion or pull request
#[derive(ClapArgs)]
pub struct Args {
    /// Repository ID (e.g. username/my-model)
    pub repo_id: String,

    /// Title of the discussion or pull request
    #[arg(long, required = true)]
    pub title: String,

    /// Body/description
    #[arg(long)]
    pub body: Option<String>,

    /// Create a pull request instead of a discussion
    #[arg(long)]
    pub pull_request: bool,

    /// Repository type
    #[arg(long, value_enum)]
    pub r#type: Option<RepoTypeArg>,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let repo_type = args.r#type.map(Into::into);
    let num = if args.pull_request {
        let params = CreatePullRequestParams {
            repo_id: args.repo_id,
            title: args.title,
            description: args.body,
            repo_type,
        };
        let d = api.create_pull_request(&params).await?;
        d.num
    } else {
        let params = CreateDiscussionParams {
            repo_id: args.repo_id,
            title: args.title,
            description: args.body,
            repo_type,
        };
        let d = api.create_discussion(&params).await?;
        d.num
    };
    Ok(CommandResult::Raw(num.to_string()))
}
