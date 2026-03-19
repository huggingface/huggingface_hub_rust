//! Webhook operations: list, create, update, enable/disable, and delete.
//!
//! Requires HF_TOKEN and the "webhooks" feature.
//! Run: cargo run -p huggingface-hub --features webhooks --example webhooks

use huggingface_hub::{CreateWebhookParams, HfApi, UpdateWebhookParams};

#[tokio::main]
async fn main() -> huggingface_hub::Result<()> {
    let api = HfApi::new()?;

    // --- Read operations ---

    let webhooks = api.list_webhooks().await?;
    println!("Current webhooks: {}", webhooks.len());
    for wh in &webhooks {
        println!("  - {:?} -> {:?}", wh.id, wh.url);
    }

    // --- Write operations ---

    let user = api.whoami().await?;
    let webhook = api
        .create_webhook(
            &CreateWebhookParams::builder()
                .url("https://example.com/webhook")
                .watched(vec![serde_json::json!({
                    "type": "user",
                    "name": user.username,
                })])
                .domains(vec!["repo".to_string()])
                .build(),
        )
        .await?;
    let webhook_id = webhook.id.as_deref().expect("webhook should have an id");
    println!("\nCreated webhook: {webhook_id}");

    let fetched = api.get_webhook(webhook_id).await?;
    println!("Fetched webhook: {:?} -> {:?}", fetched.id, fetched.url);

    let updated = api
        .update_webhook(
            &UpdateWebhookParams::builder()
                .webhook_id(webhook_id)
                .url("https://example.com/webhook-updated")
                .build(),
        )
        .await?;
    println!("Updated webhook URL: {:?}", updated.url);

    let disabled = api.disable_webhook(webhook_id).await?;
    println!("Disabled webhook: {:?}", disabled.disabled);

    let enabled = api.enable_webhook(webhook_id).await?;
    println!("Enabled webhook: {:?}", enabled.disabled);

    api.delete_webhook(webhook_id).await?;
    println!("Deleted webhook");

    Ok(())
}
