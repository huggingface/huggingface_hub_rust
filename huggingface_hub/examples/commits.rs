//! Commit operations: listing commits, refs, diffs, and branch/tag management.
//!
//! Requires HF_TOKEN environment variable.
//! Run: cargo run -p huggingface-hub --example commits

use futures::StreamExt;
use huggingface_hub::{
    CreateBranchParams, CreateRepoParams, CreateTagParams, DeleteBranchParams, DeleteRepoParams,
    DeleteTagParams, GetCommitDiffParams, GetRawDiffParams, HfApi, ListRepoCommitsParams,
    ListRepoRefsParams,
};

#[tokio::main]
async fn main() -> huggingface_hub::Result<()> {
    let api = HfApi::new()?;

    // --- Read operations ---

    let commits_stream =
        api.list_repo_commits(&ListRepoCommitsParams::builder().repo_id("gpt2").build());
    futures::pin_mut!(commits_stream);
    println!("Recent commits in gpt2:");
    let mut first_two_ids: Vec<String> = Vec::new();
    let mut count = 0;
    while let Some(Ok(commit)) = commits_stream.next().await {
        println!("  {} - {}", &commit.id[..8], commit.title);
        if first_two_ids.len() < 2 {
            first_two_ids.push(commit.id.clone());
        }
        count += 1;
        if count >= 5 {
            break;
        }
    }

    let refs = api
        .list_repo_refs(&ListRepoRefsParams::builder().repo_id("gpt2").build())
        .await?;
    println!("\nBranches:");
    for b in &refs.branches {
        println!("  {} -> {}", b.name, &b.target_commit[..8]);
    }
    println!("Tags:");
    for t in &refs.tags {
        println!("  {} -> {}", t.name, &t.target_commit[..8]);
    }

    if first_two_ids.len() == 2 {
        let compare = format!("{}..{}", first_two_ids[1], first_two_ids[0]);
        let diff = api
            .get_commit_diff(
                &GetCommitDiffParams::builder()
                    .repo_id("gpt2")
                    .compare(&compare)
                    .build(),
            )
            .await?;
        println!("\nDiff ({compare}):");
        println!("  {} chars", diff.len());

        let raw_diff = api
            .get_raw_diff(
                &GetRawDiffParams::builder()
                    .repo_id("gpt2")
                    .compare(&compare)
                    .build(),
            )
            .await?;
        println!("Raw diff: {} chars", raw_diff.len());
    }

    // --- Write operations (creates real resources on the Hub) ---

    let user = api.whoami().await?;
    let unique = std::process::id();
    let repo_name = format!("{}/example-commits-{unique}", user.username);

    api.create_repo(
        &CreateRepoParams::builder()
            .repo_id(&repo_name)
            .private(true)
            .exist_ok(true)
            .build(),
    )
    .await?;
    println!("\nCreated test repo: {repo_name}");

    api.create_branch(
        &CreateBranchParams::builder()
            .repo_id(&repo_name)
            .branch("feature-branch")
            .build(),
    )
    .await?;
    println!("Created branch: feature-branch");

    api.delete_branch(
        &DeleteBranchParams::builder()
            .repo_id(&repo_name)
            .branch("feature-branch")
            .build(),
    )
    .await?;
    println!("Deleted branch: feature-branch");

    api.create_tag(
        &CreateTagParams::builder()
            .repo_id(&repo_name)
            .tag("v0.1.0")
            .message("Initial release")
            .build(),
    )
    .await?;
    println!("Created tag: v0.1.0");

    api.delete_tag(
        &DeleteTagParams::builder()
            .repo_id(&repo_name)
            .tag("v0.1.0")
            .build(),
    )
    .await?;
    println!("Deleted tag: v0.1.0");

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
