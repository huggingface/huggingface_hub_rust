use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{HfApi, PaperInfoParams};
use serde_json::json;

use crate::cli::OutputFormat;
use crate::output::{CommandOutput, CommandResult};

/// Show details of a paper
#[derive(ClapArgs)]
pub struct Args {
    /// Paper ID (arXiv ID)
    pub paper_id: String,

    /// Output format
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let params = PaperInfoParams {
        paper_id: args.paper_id,
    };
    let paper = api.paper_info(&params).await?;
    let authors: Vec<_> = paper
        .authors
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .filter_map(|a| a.name.as_deref())
        .collect();
    let json_value = json!({
        "id": paper.id,
        "title": paper.title,
        "summary": paper.summary,
        "authors": authors,
        "published_at": paper.published_at,
        "upvotes": paper.upvotes,
    });
    let output = CommandOutput::single_item(json_value);
    Ok(CommandResult::Formatted {
        output,
        format: args.format,
        quiet: false,
    })
}
