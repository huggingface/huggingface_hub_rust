use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::HfApi;
use serde_json::json;

use crate::cli::OutputFormat;
use crate::output::{CommandOutput, CommandResult};

/// Show details of a webhook
#[derive(ClapArgs)]
pub struct Args {
    /// Webhook ID
    pub webhook_id: String,

    /// Output format
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let w = api.get_webhook(&args.webhook_id).await?;
    let json_value = json!({
        "id": w.id,
        "url": w.url,
        "domains": w.domains,
        "watched": w.watched.iter().map(|wi| json!({"type": wi.item_type, "name": wi.name})).collect::<Vec<_>>(),
        "secret": w.secret,
        "disabled": w.disabled,
    });
    let output = CommandOutput::single_item(json_value);
    Ok(CommandResult::Formatted {
        output,
        format: args.format,
        quiet: false,
    })
}
