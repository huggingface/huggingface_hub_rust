use anyhow::Result;
use clap::Args as ClapArgs;

use crate::output::CommandResult;

#[derive(ClapArgs)]
#[command(about = "Print information about the environment")]
pub struct Args {}

pub async fn execute(_args: Args) -> Result<CommandResult> {
    let mut lines = Vec::new();
    lines.push(format!("hfrs version: {}", env!("CARGO_PKG_VERSION")));
    lines.push(format!("Platform: {} {}", std::env::consts::OS, std::env::consts::ARCH));
    if let Ok(endpoint) = std::env::var("HF_ENDPOINT") {
        lines.push(format!("HF_ENDPOINT: {endpoint}"));
    }
    if std::env::var("HF_TOKEN").is_ok() {
        lines.push("HF_TOKEN: set".to_string());
    } else {
        lines.push("HF_TOKEN: not set".to_string());
    }
    if let Ok(home) = std::env::var("HF_HOME") {
        lines.push(format!("HF_HOME: {home}"));
    }
    if let Ok(cache) = std::env::var("HF_HUB_CACHE") {
        lines.push(format!("HF_HUB_CACHE: {cache}"));
    }
    let log_level = if let Ok(level) = std::env::var("HF_LOG_LEVEL") {
        level
    } else if std::env::var("HF_DEBUG").is_ok() {
        "debug (via HF_DEBUG)".to_string()
    } else {
        "warn (default)".to_string()
    };
    lines.push(format!("HF_LOG_LEVEL: {log_level}"));
    if std::env::var("HF_DEBUG").is_ok() {
        lines.push("HF_DEBUG: set".to_string());
    }
    Ok(CommandResult::Raw(lines.join("\n")))
}
