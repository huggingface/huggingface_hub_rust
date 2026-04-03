use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{DeleteCollectionItemParams, HfApi};

use crate::output::CommandResult;

/// Delete an item from a collection
#[derive(ClapArgs)]
pub struct Args {
    /// Collection slug
    pub slug: String,

    /// Item object ID (internal _id field)
    pub item_object_id: String,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let params = DeleteCollectionItemParams {
        slug: args.slug,
        item_object_id: args.item_object_id,
    };
    api.delete_collection_item(&params).await?;
    Ok(CommandResult::Silent)
}
