use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{CreateCollectionParams, HfApi};

use crate::output::CommandResult;

/// Create a new collection
#[derive(ClapArgs)]
pub struct Args {
    /// Collection title
    pub title: String,

    /// Namespace (user or organization)
    #[arg(long)]
    pub namespace: Option<String>,

    /// Description
    #[arg(long)]
    pub description: Option<String>,

    /// Make the collection private
    #[arg(long)]
    pub private: bool,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let params = CreateCollectionParams {
        title: args.title,
        description: args.description,
        private: if args.private { Some(true) } else { None },
        namespace: args.namespace,
    };
    let c = api.create_collection(&params).await?;
    Ok(CommandResult::Raw(c.slug))
}
