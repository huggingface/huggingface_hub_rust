//! Access request operations: list pending/accepted/rejected requests.
//!
//! Requires HF_TOKEN and the "access_requests" feature.
//! Run: cargo run -p huggingface-hub --features access_requests --example access_requests

use huggingface_hub::types::ListAccessRequestsParams;
use huggingface_hub::{CreateRepoParams, DeleteRepoParams, HFClient, RepoUpdateSettingsParams};

#[tokio::main]
async fn main() -> huggingface_hub::Result<()> {
    let api = HFClient::new()?;

    let user = api.whoami().await?;
    let unique = std::process::id();
    let repo = api.model(&user.username, format!("example-gated-{unique}"));

    api.create_repo(
        &CreateRepoParams::builder()
            .repo_id(repo.repo_path())
            .private(true)
            .exist_ok(true)
            .build(),
    )
    .await?;

    repo.update_settings(&RepoUpdateSettingsParams::builder().gated("auto").build())
        .await?;
    println!("Created gated repo: {}", repo.repo_path());

    let access_params = ListAccessRequestsParams::builder().repo_id(repo.repo_path()).build();

    let pending = repo.list_pending_access_requests(&access_params).await?;
    println!("Pending requests: {}", pending.len());

    let accepted = repo.list_accepted_access_requests(&access_params).await?;
    println!("Accepted requests: {}", accepted.len());

    let rejected = repo.list_rejected_access_requests(&access_params).await?;
    println!("Rejected requests: {}", rejected.len());

    api.delete_repo(&DeleteRepoParams::builder().repo_id(repo.repo_path()).missing_ok(true).build())
        .await?;
    println!("Cleaned up test repo");

    Ok(())
}
