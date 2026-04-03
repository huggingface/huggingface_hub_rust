use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{HfApi, RepoDiscussionDetailsParams};
use serde_json::json;

use crate::cli::{OutputFormat, RepoTypeArg};
use crate::output::{CommandOutput, CommandResult};

/// Show details of a discussion or pull request
#[derive(ClapArgs)]
pub struct Args {
    /// Repository ID (e.g. username/my-model)
    pub repo_id: String,

    /// Discussion number
    pub num: u64,

    /// Repository type
    #[arg(long, value_enum)]
    pub r#type: Option<RepoTypeArg>,

    /// Output format
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let repo_type = args.r#type.map(Into::into).unwrap_or(huggingface_hub::RepoType::Model);
    let repo = crate::util::make_repo(api, &args.repo_id, repo_type);
    let params = RepoDiscussionDetailsParams {
        discussion_num: args.num,
    };
    let d = repo.discussion_details(&params).await?;
    let json_value = json!({
        "num": d.num,
        "title": d.title,
        "status": d.status,
        "is_pull_request": d.is_pull_request,
        "author": d.author,
        "created_at": d.created_at,
        "target_branch": d.changes.as_ref().and_then(|c| c.base.as_deref()),
        "merge_commit_oid": d.merge_commit_oid,
        "events_count": d.events.len(),
    });
    let output = CommandOutput::single_item(json_value);
    Ok(CommandResult::Formatted {
        output,
        format: args.format,
        quiet: false,
    })
}
