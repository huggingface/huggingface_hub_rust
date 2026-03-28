pub mod close;
pub mod comment;
pub mod create;
pub mod diff;
pub mod info;
pub mod list;
pub mod merge;
pub mod rename;
pub mod reopen;

use anyhow::Result;
use clap::{Args as ClapArgs, Subcommand};
use huggingface_hub::HfApi;

use crate::output::CommandResult;

/// Manage discussions and pull requests
#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    pub command: DiscussionsCommand,
}

/// Discussions subcommands
#[derive(Subcommand)]
pub enum DiscussionsCommand {
    /// List discussions and pull requests for a repository
    List(list::Args),
    /// Show details of a discussion or pull request
    Info(info::Args),
    /// Create a new discussion or pull request
    Create(create::Args),
    /// Add a comment to a discussion
    Comment(comment::Args),
    /// Merge a pull request
    Merge(merge::Args),
    /// Close a discussion or pull request
    Close(close::Args),
    /// Reopen a closed discussion or pull request
    Reopen(reopen::Args),
    /// Rename a discussion or pull request
    Rename(rename::Args),
    /// Show the diff for a pull request
    Diff(diff::Args),
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    match args.command {
        DiscussionsCommand::List(a) => list::execute(api, a).await,
        DiscussionsCommand::Info(a) => info::execute(api, a).await,
        DiscussionsCommand::Create(a) => create::execute(api, a).await,
        DiscussionsCommand::Comment(a) => comment::execute(api, a).await,
        DiscussionsCommand::Merge(a) => merge::execute(api, a).await,
        DiscussionsCommand::Close(a) => close::execute(api, a).await,
        DiscussionsCommand::Reopen(a) => reopen::execute(api, a).await,
        DiscussionsCommand::Rename(a) => rename::execute(api, a).await,
        DiscussionsCommand::Diff(a) => diff::execute(api, a).await,
    }
}
