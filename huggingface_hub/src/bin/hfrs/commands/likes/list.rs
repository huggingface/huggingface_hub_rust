use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{HfApi, ListLikedReposParams};
use serde_json::json;

use crate::cli::OutputFormat;
use crate::output::{CommandOutput, CommandResult};

/// List liked repositories
#[derive(ClapArgs)]
pub struct Args {
    /// Username to list likes for
    pub username: String,

    /// Output format
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,

    /// Print only repo names
    #[arg(long)]
    pub quiet: bool,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let params = ListLikedReposParams {
        username: args.username,
    };
    let liked = api.list_liked_repos(&params).await?;

    let headers = vec!["Repo".to_string(), "Type".to_string(), "Created".to_string()];

    let rows = liked
        .iter()
        .map(|l| {
            let name = l.repo.as_ref().and_then(|r| r.name.as_deref()).unwrap_or_default().to_string();
            let repo_type = l
                .repo
                .as_ref()
                .and_then(|r| r.repo_type.as_deref())
                .unwrap_or_default()
                .to_string();
            let created = l.created_at.as_deref().unwrap_or_default().to_string();
            vec![name, repo_type, created]
        })
        .collect();

    let quiet_values = liked
        .iter()
        .filter_map(|l| l.repo.as_ref().and_then(|r| r.name.clone()))
        .collect();

    let json_value: serde_json::Value = liked
        .iter()
        .map(|l| {
            json!({
                "repo": l.repo.as_ref().and_then(|r| r.name.as_deref()),
                "type": l.repo.as_ref().and_then(|r| r.repo_type.as_deref()),
                "created_at": l.created_at,
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
