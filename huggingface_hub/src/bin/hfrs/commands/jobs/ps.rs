use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{HFClient, ListJobsParams};
use serde_json::json;

use crate::cli::OutputFormat;
use crate::output::{CommandOutput, CommandResult};

/// List running and recent jobs
#[derive(ClapArgs)]
pub struct Args {
    /// Namespace (user or organization)
    #[arg(long)]
    pub namespace: Option<String>,

    /// Output format
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,

    /// Print only job IDs
    #[arg(long)]
    pub quiet: bool,
}

pub async fn execute(api: &HFClient, args: Args) -> Result<CommandResult> {
    let params = ListJobsParams {
        namespace: args.namespace,
    };
    let jobs = api.list_jobs(&params).await?;

    if jobs.is_empty() && matches!(args.format, OutputFormat::Table) {
        return Ok(CommandResult::Raw("No jobs found.".to_string()));
    }

    let headers = vec![
        "ID".to_string(),
        "Image".to_string(),
        "Status".to_string(),
        "Created".to_string(),
    ];

    let rows = jobs
        .iter()
        .map(|j| {
            vec![
                j.id.clone(),
                j.docker_image.clone().unwrap_or_default(),
                j.status.as_ref().and_then(|s| s.stage.clone()).unwrap_or_default(),
                j.created_at.clone().unwrap_or_default(),
            ]
        })
        .collect();

    let quiet_values = jobs.iter().map(|j| j.id.clone()).collect();

    let json_value: serde_json::Value = jobs
        .iter()
        .map(|j| {
            json!({
                "id": j.id,
                "image": j.docker_image,
                "status": j.status.as_ref().and_then(|s| s.stage.as_deref()),
                "created_at": j.created_at,
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
