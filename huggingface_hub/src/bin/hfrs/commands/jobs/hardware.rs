use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::HfApi;
use serde_json::json;

use crate::cli::OutputFormat;
use crate::output::{CommandOutput, CommandResult};

/// List available hardware flavors for jobs
#[derive(ClapArgs)]
pub struct Args {
    /// Output format
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let hardware = api.list_job_hardware().await?;

    let headers = vec!["Name".to_string(), "CPU".to_string(), "RAM".to_string()];

    let rows = hardware
        .iter()
        .map(|h| {
            vec![
                h.name.clone().unwrap_or_default(),
                h.cpu.clone().unwrap_or_default(),
                h.ram.clone().unwrap_or_default(),
            ]
        })
        .collect();

    let json_value: serde_json::Value = hardware
        .iter()
        .map(|h| {
            json!({
                "name": h.name,
                "pretty_name": h.pretty_name,
                "cpu": h.cpu,
                "ram": h.ram,
            })
        })
        .collect::<Vec<_>>()
        .into();

    let output = CommandOutput {
        headers,
        rows,
        json_value,
        quiet_values: vec![],
    };
    Ok(CommandResult::Formatted {
        output,
        format: args.format,
        quiet: false,
    })
}
