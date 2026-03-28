use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{AddCollectionItemParams, HfApi};

use crate::output::CommandResult;

/// Add an item to a collection
#[derive(ClapArgs)]
pub struct Args {
    /// Collection slug
    pub slug: String,

    /// Item ID (repo ID or paper ID)
    pub item_id: String,

    /// Item type (model, dataset, space, paper)
    pub item_type: String,

    /// Optional note about the item
    #[arg(long)]
    pub note: Option<String>,

    /// Do not fail if the item already exists in the collection
    #[arg(long)]
    pub exists_ok: bool,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let params = AddCollectionItemParams {
        slug: args.slug,
        item_id: args.item_id,
        item_type: args.item_type,
        note: args.note,
        exists_ok: args.exists_ok,
    };
    let c = api.add_collection_item(&params).await?;
    Ok(CommandResult::Raw(c.slug))
}
