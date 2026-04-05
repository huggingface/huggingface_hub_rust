use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::HFClient;
use serde_json::json;

use crate::cli::OutputFormat;
use crate::output::{CommandOutput, CommandResult};

/// Show the currently authenticated user
#[derive(ClapArgs)]
pub struct Args {
    /// Output format
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,
}

pub async fn execute(api: &HFClient, args: Args) -> Result<CommandResult> {
    let user = api.whoami().await?;
    let json_value = json!({
        "username": user.username,
        "fullname": user.fullname,
        "email": user.email,
    });
    let output = CommandOutput::single_item(json_value);
    Ok(CommandResult::Formatted {
        output,
        format: args.format,
        quiet: false,
    })
}
