use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::HfApi;
use serde_json::json;

use crate::cli::OutputFormat;
use crate::output::{CommandOutput, CommandResult};

/// Show details of a job
#[derive(ClapArgs)]
pub struct Args {
    /// Job ID
    pub job_id: String,

    /// Namespace (user or organization)
    #[arg(long)]
    pub namespace: Option<String>,

    /// Output format
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let job = api.inspect_job(&args.job_id, args.namespace.as_deref()).await?;
    let json_value = json!({
        "id": job.id,
        "image": job.docker_image,
        "command": job.command,
        "flavor": job.flavor,
        "status": job.status.as_ref().and_then(|s| s.stage.as_deref()),
        "status_message": job.status.as_ref().and_then(|s| s.message.as_deref()),
        "url": job.url,
        "created_at": job.created_at,
    });
    let output = CommandOutput::single_item(json_value);
    Ok(CommandResult::Formatted {
        output,
        format: args.format,
        quiet: false,
    })
}
