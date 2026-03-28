use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::HfApi;
use serde_json::json;

use crate::cli::OutputFormat;
use crate::output::{CommandOutput, CommandResult};

/// List webhooks
#[derive(ClapArgs)]
pub struct Args {
    /// Output format
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,

    /// Print only webhook IDs
    #[arg(long)]
    pub quiet: bool,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let webhooks = api.list_webhooks().await?;

    let headers = vec!["ID".to_string(), "URL".to_string(), "Domains".to_string()];

    let rows = webhooks
        .iter()
        .map(|w| {
            vec![
                w.id.clone().unwrap_or_default(),
                w.url.clone().unwrap_or_default(),
                w.domains.join(", "),
            ]
        })
        .collect();

    let quiet_values = webhooks.iter().map(|w| w.id.clone().unwrap_or_default()).collect();

    let json_value: serde_json::Value = webhooks
        .iter()
        .map(|w| {
            json!({
                "id": w.id,
                "url": w.url,
                "domains": w.domains,
                "watched": w.watched.iter().map(|wi| json!({"type": wi.item_type, "name": wi.name})).collect::<Vec<_>>(),
                "disabled": w.disabled,
            })
        })
        .collect::<Vec<_>>()
        .into();

    let output = CommandOutput {
        headers,
        rows,
        json_value,
        quiet_values,
    };
    Ok(CommandResult::Formatted {
        output,
        format: args.format,
        quiet: args.quiet,
    })
}
