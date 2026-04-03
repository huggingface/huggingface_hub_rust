use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::HfApi;
use serde_json::json;

use crate::cli::{OutputFormat, RepoTypeArg};
use crate::output::{CommandOutput, CommandResult};

/// List access requests for a gated repository
#[derive(ClapArgs)]
pub struct Args {
    /// Repository ID (e.g. username/my-model)
    pub repo_id: String,

    /// Filter by status: pending, accepted, or rejected
    #[arg(long, default_value = "pending")]
    pub status: String,

    /// Repository type
    #[arg(long = "type", value_enum)]
    pub repo_type: Option<RepoTypeArg>,

    /// Output format
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,

    /// Print only usernames
    #[arg(long)]
    pub quiet: bool,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let repo_type = args.repo_type.map(Into::into).unwrap_or(huggingface_hub::RepoType::Model);
    let repo = crate::util::make_repo(api, &args.repo_id, repo_type);
    let requests = match args.status.as_str() {
        "accepted" => repo.list_accepted_access_requests().await?,
        "rejected" => repo.list_rejected_access_requests().await?,
        _ => repo.list_pending_access_requests().await?,
    };

    let headers = vec![
        "Username".to_string(),
        "Email".to_string(),
        "Status".to_string(),
        "Timestamp".to_string(),
    ];

    let rows = requests
        .iter()
        .map(|r| {
            vec![
                r.username.clone().unwrap_or_default(),
                r.email.clone().unwrap_or_default(),
                r.status.clone().unwrap_or_default(),
                r.timestamp.clone().unwrap_or_default(),
            ]
        })
        .collect();

    let quiet_values = requests.iter().filter_map(|r| r.username.clone()).collect();

    let json_value: serde_json::Value = requests
        .iter()
        .map(|r| {
            json!({
                "username": r.username,
                "email": r.email,
                "status": r.status,
                "timestamp": r.timestamp,
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
        quiet: args.quiet,
    })
}
