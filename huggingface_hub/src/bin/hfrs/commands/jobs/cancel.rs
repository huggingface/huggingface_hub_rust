use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::HfApi;

use crate::output::CommandResult;

/// Cancel a running job
#[derive(ClapArgs)]
pub struct Args {
    /// Job ID
    pub job_id: String,

    /// Namespace (user or organization)
    #[arg(long)]
    pub namespace: Option<String>,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    api.cancel_job(&args.job_id, args.namespace.as_deref()).await?;
    Ok(CommandResult::Silent)
}
