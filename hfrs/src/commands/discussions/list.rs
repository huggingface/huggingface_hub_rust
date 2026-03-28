use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{GetRepoDiscussionsParams, HfApi};
use serde_json::json;

use crate::cli::{OutputFormat, RepoTypeArg};
use crate::output::{CommandOutput, CommandResult};

/// List discussions and pull requests for a repository
#[derive(ClapArgs)]
pub struct Args {
    /// Repository ID (e.g. username/my-model)
    pub repo_id: String,

    /// Filter by status (open, closed)
    #[arg(long)]
    pub status: Option<String>,

    /// Filter by kind (discussion, pull_request)
    #[arg(long)]
    pub kind: Option<String>,

    /// Filter by author
    #[arg(long)]
    pub author: Option<String>,

    /// Repository type
    #[arg(long, value_enum)]
    pub r#type: Option<RepoTypeArg>,

    /// Output format
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,

    /// Print only discussion numbers
    #[arg(long)]
    pub quiet: bool,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let params = GetRepoDiscussionsParams {
        repo_id: args.repo_id,
        repo_type: args.r#type.map(Into::into),
        author: args.author,
        discussion_type: args.kind,
        discussion_status: args.status,
    };
    let resp = api.get_repo_discussions(&params).await?;

    let headers = vec![
        "Num".to_string(),
        "Title".to_string(),
        "Status".to_string(),
        "PR".to_string(),
    ];

    let rows = resp
        .discussions
        .iter()
        .map(|d| {
            vec![
                d.num.to_string(),
                d.title.clone().unwrap_or_default(),
                d.status.clone().unwrap_or_default(),
                d.is_pull_request
                    .map(|b| if b { "yes" } else { "no" }.to_string())
                    .unwrap_or_default(),
            ]
        })
        .collect();

    let quiet_values = resp.discussions.iter().map(|d| d.num.to_string()).collect();

    let json_value: serde_json::Value = resp
        .discussions
        .iter()
        .map(|d| {
            json!({
                "num": d.num,
                "title": d.title,
                "status": d.status,
                "is_pull_request": d.is_pull_request,
                "author": d.author,
                "created_at": d.created_at,
                "num_comments": d.num_comments,
            })
        })
        .collect::<Vec<_>>()
        .into();

    let output = CommandOutput {
        headers,
        rows,
        json_value,
        quiet_values,
    };
    Ok(CommandResult::Formatted {
        output,
        format: args.format,
        quiet: args.quiet,
    })
}
