use std::collections::HashMap;

use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{CreateScheduledJobParams, HFClient};

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

/// Create a new scheduled job
#[derive(ClapArgs)]
pub struct Args {
    /// Cron schedule expression
    pub schedule: String,

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

    /// Create in suspended state
    #[arg(long)]
    pub suspend: bool,

    /// Allow concurrent runs
    #[arg(long)]
    pub concurrency: bool,
}

pub async fn execute(api: &HFClient, args: Args) -> Result<CommandResult> {
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
    let params = CreateScheduledJobParams {
        image: args.image,
        command: args.command,
        schedule: args.schedule,
        flavor: args.flavor,
        env,
        secrets,
        timeout: args.timeout,
        namespace: args.namespace,
        suspend: if args.suspend { Some(true) } else { None },
        concurrency: if args.concurrency { Some(true) } else { None },
    };
    let job = api.create_scheduled_job(&params).await?;
    Ok(CommandResult::Raw(job.id))
}
