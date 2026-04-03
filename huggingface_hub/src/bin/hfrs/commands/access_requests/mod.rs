pub mod accept;
pub mod cancel;
pub mod grant;
pub mod list;
pub mod reject;

use anyhow::Result;
use clap::{Args as ClapArgs, Subcommand};
use huggingface_hub::HfApi;

use crate::output::CommandResult;

/// Manage gated repository access requests
#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    pub command: AccessRequestsCommand,
}

/// Access requests subcommands
#[derive(Subcommand)]
pub enum AccessRequestsCommand {
    /// List access requests for a gated repository
    List(list::Args),
    /// Accept an access request
    Accept(accept::Args),
    /// Reject an access request
    Reject(reject::Args),
    /// Cancel a pending access request
    Cancel(cancel::Args),
    /// Grant access to a user for a gated repository
    Grant(grant::Args),
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    match args.command {
        AccessRequestsCommand::List(a) => list::execute(api, a).await,
        AccessRequestsCommand::Accept(a) => accept::execute(api, a).await,
        AccessRequestsCommand::Reject(a) => reject::execute(api, a).await,
        AccessRequestsCommand::Cancel(a) => cancel::execute(api, a).await,
        AccessRequestsCommand::Grant(a) => grant::execute(api, a).await,
    }
}
