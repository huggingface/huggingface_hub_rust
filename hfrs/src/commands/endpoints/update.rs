use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{HfApi, UpdateInferenceEndpointParams};

use crate::output::CommandResult;

/// Update an inference endpoint
#[derive(ClapArgs)]
pub struct Args {
    /// Endpoint name
    pub name: String,

    /// Namespace (user or organization)
    #[arg(long)]
    pub namespace: Option<String>,

    /// Model repository ID
    #[arg(long)]
    pub repo: Option<String>,

    /// Accelerator type
    #[arg(long)]
    pub accelerator: Option<String>,

    /// Instance size
    #[arg(long)]
    pub instance_size: Option<String>,

    /// Instance type
    #[arg(long)]
    pub instance_type: Option<String>,

    /// Model framework
    #[arg(long)]
    pub framework: Option<String>,

    /// Model revision
    #[arg(long)]
    pub revision: Option<String>,

    /// Task type
    #[arg(long)]
    pub task: Option<String>,

    /// Minimum number of replicas
    #[arg(long)]
    pub min_replica: Option<u32>,

    /// Maximum number of replicas
    #[arg(long)]
    pub max_replica: Option<u32>,

    /// Scale-to-zero timeout in minutes
    #[arg(long)]
    pub scale_to_zero_timeout: Option<u32>,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let params = UpdateInferenceEndpointParams {
        name: args.name.clone(),
        namespace: args.namespace,
        accelerator: args.accelerator,
        instance_size: args.instance_size,
        instance_type: args.instance_type,
        min_replica: args.min_replica,
        max_replica: args.max_replica,
        scale_to_zero_timeout: args.scale_to_zero_timeout,
        repository: args.repo,
        framework: args.framework,
        revision: args.revision,
        task: args.task,
        custom_image: None,
        secrets: None,
    };
    api.update_inference_endpoint(&params).await?;
    Ok(CommandResult::Raw(args.name))
}
