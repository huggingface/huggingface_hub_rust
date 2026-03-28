mod cli;
mod commands;
mod output;
mod util;

use std::process::ExitCode;

use clap::Parser;
use cli::{Cli, Command};
use huggingface_hub::{HfApiBuilder, HfError};
use output::render;
use util::token::read_active_token;

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    let token = cli
        .token
        .clone()
        .or_else(|| std::env::var("HF_TOKEN").ok())
        .or_else(read_active_token);

    let mut builder = HfApiBuilder::new();
    if let Some(t) = token {
        builder = builder.token(t);
    }
    if let Some(endpoint) = cli.endpoint.clone() {
        builder = builder.endpoint(endpoint);
    }
    let api = match builder.build() {
        Ok(api) => api,
        Err(e) => {
            let message = format_hf_error(&e);
            eprintln!("Error: {message}");
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
            print_anyhow_error(&e);
            ExitCode::FAILURE
        },
    }
}

fn print_anyhow_error(err: &anyhow::Error) {
    let message = format_anyhow_error(err);
    eprintln!("Error: {message}");

    if std::env::var("HF_DEBUG").is_ok() {
        // Print the full error chain
        for cause in err.chain().skip(1) {
            eprintln!("  Caused by: {cause}");
        }
    } else {
        eprintln!("\x1b[90mSet HF_DEBUG=1 for the full error trace.\x1b[0m");
    }
}

fn format_anyhow_error(err: &anyhow::Error) -> String {
    // Try to downcast to HfError for nice messages
    if let Some(hf_err) = err.downcast_ref::<HfError>() {
        return format_hf_error(hf_err);
    }

    // Walk the chain looking for HfError
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
                        // Try to extract a clean error message from the JSON body
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
        HfError::Other(msg) => msg.clone(),
    }
}
