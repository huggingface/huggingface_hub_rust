use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::HfApi;

use crate::output::CommandResult;

/// Show resource usage statistics for a job
#[derive(ClapArgs)]
pub struct Args {}

pub async fn execute(_api: &HfApi, _args: Args) -> Result<CommandResult> {
    Ok(CommandResult::Silent)
}
