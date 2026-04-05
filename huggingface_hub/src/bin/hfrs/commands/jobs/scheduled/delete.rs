use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::HFClient;

use crate::output::CommandResult;

/// Delete a scheduled job
#[derive(ClapArgs)]
pub struct Args {
    /// Scheduled job ID
    pub id: String,

    /// Namespace (defaults to current user)
    #[arg(long)]
    pub namespace: Option<String>,
}

pub async fn execute(api: &HFClient, args: Args) -> Result<CommandResult> {
    api.delete_scheduled_job(&args.id, args.namespace.as_deref()).await?;
    Ok(CommandResult::Silent)
}
