use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{HfApi, ScaleToZeroInferenceEndpointParams};

use crate::output::CommandResult;

/// Scale an inference endpoint to zero replicas
#[derive(ClapArgs)]
pub struct Args {
    /// Endpoint name
    pub name: String,

    /// Namespace (user or organization)
    #[arg(long)]
    pub namespace: Option<String>,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let params = ScaleToZeroInferenceEndpointParams {
        name: args.name,
        namespace: args.namespace,
    };
    api.scale_to_zero_inference_endpoint(&params).await?;
    Ok(CommandResult::Silent)
}
