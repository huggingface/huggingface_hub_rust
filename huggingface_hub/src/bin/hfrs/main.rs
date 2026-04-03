mod cli;
mod commands;
mod output;
mod util;

use std::io::IsTerminal;
use std::process::ExitCode;

use clap::Parser;
use cli::{Cli, Command};
use huggingface_hub::{HfApiBuilder, HfError};
use output::render;
use owo_colors::OwoColorize;
use tracing::{debug, info};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    let color = should_use_color(cli.no_color);
    init_logging(color);

    let mut builder = HfApiBuilder::new();
    if let Some(t) = cli.token {
        debug!("using token from --token flag");
        builder = builder.token(t);
    }
    if let Some(ref endpoint) = cli.endpoint {
        debug!(endpoint = endpoint.as_str(), "using custom API endpoint");
        builder = builder.endpoint(endpoint);
    }
    if let Command::Download(ref args) = cli.command {
        if let Some(ref cache_dir) = args.cache_dir {
            builder = builder.cache_dir(cache_dir);
        }
    }
    let api = match builder.build() {
        Ok(api) => {
            info!("HfApi client initialized");
            api
        },
        Err(e) => {
            print_hf_error(&e, color);
            return ExitCode::FAILURE;
        },
    };

    let result = match cli.command {
        Command::Auth(args) => commands::auth::execute(&api, args).await,
        Command::Cache(args) => commands::cache::execute(&api, args).await,
        Command::Collections(args) => commands::collections::execute(&api, args).await,
        Command::Datasets(args) => commands::datasets::execute(&api, args).await,
        Command::Discussions(args) => commands::discussions::execute(&api, args).await,
        Command::Download(args) => commands::download::execute(&api, args).await,
        Command::Endpoints(args) => commands::endpoints::execute(&api, args).await,
        Command::Jobs(args) => commands::jobs::execute(&api, args).await,
        Command::Likes(args) => commands::likes::execute(&api, args).await,
        Command::Models(args) => commands::models::execute(&api, args).await,
        Command::Papers(args) => commands::papers::execute(&api, args).await,
        Command::Repos(args) => commands::repos::execute(&api, args).await,
        Command::Spaces(args) => commands::spaces::execute(&api, args).await,
        Command::Upload(args) => commands::upload::execute(&api, args).await,
        Command::Webhooks(args) => commands::webhooks::execute(&api, args).await,
        Command::AccessRequests(args) => commands::access_requests::execute(&api, args).await,
        Command::Env(args) => commands::env::execute(args).await,
        Command::Version(args) => commands::version::execute(args).await,
    };

    match result {
        Ok(output) => {
            if let Err(e) = render(output) {
                eprintln!("Error: {e}");
                return ExitCode::FAILURE;
            }
            ExitCode::SUCCESS
        },
        Err(e) => {
            print_anyhow_error(&e, color);
            ExitCode::FAILURE
        },
    }
}

fn should_use_color(no_color_flag: bool) -> bool {
    if no_color_flag {
        return false;
    }
    if std::env::var("NO_COLOR").is_ok() {
        return false;
    }
    if std::env::var("CLICOLOR_FORCE").is_ok_and(|v| v != "0") {
        return true;
    }
    std::io::stderr().is_terminal()
}

fn print_hf_error(err: &HfError, color: bool) {
    let message = format_hf_error(err);
    if color {
        eprintln!("{} {message}", "Error:".red().bold());
    } else {
        eprintln!("Error: {message}");
    }
}

fn print_anyhow_error(err: &anyhow::Error, color: bool) {
    let message = format_anyhow_error(err);

    if color {
        eprintln!("{} {message}", "Error:".red().bold());
    } else {
        eprintln!("Error: {message}");
    }

    if std::env::var("HF_DEBUG").is_ok() {
        for cause in err.chain().skip(1) {
            if color {
                eprintln!("  {} {cause}", "Caused by:".dimmed());
            } else {
                eprintln!("  Caused by: {cause}");
            }
        }
    } else if color {
        eprintln!("{}", "Set HF_DEBUG=1 for the full error trace.".dimmed());
    } else {
        eprintln!("Set HF_DEBUG=1 for the full error trace.");
    }
}

fn format_anyhow_error(err: &anyhow::Error) -> String {
    if let Some(hf_err) = err.downcast_ref::<HfError>() {
        return format_hf_error(hf_err);
    }

    for cause in err.chain() {
        if let Some(hf_err) = cause.downcast_ref::<HfError>() {
            return format_hf_error(hf_err);
        }
    }

    err.to_string()
}

fn format_hf_error(err: &HfError) -> String {
    match err {
        HfError::RepoNotFound { repo_id } => {
            format!("Repository '{repo_id}' not found. If the repo is private, make sure you are authenticated.")
        },
        HfError::EntryNotFound { path, repo_id } => {
            format!("File '{path}' not found in repository '{repo_id}'.")
        },
        HfError::RevisionNotFound { repo_id, revision } => {
            format!("Revision '{revision}' not found in repository '{repo_id}'.")
        },
        HfError::AuthRequired => {
            "Not authenticated. Run `hfrs auth login` or set the HF_TOKEN environment variable.".to_string()
        },
        HfError::Http { status, url, body } => {
            let status_code = status.as_u16();
            match status_code {
                401 => {
                    let mut msg = "Invalid or expired token.".to_string();
                    if std::env::var("HF_TOKEN").is_ok() {
                        msg.push_str(" Note: HF_TOKEN environment variable takes precedence over `hfrs auth login`.");
                    }
                    msg
                },
                403 => {
                    "Permission denied. Check that your token has the required scopes for this operation.".to_string()
                },
                404 => {
                    format!("Not found: {url}")
                },
                409 => {
                    if body.contains("already exists") {
                        "Resource already exists. Use --exist-ok to skip this error.".to_string()
                    } else {
                        format!("Conflict: {body}")
                    }
                },
                429 => "Rate limited. Please wait a moment and try again.".to_string(),
                500..=599 => {
                    format!(
                        "Server error ({status}). The Hugging Face Hub may be experiencing issues. Try again later."
                    )
                },
                _ => {
                    if body.is_empty() {
                        format!("{status} {url}")
                    } else {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
                            if let Some(error_msg) = json.get("error").and_then(|e| e.as_str()) {
                                return error_msg.to_string();
                            }
                        }
                        format!("{status}: {body}")
                    }
                },
            }
        },
        HfError::XetNotEnabled => {
            "Xet transfer protocol required but not enabled. Rebuild with the 'xet' feature.".to_string()
        },
        HfError::LocalEntryNotFound { path } => {
            format!("File not found in local cache: {path}. Try downloading it first.")
        },
        HfError::CacheNotEnabled => {
            "Cache is not enabled. Use --local-dir to specify a download directory.".to_string()
        },
        HfError::CacheLockTimeout { path } => {
            format!("Cache lock timed out: {}. Another process may be using this file.", path.display())
        },
        HfError::Request(e) => {
            if e.is_connect() {
                "Connection failed. Check your internet connection.".to_string()
            } else if e.is_timeout() {
                "Request timed out. The server may be slow or unreachable.".to_string()
            } else {
                format!("Network error: {e}")
            }
        },
        HfError::Middleware(e) => {
            let msg = format!("{e:#}");
            if msg.contains("Connect") {
                "Connection failed. Check your internet connection.".to_string()
            } else if msg.contains("Timeout") {
                "Request timed out. The server may be slow or unreachable.".to_string()
            } else {
                format!("Network error: {e}")
            }
        },
        HfError::Io(e) => {
            format!("I/O error: {e}")
        },
        HfError::Json(e) => {
            format!("Failed to parse response: {e}")
        },
        HfError::Url(e) => {
            format!("Invalid URL: {e}")
        },
        HfError::InvalidRepoType { expected, actual } => {
            format!("Invalid repository type: expected {expected:?}, got {actual:?}")
        },
        HfError::Other(msg) => msg.clone(),
    }
}

const XET_CRATES: &[&str] = &["hf_xet", "xet_client", "xet_core_structures", "xet_data", "xet_runtime"];

fn init_logging(color: bool) {
    let mut filter_str = if let Ok(level) = std::env::var("HF_LOG_LEVEL") {
        level
    } else if std::env::var("HF_DEBUG").is_ok() {
        "debug,h2=off,hyper_util=off,hyper=off".to_string()
    } else {
        "warn".to_string()
    };

    if let Ok(xet_level) = std::env::var("HF_XET_LOG_LEVEL") {
        for crate_name in XET_CRATES {
            filter_str.push_str(&format!(",{crate_name}={xet_level}"));
        }
    }

    let filter = EnvFilter::try_new(&filter_str).unwrap_or_else(|_| {
        eprintln!("Warning: invalid log filter '{filter_str}'. Valid levels: error, warn, info, debug, trace");
        EnvFilter::new("off")
    });

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_ansi(color)
        .with_writer(std::io::stderr)
        .init();
}
