use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{CreateInferenceEndpointParams, HfApi};

use crate::output::CommandResult;

/// Deploy a new inference endpoint
#[derive(ClapArgs)]
pub struct Args {
    /// Endpoint name
    pub name: String,

    /// Model repository ID
    #[arg(long, required = true)]
    pub repo: String,

    /// Model framework
    #[arg(long, required = true)]
    pub framework: String,

    /// Accelerator type (e.g. gpu)
    #[arg(long, required = true)]
    pub accelerator: String,

    /// Instance size (e.g. x1)
    #[arg(long, required = true)]
    pub instance_size: String,

    /// Instance type (e.g. nvidia-tesla-t4)
    #[arg(long, required = true)]
    pub instance_type: String,

    /// Cloud region
    #[arg(long, required = true)]
    pub region: String,

    /// Cloud vendor (e.g. aws)
    #[arg(long, required = true)]
    pub vendor: String,

    /// Namespace (user or organization)
    #[arg(long)]
    pub namespace: Option<String>,

    /// Task type (e.g. text-generation)
    #[arg(long, default_value = "")]
    pub task: String,

    /// Minimum number of replicas
    #[arg(long)]
    pub min_replica: Option<u32>,

    /// Maximum number of replicas
    #[arg(long)]
    pub max_replica: Option<u32>,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let params = CreateInferenceEndpointParams {
        name: args.name,
        repository: args.repo,
        framework: args.framework,
        task: args.task,
        accelerator: args.accelerator,
        instance_size: args.instance_size,
        instance_type: args.instance_type,
        region: args.region,
        vendor: args.vendor,
        namespace: args.namespace,
        revision: None,
        min_replica: args.min_replica,
        max_replica: args.max_replica,
        scale_to_zero_timeout: None,
        endpoint_type: None,
        custom_image: None,
        secrets: None,
    };
    let e = api.create_inference_endpoint(&params).await?;
    let status = e.status.state.unwrap_or_default();
    Ok(CommandResult::Raw(format!("{} ({})", e.name, status)))
}
