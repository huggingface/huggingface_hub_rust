use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::HfApi;

use crate::output::CommandResult;

/// Resume a paused inference endpoint
#[derive(ClapArgs)]
pub struct Args {}

pub async fn execute(_api: &HfApi, _args: Args) -> Result<CommandResult> {
    Ok(CommandResult::Silent)
}
