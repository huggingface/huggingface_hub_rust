use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::HfApi;

use crate::output::CommandResult;

/// Show detailed information about a model
#[derive(ClapArgs)]
pub struct Args {}

pub async fn execute(_api: &HfApi, _args: Args) -> Result<CommandResult> {
    Ok(CommandResult::Silent)
}
