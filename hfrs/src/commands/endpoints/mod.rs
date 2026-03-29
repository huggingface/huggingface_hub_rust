pub mod delete;
pub mod deploy;
pub mod describe;
pub mod list;
pub mod pause;
pub mod resume;
pub mod scale_to_zero;
pub mod update;

use anyhow::Result;
use clap::{Args as ClapArgs, Subcommand};
use huggingface_hub::HfApi;

use crate::output::CommandResult;

/// Manage inference endpoints
#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    pub command: EndpointsCommand,
}

/// Endpoints subcommands
#[derive(Subcommand)]
pub enum EndpointsCommand {
    /// List inference endpoints
    List(list::Args),
    /// Show details of an inference endpoint
    Describe(describe::Args),
    /// Deploy a new inference endpoint
    Deploy(deploy::Args),
    /// Delete an inference endpoint
    Delete(delete::Args),
    /// Pause an inference endpoint
    Pause(pause::Args),
    /// Resume a paused inference endpoint
    Resume(resume::Args),
    /// Scale an inference endpoint to zero replicas
    ScaleToZero(scale_to_zero::Args),
    /// Update an inference endpoint
    Update(update::Args),
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    match args.command {
        EndpointsCommand::List(a) => list::execute(api, a).await,
        EndpointsCommand::Describe(a) => describe::execute(api, a).await,
        EndpointsCommand::Deploy(a) => deploy::execute(api, a).await,
        EndpointsCommand::Delete(a) => delete::execute(api, a).await,
        EndpointsCommand::Pause(a) => pause::execute(api, a).await,
        EndpointsCommand::Resume(a) => resume::execute(api, a).await,
        EndpointsCommand::ScaleToZero(a) => scale_to_zero::execute(api, a).await,
        EndpointsCommand::Update(a) => update::execute(api, a).await,
    }
}
