use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{HfApi, ListInferenceEndpointsParams};
use serde_json::json;

use crate::cli::OutputFormat;
use crate::output::{CommandOutput, CommandResult};

/// List inference endpoints
#[derive(ClapArgs)]
pub struct Args {
    /// Namespace (user or organization)
    #[arg(long)]
    pub namespace: Option<String>,

    /// Output format
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,

    /// Print only endpoint names
    #[arg(long)]
    pub quiet: bool,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let params = ListInferenceEndpointsParams {
        namespace: args.namespace,
    };
    let endpoints = api.list_inference_endpoints(&params).await?;

    let headers = vec!["Name".to_string(), "Status".to_string(), "URL".to_string()];

    let rows = endpoints
        .iter()
        .map(|e| {
            vec![
                e.name.clone(),
                e.status.state.clone().unwrap_or_default(),
                e.url.clone().unwrap_or_default(),
            ]
        })
        .collect();

    let quiet_values = endpoints.iter().map(|e| e.name.clone()).collect();

    let json_value: serde_json::Value = endpoints
        .iter()
        .map(|e| {
            json!({
                "name": e.name,
                "namespace": e.namespace,
                "status": e.status.state,
                "url": e.url,
                "endpoint_type": e.endpoint_type,
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
