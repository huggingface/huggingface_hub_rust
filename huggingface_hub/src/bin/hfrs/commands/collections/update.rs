use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{HfApi, UpdateCollectionMetadataParams};

use crate::output::CommandResult;

/// Update collection metadata
#[derive(ClapArgs)]
pub struct Args {
    /// Collection slug
    pub slug: String,

    /// New title
    #[arg(long)]
    pub title: Option<String>,

    /// New description
    #[arg(long)]
    pub description: Option<String>,

    /// Set private (true/false)
    #[arg(long)]
    pub private: Option<bool>,

    /// New theme
    #[arg(long)]
    pub theme: Option<String>,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let params = UpdateCollectionMetadataParams {
        slug: args.slug,
        title: args.title,
        description: args.description,
        private: args.private,
        position: None,
        theme: args.theme,
    };
    api.update_collection_metadata(&params).await?;
    Ok(CommandResult::Silent)
}
