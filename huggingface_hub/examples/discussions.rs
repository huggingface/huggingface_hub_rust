//! Discussion operations: list, create, comment, and pull request management.
//!
//! Requires HF_TOKEN and the "discussions" feature.
//! Run: cargo run -p huggingface-hub --features discussions --example discussions

use huggingface_hub::{
    ChangeDiscussionStatusParams, CommentDiscussionParams, CreateDiscussionParams, CreateRepoParams, DeleteRepoParams,
    EditDiscussionCommentParams, GetDiscussionDetailsParams, GetRepoDiscussionsParams, HfApi, RenameDiscussionParams,
};

#[tokio::main]
async fn main() -> huggingface_hub::Result<()> {
    let api = HfApi::new()?;

    // --- Read operations ---

    let discussions = api
        .get_repo_discussions(&GetRepoDiscussionsParams::builder().repo_id("gpt2").build())
        .await?;
    println!("Discussions in gpt2: {} (total: {:?})", discussions.discussions.len(), discussions.count);

    if let Some(first) = discussions.discussions.first() {
        let details = api
            .get_discussion_details(
                &GetDiscussionDetailsParams::builder()
                    .repo_id("gpt2")
                    .discussion_num(first.num)
                    .build(),
            )
            .await?;
        println!("Discussion #{:?}: {:?} (status: {:?})", details.num, details.title, details.status);
    }

    // --- Write operations ---

    let user = api.whoami().await?;
    let unique = std::process::id();
    let repo_name = format!("{}/example-discussions-{unique}", user.username);

    api.create_repo(
        &CreateRepoParams::builder()
            .repo_id(&repo_name)
            .private(true)
            .exist_ok(true)
            .build(),
    )
    .await?;
    println!("\nCreated test repo: {repo_name}");

    let discussion = api
        .create_discussion(
            &CreateDiscussionParams::builder()
                .repo_id(&repo_name)
                .title("Example Discussion")
                .description("Created by Rust example")
                .build(),
        )
        .await?;
    let discussion_num = discussion.num.expect("created discussion should have a num");
    println!("Created discussion #{discussion_num}");

    let comment = api
        .comment_discussion(
            &CommentDiscussionParams::builder()
                .repo_id(&repo_name)
                .discussion_num(discussion_num)
                .comment("This is a test comment")
                .build(),
        )
        .await?;
    let comment_id = comment.id.expect("comment should have an id");
    println!("Added comment: {comment_id}");

    let edited = api
        .edit_discussion_comment(
            &EditDiscussionCommentParams::builder()
                .repo_id(&repo_name)
                .discussion_num(discussion_num)
                .comment_id(&comment_id)
                .new_content("Edited test comment")
                .build(),
        )
        .await?;
    println!("Edited comment: {:?}", edited.id);

    let renamed = api
        .rename_discussion(
            &RenameDiscussionParams::builder()
                .repo_id(&repo_name)
                .discussion_num(discussion_num)
                .new_title("Renamed Example Discussion")
                .build(),
        )
        .await?;
    println!("Renamed discussion: {:?}", renamed.title);

    api.change_discussion_status(
        &ChangeDiscussionStatusParams::builder()
            .repo_id(&repo_name)
            .discussion_num(discussion_num)
            .new_status("closed")
            .build(),
    )
    .await?;
    println!("Closed discussion");

    api.delete_repo(&DeleteRepoParams::builder().repo_id(&repo_name).missing_ok(true).build())
        .await?;
    println!("Cleaned up test repo");

    Ok(())
}
