pub mod info;
pub mod list;
pub mod search;

use anyhow::Result;
use clap::{Args as ClapArgs, Subcommand};
use huggingface_hub::HfApi;

use crate::output::CommandResult;

/// Browse and manage papers
#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    pub command: PapersCommand,
}

/// Papers subcommands
#[derive(Subcommand)]
pub enum PapersCommand {
    /// Show details of a paper
    Info(info::Args),
    /// List daily papers
    List(list::Args),
    /// Search for papers
    Search(search::Args),
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    match args.command {
        PapersCommand::Info(a) => info::execute(api, a).await,
        PapersCommand::List(a) => list::execute(api, a).await,
        PapersCommand::Search(a) => search::execute(api, a).await,
    }
}
