use anyhow::{anyhow, Result};
use clap::Args as ClapArgs;
use huggingface_hub::{HfApi, UpdateWebhookParams};
use serde_json::json;

use crate::output::CommandResult;

/// Update an existing webhook
#[derive(ClapArgs)]
pub struct Args {
    /// Webhook ID
    pub webhook_id: String,

    /// New webhook URL
    #[arg(long)]
    pub url: Option<String>,

    /// Items to watch, in "type:name" format (replaces existing)
    #[arg(long = "watch")]
    pub watch: Vec<String>,

    /// Domains to receive events for (replaces existing)
    #[arg(long = "domain")]
    pub domain: Vec<String>,

    /// New webhook secret
    #[arg(long)]
    pub secret: Option<String>,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let watched = if args.watch.is_empty() {
        None
    } else {
        let items = args
            .watch
            .iter()
            .map(|s| {
                let parts: Vec<&str> = s.splitn(2, ':').collect();
                if parts.len() != 2 {
                    return Err(anyhow!("invalid watch format {:?}, expected type:name", s));
                }
                Ok(json!({"type": parts[0], "name": parts[1]}))
            })
            .collect::<Result<Vec<_>>>()?;
        Some(items)
    };

    let domains = if args.domain.is_empty() {
        None
    } else {
        Some(args.domain)
    };

    let params = UpdateWebhookParams {
        webhook_id: args.webhook_id,
        url: args.url,
        watched,
        domains,
        secret: args.secret,
    };
    let w = api.update_webhook(&params).await?;
    Ok(CommandResult::Raw(w.id.unwrap_or_default()))
}
