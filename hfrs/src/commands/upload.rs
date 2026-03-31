use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::Args as ClapArgs;
use huggingface_hub::{AddSource, CreateRepoParams, HfApi, RepoExistsParams, UploadFileParams, UploadFolderParams};
use tracing::info;

use crate::cli::RepoTypeArg;
use crate::output::CommandResult;

/// Upload files to the Hub
#[derive(ClapArgs)]
pub struct Args {
    /// Repository ID (e.g. username/my-model)
    pub repo_id: String,

    /// Local file or folder to upload (defaults to current directory)
    pub local_path: Option<PathBuf>,

    /// Path in the repository to upload to
    pub path_in_repo: Option<String>,

    /// Repository type
    #[arg(long, visible_alias = "repo-type", value_enum, default_value = "model")]
    pub r#type: RepoTypeArg,

    /// Git revision (branch, tag, or commit SHA)
    #[arg(long)]
    pub revision: Option<String>,

    /// Create the repo as private if it does not exist yet
    #[arg(long)]
    pub private: bool,

    /// Include patterns for folder upload (can be specified multiple times)
    #[arg(long)]
    pub include: Vec<String>,

    /// Exclude patterns for folder upload (can be specified multiple times)
    #[arg(long)]
    pub exclude: Vec<String>,

    /// Delete patterns for folder upload (can be specified multiple times)
    #[arg(long)]
    pub delete: Vec<String>,

    /// Commit message
    #[arg(long)]
    pub commit_message: Option<String>,

    /// Commit description
    #[arg(long)]
    pub commit_description: Option<String>,

    /// Create a pull request instead of committing directly
    #[arg(long)]
    pub create_pr: bool,

    /// Print only the commit URL, suppress progress
    #[arg(long)]
    pub quiet: bool,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let repo_type: huggingface_hub::RepoType = args.r#type.into();
    let local_path = args.local_path.unwrap_or_else(|| PathBuf::from("."));

    // Ensure the repo exists, creating it if necessary
    let exists_params = RepoExistsParams {
        repo_id: args.repo_id.clone(),
        repo_type: Some(repo_type),
    };
    if !api.repo_exists(&exists_params).await? {
        info!(repo_id = args.repo_id.as_str(), private = args.private, "creating repository");
        let create_params = CreateRepoParams {
            repo_id: args.repo_id.clone(),
            repo_type: Some(repo_type),
            private: if args.private { Some(true) } else { None },
            exist_ok: true,
            space_sdk: None,
        };
        api.create_repo(&create_params).await?;
    }

    let commit_info = if local_path.is_file() {
        let path_in_repo = args.path_in_repo.unwrap_or_else(|| {
            local_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        });
        let params = UploadFileParams {
            repo_id: args.repo_id,
            source: AddSource::File(local_path),
            path_in_repo,
            repo_type: Some(repo_type),
            revision: args.revision,
            commit_message: args.commit_message,
            commit_description: args.commit_description,
            create_pr: if args.create_pr { Some(true) } else { None },
            parent_commit: None,
        };
        api.upload_file(&params).await?
    } else if local_path.is_dir() {
        let allow_patterns = if !args.include.is_empty() {
            Some(args.include)
        } else {
            None
        };
        let ignore_patterns = if !args.exclude.is_empty() {
            Some(args.exclude)
        } else {
            None
        };
        let delete_patterns = if !args.delete.is_empty() {
            Some(args.delete)
        } else {
            None
        };
        let params = UploadFolderParams {
            repo_id: args.repo_id,
            folder_path: local_path,
            path_in_repo: args.path_in_repo,
            repo_type: Some(repo_type),
            revision: args.revision,
            commit_message: args.commit_message,
            commit_description: args.commit_description,
            create_pr: if args.create_pr { Some(true) } else { None },
            allow_patterns,
            ignore_patterns,
            delete_patterns,
        };
        api.upload_folder(&params).await?
    } else {
        bail!("local path does not exist: {}", local_path.display());
    };

    if args.quiet {
        Ok(CommandResult::Silent)
    } else {
        let url = commit_info.commit_url.or(commit_info.pr_url).unwrap_or_default();
        Ok(CommandResult::Raw(url))
    }
}
