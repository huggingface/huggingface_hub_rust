use clap::{Parser, Subcommand, ValueEnum};
use huggingface_hub::RepoType;

/// Hugging Face Hub CLI
#[derive(Parser)]
#[command(name = "hfrs", about = "Interact with the Hugging Face Hub", version)]
pub struct Cli {
    /// HF API token (overrides HF_TOKEN env var and stored credentials)
    #[arg(long, env = "HF_TOKEN", global = true, hide_env_values = true)]
    pub token: Option<String>,

    /// Hub endpoint URL
    #[arg(long, env = "HF_ENDPOINT", global = true)]
    pub endpoint: Option<String>,

    /// Output format
    #[arg(long, value_enum, global = true, default_value = "table")]
    pub format: OutputFormat,

    /// Suppress output except for quiet values
    #[arg(long, global = true)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Command,
}

/// Top-level commands
#[derive(Subcommand)]
pub enum Command {
    /// Manage authentication credentials
    Auth(crate::commands::auth::Args),
    /// Manage the local model cache
    Cache(crate::commands::cache::Args),
    /// Manage collections
    Collections(crate::commands::collections::Args),
    /// Manage datasets
    Datasets(crate::commands::datasets::Args),
    /// Manage discussions and pull requests
    Discussions(crate::commands::discussions::Args),
    /// Download files from the Hub
    Download(crate::commands::download::Args),
    /// Manage inference endpoints
    Endpoints(crate::commands::endpoints::Args),
    /// Manage compute jobs
    Jobs(crate::commands::jobs::Args),
    /// Manage likes
    Likes(crate::commands::likes::Args),
    /// Manage models
    Models(crate::commands::models::Args),
    /// Browse and manage papers
    Papers(crate::commands::papers::Args),
    /// Manage repositories
    #[command(alias = "repo")]
    Repos(crate::commands::repos::Args),
    /// Manage Spaces
    Spaces(crate::commands::spaces::Args),
    /// Upload files to the Hub
    Upload(crate::commands::upload::Args),
    /// Manage webhooks
    Webhooks(crate::commands::webhooks::Args),
    /// Manage gated repository access requests
    #[command(name = "access-requests")]
    AccessRequests(crate::commands::access_requests::Args),
    /// Show environment and configuration info
    Env(crate::commands::env::Args),
    /// Show CLI version
    Version(crate::commands::version::Args),
}

/// Output format
#[derive(Clone, ValueEnum)]
pub enum OutputFormat {
    /// Render as a table
    Table,
    /// Render as JSON
    Json,
}

/// Repository type filter
#[derive(Clone, ValueEnum)]
pub enum RepoTypeArg {
    /// Model repository
    Model,
    /// Dataset repository
    Dataset,
    /// Space
    Space,
}

impl From<RepoTypeArg> for RepoType {
    fn from(arg: RepoTypeArg) -> Self {
        match arg {
            RepoTypeArg::Model => RepoType::Model,
            RepoTypeArg::Dataset => RepoType::Dataset,
            RepoTypeArg::Space => RepoType::Space,
        }
    }
}
