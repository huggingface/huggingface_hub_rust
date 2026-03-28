use anyhow::Result;
use clap::{Args as ClapArgs, Subcommand};
use huggingface_hub::{CreateTagParams, DeleteTagParams, HfApi, ListRepoRefsParams};
use serde_json::json;

use crate::cli::{OutputFormat, RepoTypeArg};
use crate::output::{CommandOutput, CommandResult};

/// Manage repository tags
#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    pub command: TagCommand,
}

/// Tag subcommands
#[derive(Subcommand)]
pub enum TagCommand {
    /// Create a new tag
    Create(TagCreateArgs),
    /// Delete a tag
    Delete(TagDeleteArgs),
    /// List tags
    List(TagListArgs),
}

/// Create a new tag
#[derive(ClapArgs)]
pub struct TagCreateArgs {
    /// Repository ID
    pub repo_id: String,

    /// Tag name to create
    pub tag: String,

    /// Tag message (creates an annotated tag)
    #[arg(short = 'm', long)]
    pub message: Option<String>,

    /// Starting revision (branch, tag, or commit SHA)
    #[arg(long)]
    pub revision: Option<String>,

    /// Repository type
    #[arg(long, value_enum, default_value = "model")]
    pub r#type: RepoTypeArg,
}

/// Delete a tag
#[derive(ClapArgs)]
pub struct TagDeleteArgs {
    /// Repository ID
    pub repo_id: String,

    /// Tag name to delete
    pub tag: String,

    /// Repository type
    #[arg(long, value_enum, default_value = "model")]
    pub r#type: RepoTypeArg,
}

/// List tags
#[derive(ClapArgs)]
pub struct TagListArgs {
    /// Repository ID
    pub repo_id: String,

    /// Repository type
    #[arg(long, value_enum, default_value = "model")]
    pub r#type: RepoTypeArg,

    /// Output format
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    match args.command {
        TagCommand::Create(a) => create(api, a).await,
        TagCommand::Delete(a) => delete(api, a).await,
        TagCommand::List(a) => list(api, a).await,
    }
}

async fn create(api: &HfApi, args: TagCreateArgs) -> Result<CommandResult> {
    let repo_type: huggingface_hub::RepoType = args.r#type.into();
    let params = CreateTagParams {
        repo_id: args.repo_id,
        tag: args.tag,
        revision: args.revision,
        message: args.message,
        repo_type: Some(repo_type),
    };
    api.create_tag(&params).await?;
    Ok(CommandResult::Raw("Tag created.".to_string()))
}

async fn delete(api: &HfApi, args: TagDeleteArgs) -> Result<CommandResult> {
    let repo_type: huggingface_hub::RepoType = args.r#type.into();
    let params = DeleteTagParams {
        repo_id: args.repo_id,
        tag: args.tag,
        repo_type: Some(repo_type),
    };
    api.delete_tag(&params).await?;
    Ok(CommandResult::Silent)
}

async fn list(api: &HfApi, args: TagListArgs) -> Result<CommandResult> {
    let repo_type: huggingface_hub::RepoType = args.r#type.into();
    let params = ListRepoRefsParams {
        repo_id: args.repo_id,
        repo_type: Some(repo_type),
        include_pull_requests: false,
    };
    let refs = api.list_repo_refs(&params).await?;

    let headers = vec!["Name".to_string(), "Ref".to_string(), "Commit".to_string()];
    let rows = refs
        .tags
        .iter()
        .map(|t| vec![t.name.clone(), t.git_ref.clone(), t.target_commit.clone()])
        .collect();
    let quiet_values = refs.tags.iter().map(|t| t.name.clone()).collect();
    let json_value = refs
        .tags
        .iter()
        .map(|t| {
            json!({
                "name": t.name,
                "ref": t.git_ref,
                "target_commit": t.target_commit,
            })
        })
        .collect::<Vec<_>>()
        .into();

    let output = CommandOutput {
        headers,
        rows,
        json_value,
        quiet_values,
    };
    Ok(CommandResult::Formatted {
        output,
        format: args.format,
        quiet: false,
    })
}
