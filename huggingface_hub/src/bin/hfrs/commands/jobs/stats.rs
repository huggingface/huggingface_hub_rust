use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::HFClient;
use serde_json::json;

use crate::cli::OutputFormat;
use crate::output::{CommandOutput, CommandResult};

/// Show resource usage statistics for a job
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

pub async fn execute(api: &HFClient, args: Args) -> Result<CommandResult> {
    let metrics = api.fetch_job_metrics(&args.job_id, args.namespace.as_deref()).await?;
    let json_value = if let Some(m) = metrics.first() {
        json!({
            "cpu_usage_pct": m.cpu_usage_pct,
            "cpu_millicores": m.cpu_millicores,
            "memory_used_bytes": m.memory_used_bytes,
            "memory_total_bytes": m.memory_total_bytes,
            "rx_bps": m.rx_bps,
            "tx_bps": m.tx_bps,
        })
    } else {
        json!({})
    };
    let output = CommandOutput::single_item(json_value);
    Ok(CommandResult::Formatted {
        output,
        format: args.format,
        quiet: false,
    })
}
