use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::HFClient;
use serde_json::json;

use crate::cli::OutputFormat;
use crate::output::{CommandOutput, CommandResult};

/// Show details of a scheduled job
#[derive(ClapArgs)]
pub struct Args {
    /// Scheduled job ID
    pub id: String,

    /// Namespace (defaults to current user)
    #[arg(long)]
    pub namespace: Option<String>,

    /// Output format
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,
}

pub async fn execute(api: &HFClient, args: Args) -> Result<CommandResult> {
    let job = api.inspect_scheduled_job(&args.id, args.namespace.as_deref()).await?;
    let json_value = json!({
        "id": job.id,
        "image": job.docker_image,
        "command": job.command,
        "schedule": job.schedule,
        "flavor": job.flavor,
        "suspended": job.suspended,
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
