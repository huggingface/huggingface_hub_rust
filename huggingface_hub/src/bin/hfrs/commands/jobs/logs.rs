use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::HfApi;

use crate::output::CommandResult;

/// Stream or fetch logs for a job
#[derive(ClapArgs)]
pub struct Args {
    /// Job ID
    pub job_id: String,

    /// Namespace (user or organization)
    #[arg(long)]
    pub namespace: Option<String>,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let entries = api.fetch_job_logs(&args.job_id, args.namespace.as_deref()).await?;
    let output = entries
        .iter()
        .map(|e| {
            let ts = e.timestamp.as_deref().unwrap_or("");
            let data = e.data.as_deref().unwrap_or("");
            format!("{ts} {data}")
        })
        .collect::<Vec<_>>()
        .join("\n");
    Ok(CommandResult::Raw(output))
}
