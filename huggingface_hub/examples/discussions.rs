//! Discussion operations: list, create, comment, and pull request management.
//!
//! Requires HF_TOKEN and the "discussions" feature.
//! Run: cargo run -p huggingface-hub --features discussions --example discussions

use huggingface_hub::{
    CreateRepoParams, DeleteRepoParams, HFClient, RepoChangeDiscussionStatusParams, RepoCommentDiscussionParams,
    RepoCreateDiscussionParams, RepoDiscussionDetailsParams, RepoEditDiscussionCommentParams,
    RepoListDiscussionsParams, RepoRenameDiscussionParams,
};

#[tokio::main]
async fn main() -> huggingface_hub::Result<()> {
    let api = HFClient::new()?;

    // --- Read operations ---

    let repo = api.model("openai-community", "gpt2");
    let discussions = repo.list_discussions(&RepoListDiscussionsParams::default()).await?;
    println!("Discussions in gpt2: {} (total: {:?})", discussions.discussions.len(), discussions.count);

    if let Some(first) = discussions.discussions.first() {
        let details = repo
            .discussion_details(&RepoDiscussionDetailsParams::builder().discussion_num(first.num).build())
            .await?;
        println!("Discussion #{}: {:?} (status: {:?})", details.num, details.title, details.status);
    }

    // --- Write operations ---

    let user = api.whoami().await?;
    let unique = std::process::id();
    let repo = api.model(&user.username, format!("example-discussions-{unique}"));

    api.create_repo(
        &CreateRepoParams::builder()
            .repo_id(repo.repo_path())
            .private(true)
            .exist_ok(true)
            .build(),
    )
    .await?;
    println!("\nCreated test repo: {}", repo.repo_path());

    let discussion = repo
        .create_discussion(
            &RepoCreateDiscussionParams::builder()
                .title("Example Discussion")
                .description("Created by Rust example")
                .build(),
        )
        .await?;
    println!("Created discussion #{}", discussion.num);

    let comment = repo
        .comment_discussion(
            &RepoCommentDiscussionParams::builder()
                .discussion_num(discussion.num)
                .comment("This is a test comment")
                .build(),
        )
        .await?;
    let comment_id = comment.id.expect("comment should have an id");
    println!("Added comment: {comment_id}");

    let edited = repo
        .edit_discussion_comment(
            &RepoEditDiscussionCommentParams::builder()
                .discussion_num(discussion.num)
                .comment_id(&comment_id)
                .new_content("Edited test comment")
                .build(),
        )
        .await?;
    println!("Edited comment: {:?}", edited.id);

    let renamed = repo
        .rename_discussion(
            &RepoRenameDiscussionParams::builder()
                .discussion_num(discussion.num)
                .new_title("Renamed Example Discussion")
                .build(),
        )
        .await?;
    println!("Renamed discussion: {:?}", renamed.title);

    repo.change_discussion_status(
        &RepoChangeDiscussionStatusParams::builder()
            .discussion_num(discussion.num)
            .new_status("closed")
            .build(),
    )
    .await?;
    println!("Closed discussion");

    api.delete_repo(&DeleteRepoParams::builder().repo_id(repo.repo_path()).missing_ok(true).build())
        .await?;
    println!("Cleaned up test repo");

    Ok(())
}
