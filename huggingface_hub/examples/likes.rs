//! Like operations: like/unlike repos, list liked repos and likers.
//!
//! Requires HF_TOKEN and the "likes" feature.
//! Run: cargo run -p huggingface-hub --features likes --example likes

use futures::StreamExt;
use huggingface_hub::types::LikeParams;
use huggingface_hub::{HFClient, ListLikedReposParams};

#[tokio::main]
async fn main() -> huggingface_hub::Result<()> {
    let api = HFClient::new()?;
    let repo = api.model("openai-community", "gpt2");

    let user = api.whoami().await?;
    let liked = api
        .list_liked_repos(&ListLikedReposParams::builder().username(&user.username).build())
        .await?;
    println!("Liked repos by {}:", user.username);
    for repo in liked.iter().take(5) {
        println!("  - {:?}", repo);
    }

    let likers_stream = repo.list_likers(None)?;
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

    let like_params = LikeParams::builder().repo_id(repo.repo_path()).build();
    repo.like(&like_params).await?;
    println!("\nLiked gpt2");

    repo.unlike(&like_params).await?;
    println!("Unliked gpt2");

    Ok(())
}
