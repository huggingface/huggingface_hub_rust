use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::HfApi;

use crate::output::CommandResult;

/// Resume a suspended scheduled job
#[derive(ClapArgs)]
pub struct Args {
    /// Scheduled job ID
    pub id: String,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    api.resume_scheduled_job(&args.id).await?;
    Ok(CommandResult::Silent)
}
