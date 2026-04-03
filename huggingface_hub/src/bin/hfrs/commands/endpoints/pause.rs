use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{HfApi, PauseInferenceEndpointParams};

use crate::output::CommandResult;

/// Pause an inference endpoint
#[derive(ClapArgs)]
pub struct Args {
    /// Endpoint name
    pub name: String,

    /// Namespace (user or organization)
    #[arg(long)]
    pub namespace: Option<String>,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let params = PauseInferenceEndpointParams {
        name: args.name,
        namespace: args.namespace,
    };
    api.pause_inference_endpoint(&params).await?;
    Ok(CommandResult::Silent)
}
