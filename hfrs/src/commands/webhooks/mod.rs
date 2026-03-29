pub mod create;
pub mod delete;
pub mod disable;
pub mod enable;
pub mod info;
pub mod list;
pub mod update;

use anyhow::Result;
use clap::{Args as ClapArgs, Subcommand};
use huggingface_hub::HfApi;

use crate::output::CommandResult;

/// Manage webhooks
#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    pub command: WebhooksCommand,
}

/// Webhooks subcommands
#[derive(Subcommand)]
pub enum WebhooksCommand {
    /// List webhooks
    List(list::Args),
    /// Show details of a webhook
    Info(info::Args),
    /// Create a new webhook
    Create(create::Args),
    /// Update an existing webhook
    Update(update::Args),
    /// Delete a webhook
    Delete(delete::Args),
    /// Enable a webhook
    Enable(enable::Args),
    /// Disable a webhook
    Disable(disable::Args),
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    match args.command {
        WebhooksCommand::List(a) => list::execute(api, a).await,
        WebhooksCommand::Info(a) => info::execute(api, a).await,
        WebhooksCommand::Create(a) => create::execute(api, a).await,
        WebhooksCommand::Update(a) => update::execute(api, a).await,
        WebhooksCommand::Delete(a) => delete::execute(api, a).await,
        WebhooksCommand::Enable(a) => enable::execute(api, a).await,
        WebhooksCommand::Disable(a) => disable::execute(api, a).await,
    }
}
