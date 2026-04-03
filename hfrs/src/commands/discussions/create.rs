use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{HfApi, RepoCreateDiscussionParams, RepoCreatePullRequestParams};

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
    let repo_type = args.r#type.map(Into::into).unwrap_or(huggingface_hub::RepoType::Model);
    let repo = crate::util::make_repo(api, &args.repo_id, repo_type);
    let num = if args.pull_request {
        let params = RepoCreatePullRequestParams {
            title: args.title,
            description: args.body,
        };
        let d = repo.create_pull_request(&params).await?;
        d.num
    } else {
        let params = RepoCreateDiscussionParams {
            title: args.title,
            description: args.body,
        };
        let d = repo.create_discussion(&params).await?;
        d.num
    };
    match num {
        Some(n) => Ok(CommandResult::Raw(n.to_string())),
        None => anyhow::bail!("Server did not return a discussion number"),
    }
}
