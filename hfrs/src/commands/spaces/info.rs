use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{HfApi, RepoInfo, RepoInfoParams};
use serde_json::json;

use crate::cli::OutputFormat;
use crate::output::{CommandOutput, CommandResult};

/// Show detailed information about a Space
#[derive(ClapArgs)]
pub struct Args {
    /// Space ID (e.g. gradio/hello_world)
    pub space_id: String,

    /// Git revision (branch, tag, or commit SHA)
    #[arg(long)]
    pub revision: Option<String>,

    /// Output format
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let (owner, name) = crate::util::split_repo_id(&args.space_id);
    let repo = api.space(owner, name);
    let info_params = RepoInfoParams {
        revision: args.revision,
    };
    let repo_info = repo.info(&info_params).await?;
    let info = match repo_info {
        RepoInfo::Space(s) => s,
        _ => anyhow::bail!("Expected space info"),
    };
    let json_value = json!({
        "id": info.id,
        "author": info.author,
        "sha": info.sha,
        "private": info.private,
        "sdk": info.sdk,
        "likes": info.likes,
        "tags": info.tags,
        "created_at": info.created_at,
        "last_modified": info.last_modified,
        "trending_score": info.trending_score,
    });
    let output = CommandOutput::single_item(json_value);
    Ok(CommandResult::Formatted {
        output,
        format: args.format,
        quiet: false,
    })
}
