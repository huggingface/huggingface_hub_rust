//! Like operations: like/unlike repos, list liked repos and likers.
//!
//! Requires HF_TOKEN and the "likes" feature.
//! Run: cargo run -p huggingface-hub --features likes --example likes

use futures::StreamExt;
use huggingface_hub::{HfApi, LikeParams, ListLikedReposParams, ListRepoLikersParams};

#[tokio::main]
async fn main() -> huggingface_hub::Result<()> {
    let api = HfApi::new()?;

    let user = api.whoami().await?;
    let liked = api
        .list_liked_repos(
            &ListLikedReposParams::builder()
                .username(&user.username)
                .build(),
        )
        .await?;
    println!("Liked repos by {}:", user.username);
    for repo in liked.iter().take(5) {
        println!("  - {:?}", repo);
    }

    let likers_stream =
        api.list_repo_likers(&ListRepoLikersParams::builder().repo_id("gpt2").build());
    futures::pin_mut!(likers_stream);
    println!("\nLikers of gpt2:");
    let mut count = 0;
    while let Some(Ok(liker)) = likers_stream.next().await {
        println!("  - {}", liker.username);
        count += 1;
        if count >= 3 {
            break;
        }
    }

    api.like(&LikeParams::builder().repo_id("gpt2").build())
        .await?;
    println!("\nLiked gpt2");

    api.unlike(&LikeParams::builder().repo_id("gpt2").build())
        .await?;
    println!("Unliked gpt2");

    Ok(())
}
