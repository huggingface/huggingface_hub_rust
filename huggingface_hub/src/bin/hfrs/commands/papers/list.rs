use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{HfApi, ListDailyPapersParams};
use serde_json::json;

use crate::cli::OutputFormat;
use crate::output::{CommandOutput, CommandResult};

/// List daily papers
#[derive(ClapArgs)]
pub struct Args {
    /// Filter by date (YYYY-MM-DD)
    #[arg(long)]
    pub date: Option<String>,

    /// Filter by week
    #[arg(long)]
    pub week: Option<String>,

    /// Filter by month
    #[arg(long)]
    pub month: Option<String>,

    /// Filter by submitter
    #[arg(long)]
    pub submitter: Option<String>,

    /// Sort order
    #[arg(long)]
    pub sort: Option<String>,

    /// Maximum number of results
    #[arg(long, default_value = "50")]
    pub limit: usize,

    /// Output format
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,

    /// Print only paper IDs
    #[arg(long)]
    pub quiet: bool,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let params = ListDailyPapersParams {
        date: args.date,
        week: args.week,
        month: args.month,
        submitter: args.submitter,
        sort: args.sort,
        p: None,
        limit: Some(args.limit),
    };
    let papers = api.list_daily_papers(&params).await?;

    if papers.is_empty() && matches!(args.format, OutputFormat::Table) {
        return Ok(CommandResult::Raw("No papers found.".to_string()));
    }

    let headers = vec!["Title".to_string(), "Paper ID".to_string(), "Upvotes".to_string()];

    let rows = papers
        .iter()
        .map(|p| {
            let title = p
                .title
                .as_deref()
                .or_else(|| p.paper.as_ref().and_then(|pi| pi.title.as_deref()))
                .unwrap_or_default()
                .to_string();
            let id = p.paper.as_ref().map(|pi| pi.id.as_str()).unwrap_or_default().to_string();
            let upvotes = p.upvotes.map(|u| u.to_string()).unwrap_or_default();
            vec![title, id, upvotes]
        })
        .collect();

    let quiet_values = papers.iter().filter_map(|p| p.paper.as_ref().map(|pi| pi.id.clone())).collect();

    let json_value: serde_json::Value = papers
        .iter()
        .map(|p| {
            json!({
                "id": p.paper.as_ref().map(|pi| pi.id.as_str()),
                "title": p.title.as_deref().or_else(|| p.paper.as_ref().and_then(|pi| pi.title.as_deref())),
                "upvotes": p.upvotes,
                "published_at": p.published_at,
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
