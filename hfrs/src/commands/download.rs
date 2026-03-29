use std::path::PathBuf;

use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{DownloadFileParams, HfApi, SnapshotDownloadParams};

use crate::cli::RepoTypeArg;
use crate::output::CommandResult;

/// Download files from the Hub
#[derive(ClapArgs)]
pub struct Args {
    /// Repository ID (e.g. username/my-model)
    pub repo_id: String,

    /// Specific filenames to download
    pub filenames: Vec<String>,

    /// Repository type
    #[arg(long, value_enum, default_value = "model")]
    pub r#type: RepoTypeArg,

    /// Git revision (branch, tag, or commit SHA)
    #[arg(long)]
    pub revision: Option<String>,

    /// Include patterns for snapshot download (can be specified multiple times)
    #[arg(long)]
    pub include: Vec<String>,

    /// Exclude patterns for snapshot download (can be specified multiple times)
    #[arg(long)]
    pub exclude: Vec<String>,

    /// Local cache directory
    #[arg(long)]
    pub cache_dir: Option<PathBuf>,

    /// Local directory to save files into (bypasses cache)
    #[arg(long)]
    pub local_dir: Option<PathBuf>,

    /// Force re-download even if cached
    #[arg(long)]
    pub force_download: bool,

    /// Print only the local path, suppress progress
    #[arg(long)]
    pub quiet: bool,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let repo_type: huggingface_hub::RepoType = args.r#type.into();

    let path = if args.filenames.len() == 1 && args.include.is_empty() && args.exclude.is_empty() {
        let params = DownloadFileParams {
            repo_id: args.repo_id,
            filename: args.filenames.into_iter().next().unwrap(),
            local_dir: args.local_dir,
            repo_type: Some(repo_type),
            revision: args.revision,
            force_download: if args.force_download { Some(true) } else { None },
            local_files_only: None,
            cache_dir: args.cache_dir,
        };
        api.download_file(&params).await?
    } else {
        let allow_patterns = if !args.filenames.is_empty() {
            Some(args.filenames)
        } else if !args.include.is_empty() {
            Some(args.include)
        } else {
            None
        };
        let ignore_patterns = if !args.exclude.is_empty() {
            Some(args.exclude)
        } else {
            None
        };
        let params = SnapshotDownloadParams {
            repo_id: args.repo_id,
            repo_type: Some(repo_type),
            revision: args.revision,
            allow_patterns,
            ignore_patterns,
            local_dir: args.local_dir,
            force_download: if args.force_download { Some(true) } else { None },
            local_files_only: None,
            max_workers: None,
            cache_dir: args.cache_dir,
        };
        api.snapshot_download(&params).await?
    };

    if args.quiet {
        Ok(CommandResult::Silent)
    } else {
        Ok(CommandResult::Raw(path.display().to_string()))
    }
}
