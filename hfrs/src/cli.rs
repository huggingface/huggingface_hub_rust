use clap::builder::styling::{AnsiColor, Effects, Styles};
use clap::{Parser, Subcommand, ValueEnum};
use huggingface_hub::RepoType;

const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Yellow.on_default().effects(Effects::BOLD))
    .usage(AnsiColor::Yellow.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Cyan.on_default())
    .valid(AnsiColor::Green.on_default())
    .invalid(AnsiColor::Red.on_default());

#[derive(Parser)]
#[command(name = "hfrs", about = "Hugging Face Hub CLI (Rust)", version, styles = STYLES)]
pub struct Cli {
    /// Authentication token (overrides HF_TOKEN env var and stored credentials)
    #[arg(long, env = "HF_TOKEN", global = true, hide_env_values = true)]
    pub token: Option<String>,

    /// API endpoint override
    #[arg(long, env = "HF_ENDPOINT", global = true)]
    pub endpoint: Option<String>,

    /// Disable colored output
    #[arg(long, global = true)]
    pub no_color: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Manage authentication (login, logout, etc.)
    Auth(crate::commands::auth::Args),
    /// Manage local cache directory
    Cache(crate::commands::cache::Args),
    /// Interact with collections on the Hub
    Collections(crate::commands::collections::Args),
    /// Interact with datasets on the Hub
    Datasets(crate::commands::datasets::Args),
    /// Manage discussions and pull requests on the Hub
    Discussions(crate::commands::discussions::Args),
    /// Download files from the Hub
    Download(crate::commands::download::Args),
    /// Manage Hugging Face Inference Endpoints
    Endpoints(crate::commands::endpoints::Args),
    /// Run and manage Jobs on the Hub
    Jobs(crate::commands::jobs::Args),
    /// Manage likes on the Hub
    Likes(crate::commands::likes::Args),
    /// Interact with models on the Hub
    Models(crate::commands::models::Args),
    /// Interact with papers on the Hub
    Papers(crate::commands::papers::Args),
    /// Manage repos on the Hub
    #[command(alias = "repo")]
    Repos(crate::commands::repos::Args),
    /// Interact with Spaces on the Hub
    Spaces(crate::commands::spaces::Args),
    /// Upload a file or folder to the Hub
    Upload(crate::commands::upload::Args),
    /// Manage webhooks on the Hub
    Webhooks(crate::commands::webhooks::Args),
    /// Manage gated repo access requests
    #[command(name = "access-requests")]
    AccessRequests(crate::commands::access_requests::Args),
    /// Print information about the environment
    Env(crate::commands::env::Args),
    /// Print the hfrs version
    Version(crate::commands::version::Args),
}

#[derive(Clone, ValueEnum)]
pub enum OutputFormat {
    Table,
    Json,
}

#[derive(Clone, ValueEnum)]
pub enum RepoTypeArg {
    Model,
    Dataset,
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
