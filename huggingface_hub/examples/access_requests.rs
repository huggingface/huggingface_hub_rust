//! Access request operations: list pending/accepted/rejected requests.
//!
//! Requires HF_TOKEN and the "access_requests" feature.
//! Run: cargo run -p huggingface-hub --features access_requests --example access_requests

use huggingface_hub::{
    CreateRepoParams, DeleteRepoParams, HfApi, ListAccessRequestsParams, UpdateRepoParams,
};

#[tokio::main]
async fn main() -> huggingface_hub::Result<()> {
    let api = HfApi::new()?;

    let user = api.whoami().await?;
    let unique = std::process::id();
    let repo_name = format!("{}/example-gated-{unique}", user.username);

    api.create_repo(
        &CreateRepoParams::builder()
            .repo_id(&repo_name)
            .private(true)
            .exist_ok(true)
            .build(),
    )
    .await?;

    api.update_repo_settings(
        &UpdateRepoParams::builder()
            .repo_id(&repo_name)
            .gated("auto")
            .build(),
    )
    .await?;
    println!("Created gated repo: {repo_name}");

    let params = ListAccessRequestsParams::builder()
        .repo_id(&repo_name)
        .build();

    let pending = api.list_pending_access_requests(&params).await?;
    println!("Pending requests: {}", pending.len());

    let accepted = api.list_accepted_access_requests(&params).await?;
    println!("Accepted requests: {}", accepted.len());

    let rejected = api.list_rejected_access_requests(&params).await?;
    println!("Rejected requests: {}", rejected.len());

    api.delete_repo(
        &DeleteRepoParams::builder()
            .repo_id(&repo_name)
            .missing_ok(true)
            .build(),
    )
    .await?;
    println!("Cleaned up test repo");

    Ok(())
}
