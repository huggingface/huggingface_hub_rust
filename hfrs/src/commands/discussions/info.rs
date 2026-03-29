use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{GetDiscussionDetailsParams, HfApi};
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
    let params = GetDiscussionDetailsParams {
        repo_id: args.repo_id,
        discussion_num: args.num,
        repo_type: args.r#type.map(Into::into),
    };
    let d = api.get_discussion_details(&params).await?;
    let json_value = json!({
        "num": d.num,
        "title": d.title,
        "status": d.status,
        "is_pull_request": d.is_pull_request,
        "author": d.author,
        "created_at": d.created_at,
        "target_branch": d.target_branch,
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
