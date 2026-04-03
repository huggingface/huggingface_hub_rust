use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{HfApi, RepoUpdateSettingsParams};

use crate::cli::RepoTypeArg;
use crate::output::CommandResult;

/// Update repository settings
#[derive(ClapArgs)]
pub struct Args {
    /// Repository ID (e.g. username/my-model)
    pub repo_id: String,

    /// Repository type
    #[arg(long, visible_alias = "repo-type", value_enum, default_value = "model")]
    pub r#type: RepoTypeArg,

    /// Gating strategy (e.g. "auto", "manual", or "false" to disable)
    #[arg(long)]
    pub gated: Option<String>,

    /// Set private visibility
    #[arg(long)]
    pub private: Option<bool>,

    /// Repository description
    #[arg(long)]
    pub description: Option<String>,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let repo_type: huggingface_hub::RepoType = args.r#type.into();
    let repo = crate::util::make_repo(api, &args.repo_id, repo_type);
    let params = RepoUpdateSettingsParams {
        private: args.private,
        gated: args.gated,
        description: args.description,
    };
    repo.update_settings(&params).await?;
    Ok(CommandResult::Silent)
}
