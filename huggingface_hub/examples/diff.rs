//! Diff parsing: raw diff retrieval, string parsing, and streaming.
//!
//! Requires HF_TOKEN environment variable.
//! Run: cargo run -p huggingface-hub --example diff

use futures::StreamExt;
use huggingface_hub::diff::{parse_raw_diff, stream_raw_diff, GIT_EMPTY_TREE_HASH};
use huggingface_hub::{HFClient, RepoGetRawDiffParams, RepoGetRawDiffStreamParams, RepoListCommitsParams};

#[tokio::main]
async fn main() -> huggingface_hub::Result<()> {
    let api = HFClient::new()?;
    let repo = api.model("openai-community", "gpt2");

    // --- Parse a raw diff string ---

    let commits_stream = repo.list_commits(&RepoListCommitsParams::default())?;
    futures::pin_mut!(commits_stream);
    let mut recent_ids: Vec<String> = Vec::new();
    while let Some(Ok(commit)) = commits_stream.next().await {
        recent_ids.push(commit.id.clone());
        if recent_ids.len() >= 2 {
            break;
        }
    }

    if recent_ids.len() == 2 {
        let compare = format!("{}..{}", recent_ids[1], recent_ids[0]);
        let raw = repo
            .get_raw_diff(&RepoGetRawDiffParams::builder().compare(&compare).build())
            .await?;
        let diffs = parse_raw_diff(&raw).map_err(|e| huggingface_hub::HFError::Other(e.to_string()))?;
        println!("Parsed {compare}: {} files changed", diffs.len());
        for d in &diffs {
            println!("  {:?} {} ({} bytes, binary={})", d.status, d.file_path, d.new_file_size, d.is_binary);
        }
    }

    // --- Stream a diff against the empty tree (all files in HEAD) ---

    let compare = format!("{GIT_EMPTY_TREE_HASH}..main");
    let byte_stream = repo
        .get_raw_diff_stream(&RepoGetRawDiffStreamParams::builder().compare(&compare).build())
        .await?;

    let byte_stream = byte_stream.map(|r| r.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)));
    let mut diff_stream = stream_raw_diff(byte_stream);
    let mut count = 0;
    println!("\nStreaming all files in main (first 10):");
    while let Some(result) = diff_stream.next().await {
        match result {
            Ok(d) => println!("  {:?} {} ({} bytes)", d.status, d.file_path, d.new_file_size),
            Err(e) => eprintln!("  parse error: {e}"),
        }
        count += 1;
        if count >= 10 {
            break;
        }
    }

    Ok(())
}
