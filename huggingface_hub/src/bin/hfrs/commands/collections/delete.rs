use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{DeleteCollectionParams, HfApi};

use crate::output::CommandResult;

/// Delete a collection
#[derive(ClapArgs)]
pub struct Args {
    /// Collection slug
    pub slug: String,

    /// Do not fail if the collection does not exist
    #[arg(long)]
    pub missing_ok: bool,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let params = DeleteCollectionParams {
        slug: args.slug,
        missing_ok: args.missing_ok,
    };
    api.delete_collection(&params).await?;
    Ok(CommandResult::Silent)
}
