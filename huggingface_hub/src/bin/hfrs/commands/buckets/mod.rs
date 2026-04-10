pub mod create;
pub mod delete;
pub mod info;
pub mod move_bucket;

use anyhow::Result;
use clap::{Args as ClapArgs, Subcommand};
use huggingface_hub::HFClient;

use crate::output::CommandResult;

#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    pub command: BucketsCommand,
}

#[derive(Subcommand)]
pub enum BucketsCommand {
    /// Create a new bucket
    Create(create::Args),
    /// Show detailed information about a bucket
    Info(info::Args),
    /// Delete a bucket
    Delete(delete::Args),
    /// Move (rename) a bucket
    Move(move_bucket::Args),
}

pub async fn execute(api: &HFClient, args: Args) -> Result<CommandResult> {
    match args.command {
        BucketsCommand::Create(a) => create::execute(api, a).await,
        BucketsCommand::Info(a) => info::execute(api, a).await,
        BucketsCommand::Delete(a) => delete::execute(api, a).await,
        BucketsCommand::Move(a) => move_bucket::execute(api, a).await,
    }
}

/// Parse a bucket ID from CLI input.
/// Accepts `namespace/name` or `hf://buckets/namespace/name`.
/// Returns `(namespace, name)` or an error.
pub(crate) fn parse_bucket_id(input: &str) -> Result<(String, String)> {
    let id = input.strip_prefix("hf://buckets/").unwrap_or(input);

    match id.split_once('/') {
        Some((ns, name)) if !ns.is_empty() && !name.is_empty() && !name.contains('/') => {
            Ok((ns.to_string(), name.to_string()))
        },
        _ => anyhow::bail!(
            "Invalid bucket ID '{input}'. Expected format: 'namespace/bucket_name' or 'hf://buckets/namespace/bucket_name'"
        ),
    }
}
