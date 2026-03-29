use anyhow::Result;
use clap::Args as ClapArgs;
use futures::StreamExt;
use huggingface_hub::{HfApi, ListSpacesParams};
use serde_json::json;

use crate::cli::OutputFormat;
use crate::output::{CommandOutput, CommandResult};

/// List Spaces on the Hub
#[derive(ClapArgs)]
pub struct Args {
    /// Search query
    #[arg(long)]
    pub search: Option<String>,

    /// Filter by author/organization
    #[arg(long)]
    pub author: Option<String>,

    /// Filter tags (can be specified multiple times)
    #[arg(long)]
    pub filter: Vec<String>,

    /// Sort field
    #[arg(long)]
    pub sort: Option<String>,

    /// Maximum number of results
    #[arg(long, default_value = "10")]
    pub limit: usize,

    /// Output format
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,

    /// Print only Space IDs
    #[arg(long)]
    pub quiet: bool,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let filter = if args.filter.is_empty() {
        None
    } else {
        Some(args.filter.join(","))
    };

    let params = ListSpacesParams {
        search: args.search,
        author: args.author,
        filter,
        sort: args.sort,
        limit: Some(args.limit),
        full: None,
    };

    let stream = api.list_spaces(&params);
    futures::pin_mut!(stream);

    let mut spaces = Vec::new();
    while let Some(item) = stream.next().await {
        spaces.push(item?);
        if spaces.len() >= args.limit {
            break;
        }
    }

    let headers = vec![
        "ID".to_string(),
        "Author".to_string(),
        "SDK".to_string(),
        "Likes".to_string(),
    ];

    let rows = spaces
        .iter()
        .map(|s| {
            vec![
                s.id.clone(),
                s.author.clone().unwrap_or_default(),
                s.sdk.clone().unwrap_or_default(),
                s.likes.map(|v| v.to_string()).unwrap_or_default(),
            ]
        })
        .collect();

    let quiet_values = spaces.iter().map(|s| s.id.clone()).collect();

    let json_value: serde_json::Value = spaces
        .iter()
        .map(|s| {
            json!({
                "id": s.id,
                "author": s.author,
                "sdk": s.sdk,
                "likes": s.likes,
                "tags": s.tags,
                "trending_score": s.trending_score,
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
