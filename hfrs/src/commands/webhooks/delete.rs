use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::HfApi;

use crate::output::CommandResult;

/// Delete a webhook
#[derive(ClapArgs)]
pub struct Args {
    /// Webhook ID
    pub webhook_id: String,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    api.delete_webhook(&args.webhook_id).await?;
    Ok(CommandResult::Silent)
}
