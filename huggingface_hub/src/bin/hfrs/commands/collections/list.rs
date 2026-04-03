use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{HfApi, ListCollectionsParams};
use serde_json::json;

use crate::cli::OutputFormat;
use crate::output::{CommandOutput, CommandResult};

/// List collections
#[derive(ClapArgs)]
pub struct Args {
    /// Filter by owner
    #[arg(long)]
    pub owner: Option<String>,

    /// Filter by item ID
    #[arg(long)]
    pub item: Option<String>,

    /// Sort field
    #[arg(long)]
    pub sort: Option<String>,

    /// Maximum number of results
    #[arg(long, default_value = "10")]
    pub limit: usize,

    /// Output format
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,

    /// Print only slugs
    #[arg(long)]
    pub quiet: bool,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let params = ListCollectionsParams {
        owner: args.owner,
        item: args.item,
        item_type: None,
        limit: Some(args.limit),
        offset: None,
    };
    let collections = api.list_collections(&params).await?;

    let headers = vec![
        "Slug".to_string(),
        "Title".to_string(),
        "Items".to_string(),
        "Upvotes".to_string(),
    ];

    let rows = collections
        .iter()
        .map(|c| {
            vec![
                c.slug.clone(),
                c.title.clone().unwrap_or_default(),
                c.items.len().to_string(),
                c.upvotes.map(|u| u.to_string()).unwrap_or_default(),
            ]
        })
        .collect();

    let quiet_values = collections.iter().map(|c| c.slug.clone()).collect();

    let json_value: serde_json::Value = collections
        .iter()
        .map(|c| {
            json!({
                "slug": c.slug,
                "title": c.title,
                "description": c.description,
                "private": c.private,
                "upvotes": c.upvotes,
                "items_count": c.items.len(),
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
