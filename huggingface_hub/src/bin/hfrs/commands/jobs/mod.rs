pub mod cancel;
pub mod hardware;
pub mod inspect;
pub mod logs;
pub mod ps;
pub mod run;
pub mod scheduled;
pub mod stats;

use anyhow::Result;
use clap::{Args as ClapArgs, Subcommand};
use huggingface_hub::HFClient;

use crate::output::CommandResult;

/// Manage compute jobs
#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    pub command: JobsCommand,
}

/// Jobs subcommands
#[derive(Subcommand)]
pub enum JobsCommand {
    /// Run a compute job
    Run(run::Args),
    /// List running and recent jobs
    Ps(ps::Args),
    /// Show details of a job
    Inspect(inspect::Args),
    /// Cancel a running job
    Cancel(cancel::Args),
    /// Stream or fetch logs for a job
    Logs(logs::Args),
    /// List available hardware flavors for jobs
    Hardware(hardware::Args),
    /// Show resource usage statistics for a job
    Stats(stats::Args),
    /// Manage scheduled jobs
    Scheduled(scheduled::Args),
}

pub async fn execute(api: &HFClient, args: Args) -> Result<CommandResult> {
    match args.command {
        JobsCommand::Run(a) => run::execute(api, a).await,
        JobsCommand::Ps(a) => ps::execute(api, a).await,
        JobsCommand::Inspect(a) => inspect::execute(api, a).await,
        JobsCommand::Cancel(a) => cancel::execute(api, a).await,
        JobsCommand::Logs(a) => logs::execute(api, a).await,
        JobsCommand::Hardware(a) => hardware::execute(api, a).await,
        JobsCommand::Stats(a) => stats::execute(api, a).await,
        JobsCommand::Scheduled(a) => scheduled::execute(api, a).await,
    }
}
