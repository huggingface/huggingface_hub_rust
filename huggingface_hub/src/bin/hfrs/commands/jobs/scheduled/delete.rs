use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::HfApi;

use crate::output::CommandResult;

/// Delete a scheduled job
#[derive(ClapArgs)]
pub struct Args {
    /// Scheduled job ID
    pub id: String,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    api.delete_scheduled_job(&args.id).await?;
    Ok(CommandResult::Silent)
}
