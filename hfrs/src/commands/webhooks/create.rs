use anyhow::{anyhow, Result};
use clap::Args as ClapArgs;
use huggingface_hub::{CreateWebhookParams, HfApi};
use serde_json::json;

use crate::output::CommandResult;

/// Create a new webhook
#[derive(ClapArgs)]
pub struct Args {
    /// Webhook URL
    #[arg(long, required = true)]
    pub url: String,

    /// Items to watch, in "type:name" format (e.g. "user:myuser")
    #[arg(long = "watch", required = true)]
    pub watch: Vec<String>,

    /// Domains to receive events for (e.g. "repo", "discussion")
    #[arg(long = "domain")]
    pub domain: Vec<String>,

    /// Webhook secret
    #[arg(long)]
    pub secret: Option<String>,
}

pub async fn execute(api: &HfApi, args: Args) -> Result<CommandResult> {
    let watched = args
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

    let domains = if args.domain.is_empty() {
        None
    } else {
        Some(args.domain)
    };

    let params = CreateWebhookParams {
        url: args.url,
        watched,
        domains,
        secret: args.secret,
    };
    let w = api.create_webhook(&params).await?;
    Ok(CommandResult::Raw(w.id.unwrap_or_default()))
}
