use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{GetInferenceEndpointParams, HfApi};
use serde_json::json;

use crate::cli::OutputFormat;
use crate::output::{CommandOutput, CommandResult};

/// Show details of an inference endpoint
#[derive(ClapArgs)]
pub struct Args {
    /// Endpoint name
    pub name: String,

    /// Namespace (user or organization)
    #[arg(long)]
    pub namespace: Option<String>,

    /// Output format
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let params = GetInferenceEndpointParams {
        name: args.name,
        namespace: args.namespace,
    };
    let e = api.get_inference_endpoint(&params).await?;
    let json_value = json!({
        "name": e.name,
        "namespace": e.namespace,
        "status": e.status.state,
        "status_message": e.status.message,
        "url": e.url,
        "endpoint_type": e.endpoint_type,
        "repository": e.model.as_ref().and_then(|m| m.repository.as_deref()),
        "framework": e.model.as_ref().and_then(|m| m.framework.as_deref()),
        "task": e.model.as_ref().and_then(|m| m.task.as_deref()),
        "vendor": e.provider.as_ref().and_then(|p| p.vendor.as_deref()),
        "region": e.provider.as_ref().and_then(|p| p.region.as_deref()),
        "created_at": e.created_at,
        "updated_at": e.updated_at,
    });
    let output = CommandOutput::single_item(json_value);
    Ok(CommandResult::Formatted {
        output,
        format: args.format,
        quiet: false,
    })
}
