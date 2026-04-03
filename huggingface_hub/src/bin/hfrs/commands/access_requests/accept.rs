use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{HfApi, RepoAccessRequestUserParams};

use crate::cli::RepoTypeArg;
use crate::output::CommandResult;

/// Accept an access request
#[derive(ClapArgs)]
pub struct Args {
    /// Repository ID (e.g. username/my-model)
    pub repo_id: String,

    /// Username to accept
    pub user: String,

    /// Repository type
    #[arg(long = "type", value_enum)]
    pub repo_type: Option<RepoTypeArg>,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let repo_type = args.repo_type.map(Into::into).unwrap_or(huggingface_hub::RepoType::Model);
    let repo = crate::util::make_repo(api, &args.repo_id, repo_type);
    repo.accept_access_request(&RepoAccessRequestUserParams { user: args.user })
        .await?;
    Ok(CommandResult::Silent)
}
