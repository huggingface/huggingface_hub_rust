use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{HfApi, RepoInfo, RepoInfoParams};
use serde_json::json;

use crate::cli::OutputFormat;
use crate::output::{CommandOutput, CommandResult};

/// Show detailed information about a model
#[derive(ClapArgs)]
pub struct Args {
    /// Model ID (e.g. gpt2 or openai-community/gpt2)
    pub model_id: String,

    /// Git revision (branch, tag, or commit SHA)
    #[arg(long)]
    pub revision: Option<String>,

    /// Output format
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let (owner, name) = crate::util::split_repo_id(&args.model_id);
    let repo = api.model(owner, name);
    let info_params = RepoInfoParams {
        revision: args.revision,
    };
    let repo_info = repo.info(&info_params).await?;
    let info = match repo_info {
        RepoInfo::Model(m) => m,
        _ => anyhow::bail!("Expected model info"),
    };
    let json_value = json!({
        "id": info.id,
        "author": info.author,
        "sha": info.sha,
        "private": info.private,
        "downloads": info.downloads,
        "likes": info.likes,
        "pipeline_tag": info.pipeline_tag,
        "library_name": info.library_name,
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
