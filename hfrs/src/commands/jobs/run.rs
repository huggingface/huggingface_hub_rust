use std::collections::HashMap;

use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{HfApi, RunJobParams};

use crate::output::CommandResult;

fn parse_kv_pairs(pairs: Vec<String>) -> HashMap<String, String> {
    pairs
        .into_iter()
        .filter_map(|s| {
            let mut parts = s.splitn(2, '=');
            let key = parts.next()?.to_string();
            let val = parts.next()?.to_string();
            Some((key, val))
        })
        .collect()
}

/// Run a compute job
#[derive(ClapArgs)]
pub struct Args {
    /// Docker image to use
    pub image: String,

    /// Command to run in the container
    #[arg(trailing_var_arg = true)]
    pub command: Vec<String>,

    /// Hardware flavor
    #[arg(long)]
    pub flavor: Option<String>,

    /// Environment variables as KEY=VALUE pairs
    #[arg(long = "env", value_name = "KEY=VALUE")]
    pub env: Vec<String>,

    /// Secret variables as KEY=VALUE pairs
    #[arg(long = "secret", value_name = "KEY=VALUE")]
    pub secrets: Vec<String>,

    /// Job timeout
    #[arg(long)]
    pub timeout: Option<String>,

    /// Namespace (user or organization)
    #[arg(long)]
    pub namespace: Option<String>,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let env = if args.env.is_empty() {
        None
    } else {
        Some(parse_kv_pairs(args.env))
    };
    let secrets = if args.secrets.is_empty() {
        None
    } else {
        Some(parse_kv_pairs(args.secrets))
    };
    let params = RunJobParams {
        image: args.image,
        command: args.command,
        flavor: args.flavor,
        env,
        secrets,
        timeout: args.timeout,
        labels: None,
        namespace: args.namespace,
    };
    let job = api.run_job(&params).await?;
    Ok(CommandResult::Raw(job.id))
}
