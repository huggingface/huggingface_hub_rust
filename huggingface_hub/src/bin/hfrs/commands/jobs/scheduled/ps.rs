use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::HfApi;
use serde_json::json;

use crate::cli::OutputFormat;
use crate::output::{CommandOutput, CommandResult};

/// List scheduled jobs
#[derive(ClapArgs)]
pub struct Args {
    /// Output format
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,

    /// Print only job IDs
    #[arg(long)]
    pub quiet: bool,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let jobs = api.list_scheduled_jobs().await?;

    if jobs.is_empty() && matches!(args.format, OutputFormat::Table) {
        return Ok(CommandResult::Raw("No scheduled jobs found.".to_string()));
    }

    let headers = vec![
        "ID".to_string(),
        "Schedule".to_string(),
        "Image".to_string(),
        "Suspended".to_string(),
    ];

    let rows = jobs
        .iter()
        .map(|j| {
            vec![
                j.id.clone(),
                j.schedule.clone().unwrap_or_default(),
                j.docker_image.clone().unwrap_or_default(),
                j.suspended.map(|s| s.to_string()).unwrap_or_default(),
            ]
        })
        .collect();

    let quiet_values = jobs.iter().map(|j| j.id.clone()).collect();

    let json_value: serde_json::Value = jobs
        .iter()
        .map(|j| {
            json!({
                "id": j.id,
                "schedule": j.schedule,
                "image": j.docker_image,
                "suspended": j.suspended,
            })
        })
        .collect::<Vec<_>>()
        .into();

    let output = CommandOutput {
        headers,
        rows,
        json_value,
        quiet_values,
    };
    Ok(CommandResult::Formatted {
        output,
        format: args.format,
        quiet: args.quiet,
    })
}
