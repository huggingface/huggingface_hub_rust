pub mod delete;
pub mod inspect;
pub mod ps;
pub mod resume;
pub mod run;
pub mod suspend;

use anyhow::Result;
use clap::{Args as ClapArgs, Subcommand};
use huggingface_hub::HfApi;

use crate::output::CommandResult;

/// Manage scheduled jobs
#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    pub command: ScheduledCommand,
}

/// Scheduled job subcommands
#[derive(Subcommand)]
pub enum ScheduledCommand {
    /// Create a new scheduled job
    Run(run::Args),
    /// List scheduled jobs
    Ps(ps::Args),
    /// Show details of a scheduled job
    Inspect(inspect::Args),
    /// Delete a scheduled job
    Delete(delete::Args),
    /// Suspend a scheduled job
    Suspend(suspend::Args),
    /// Resume a suspended scheduled job
    Resume(resume::Args),
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    match args.command {
        ScheduledCommand::Run(a) => run::execute(api, a).await,
        ScheduledCommand::Ps(a) => ps::execute(api, a).await,
        ScheduledCommand::Inspect(a) => inspect::execute(api, a).await,
        ScheduledCommand::Delete(a) => delete::execute(api, a).await,
        ScheduledCommand::Suspend(a) => suspend::execute(api, a).await,
        ScheduledCommand::Resume(a) => resume::execute(api, a).await,
    }
}
