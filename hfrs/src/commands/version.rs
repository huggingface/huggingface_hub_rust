use anyhow::Result;
use clap::Args as ClapArgs;

use crate::output::CommandResult;

/// Show CLI version
#[derive(ClapArgs)]
pub struct Args {}

pub async fn execute(_args: Args) -> Result<CommandResult> {
    Ok(CommandResult::Silent)
}
