use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{HfApi, UpdateCollectionItemParams};

use crate::output::CommandResult;

/// Update an item in a collection
#[derive(ClapArgs)]
pub struct Args {
    /// Collection slug
    pub slug: String,

    /// Item object ID (internal _id field)
    pub item_object_id: String,

    /// New note
    #[arg(long)]
    pub note: Option<String>,

    /// New position
    #[arg(long)]
    pub position: Option<i64>,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let params = UpdateCollectionItemParams {
        slug: args.slug,
        item_object_id: args.item_object_id,
        note: args.note,
        position: args.position,
    };
    api.update_collection_item(&params).await?;
    Ok(CommandResult::Silent)
}
