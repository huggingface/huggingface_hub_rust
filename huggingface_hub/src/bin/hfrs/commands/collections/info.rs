use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{GetCollectionParams, HfApi};
use serde_json::json;

use crate::cli::OutputFormat;
use crate::output::{CommandOutput, CommandResult};

/// Show details of a collection
#[derive(ClapArgs)]
pub struct Args {
    /// Collection slug (e.g. username/my-collection-abc123)
    pub slug: String,

    /// Output format
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let params = GetCollectionParams { slug: args.slug };
    let c = api.get_collection(&params).await?;
    let json_value = json!({
        "slug": c.slug,
        "title": c.title,
        "description": c.description,
        "private": c.private,
        "upvotes": c.upvotes,
        "theme": c.theme,
        "last_updated": c.last_updated,
        "items_count": c.items.len(),
    });
    let output = CommandOutput::single_item(json_value);
    Ok(CommandResult::Formatted {
        output,
        format: args.format,
        quiet: false,
    })
}
