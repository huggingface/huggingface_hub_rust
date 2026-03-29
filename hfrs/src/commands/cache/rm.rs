use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::HfApi;

use crate::output::CommandResult;

/// Delete cached revisions
#[derive(ClapArgs)]
pub struct Args {
    /// Cache entries to delete (repo_id or repo_id@revision format)
    #[arg(required = true)]
    pub targets: Vec<String>,
}

pub async fn execute(_api: &HfApi, _args: Args) -> Result<CommandResult> {
    anyhow::bail!(
        "Cache deletion is not yet supported. \
        To delete cached files, remove them manually from the HF cache directory \
        (default: ~/.cache/huggingface/hub)."
    )
}
