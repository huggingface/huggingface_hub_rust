pub mod add_item;
pub mod create;
pub mod delete;
pub mod delete_item;
pub mod info;
pub mod list;
pub mod update;
pub mod update_item;

use anyhow::Result;
use clap::{Args as ClapArgs, Subcommand};
use huggingface_hub::HfApi;

use crate::output::CommandResult;

/// Manage collections
#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    pub command: CollectionsCommand,
}

/// Collections subcommands
#[derive(Subcommand)]
pub enum CollectionsCommand {
    /// Show details of a collection
    Info(info::Args),
    /// List collections
    List(list::Args),
    /// Create a new collection
    Create(create::Args),
    /// Delete a collection
    Delete(delete::Args),
    /// Update collection metadata
    Update(update::Args),
    /// Add an item to a collection
    AddItem(add_item::Args),
    /// Update an item in a collection
    UpdateItem(update_item::Args),
    /// Delete an item from a collection
    DeleteItem(delete_item::Args),
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    match args.command {
        CollectionsCommand::Info(a) => info::execute(api, a).await,
        CollectionsCommand::List(a) => list::execute(api, a).await,
        CollectionsCommand::Create(a) => create::execute(api, a).await,
        CollectionsCommand::Delete(a) => delete::execute(api, a).await,
        CollectionsCommand::Update(a) => update::execute(api, a).await,
        CollectionsCommand::AddItem(a) => add_item::execute(api, a).await,
        CollectionsCommand::UpdateItem(a) => update_item::execute(api, a).await,
        CollectionsCommand::DeleteItem(a) => delete_item::execute(api, a).await,
    }
}
