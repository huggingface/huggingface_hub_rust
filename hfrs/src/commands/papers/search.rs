use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{HfApi, ListPapersParams};
use serde_json::json;

use crate::cli::OutputFormat;
use crate::output::{CommandOutput, CommandResult};

/// Search for papers
#[derive(ClapArgs)]
pub struct Args {
    /// Search query
    pub query: String,

    /// Maximum number of results
    #[arg(long, default_value = "20")]
    pub limit: usize,

    /// Output format
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,

    /// Print only paper IDs
    #[arg(long)]
    pub quiet: bool,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let params = ListPapersParams {
        query: Some(args.query),
        limit: Some(args.limit),
    };
    let results = api.list_papers(&params).await?;

    let headers = vec!["Title".to_string(), "Paper ID".to_string()];

    let rows = results
        .iter()
        .map(|r| {
            let title = r
                .title
                .as_deref()
                .or_else(|| r.paper.as_ref().and_then(|p| p.title.as_deref()))
                .unwrap_or_default()
                .to_string();
            let id = r.paper.as_ref().map(|p| p.id.as_str()).unwrap_or_default().to_string();
            vec![title, id]
        })
        .collect();

    let quiet_values = results.iter().filter_map(|r| r.paper.as_ref().map(|p| p.id.clone())).collect();

    let json_value: serde_json::Value = results
        .iter()
        .map(|r| {
            json!({
                "id": r.paper.as_ref().map(|p| p.id.as_str()),
                "title": r.title.as_deref().or_else(|| r.paper.as_ref().and_then(|p| p.title.as_deref())),
                "published_at": r.published_at,
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
