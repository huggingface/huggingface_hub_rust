//! Integration tests for downloading files from the Hub.
//!
//! Tests regular (non-xet) HTTP downloads of small files.
//! Requires HF_TOKEN environment variable.
//!
//! Run: source ~/hf/prod_token && cargo test -p huggingface-hub --test download_test

use huggingface_hub::types::{DownloadFileParams, RepoType};
use huggingface_hub::{HfApi, HfApiBuilder};
use sha2::{Digest, Sha256};

fn api() -> Option<HfApi> {
    if std::env::var("HF_TOKEN").is_err() {
        return None;
    }
    Some(HfApiBuilder::new().build().expect("Failed to create HfApi"))
}

#[tokio::test]
async fn test_download_small_json_file() {
    let Some(api) = api() else { return };
    let dir = tempfile::tempdir().unwrap();

    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("config.json")
        .local_dir(dir.path().to_path_buf())
        .build();
    let path = api.download_file(&params).await.unwrap();

    assert!(path.exists());
    let content = std::fs::read_to_string(&path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(json["model_type"], "gpt2");
    assert!(json["vocab_size"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn test_download_preserves_subdirectory_structure() {
    let Some(api) = api() else { return };
    let dir = tempfile::tempdir().unwrap();

    let params = DownloadFileParams::builder()
        .repo_id("openai-community/gpt2")
        .filename("config.json")
        .local_dir(dir.path().to_path_buf())
        .build();
    let path = api.download_file(&params).await.unwrap();

    assert_eq!(path, dir.path().join("config.json"));
    assert!(path.exists());
}

#[tokio::test]
async fn test_download_with_specific_revision() {
    let Some(api) = api() else { return };
    let dir = tempfile::tempdir().unwrap();

    let params = DownloadFileParams::builder()
        .repo_id("openai-community/gpt2")
        .filename("config.json")
        .local_dir(dir.path().to_path_buf())
        .revision("main")
        .build();
    let path = api.download_file(&params).await.unwrap();

    assert!(path.exists());
    let content = std::fs::read_to_string(&path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(json["model_type"], "gpt2");
}

#[tokio::test]
async fn test_download_dataset_file() {
    let Some(api) = api() else { return };
    let dir = tempfile::tempdir().unwrap();

    let params = DownloadFileParams::builder()
        .repo_id("rajpurkar/squad")
        .filename("README.md")
        .local_dir(dir.path().to_path_buf())
        .repo_type(RepoType::Dataset)
        .build();
    let path = api.download_file(&params).await.unwrap();

    assert!(path.exists());
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("SQuAD") || content.contains("squad"));
}

#[tokio::test]
async fn test_download_nonexistent_file_returns_error() {
    let Some(api) = api() else { return };
    let dir = tempfile::tempdir().unwrap();

    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("this_file_does_not_exist_at_all.bin")
        .local_dir(dir.path().to_path_buf())
        .build();
    let result = api.download_file(&params).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_download_from_nonexistent_repo_returns_error() {
    let Some(api) = api() else { return };
    let dir = tempfile::tempdir().unwrap();

    let params = DownloadFileParams::builder()
        .repo_id("this-user-does-not-exist-99999/this-repo-does-not-exist")
        .filename("anything.txt")
        .local_dir(dir.path().to_path_buf())
        .build();
    let result = api.download_file(&params).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_download_multiple_files_to_same_dir() {
    let Some(api) = api() else { return };
    let dir = tempfile::tempdir().unwrap();

    let files = ["config.json", "README.md"];
    for filename in &files {
        let params = DownloadFileParams::builder()
            .repo_id("gpt2")
            .filename(*filename)
            .local_dir(dir.path().to_path_buf())
            .build();
        let path = api.download_file(&params).await.unwrap();
        assert!(path.exists());
    }

    assert!(dir.path().join("config.json").exists());
    assert!(dir.path().join("README.md").exists());
}

#[tokio::test]
async fn test_download_file_content_is_deterministic() {
    let Some(api) = api() else { return };
    let dir1 = tempfile::tempdir().unwrap();
    let dir2 = tempfile::tempdir().unwrap();

    for dir in [&dir1, &dir2] {
        let params = DownloadFileParams::builder()
            .repo_id("gpt2")
            .filename("config.json")
            .local_dir(dir.path().to_path_buf())
            .build();
        api.download_file(&params).await.unwrap();
    }

    let content1 = std::fs::read(dir1.path().join("config.json")).unwrap();
    let content2 = std::fs::read(dir2.path().join("config.json")).unwrap();

    let hash1 = Sha256::digest(&content1);
    let hash2 = Sha256::digest(&content2);
    assert_eq!(hash1, hash2);
}

#[tokio::test]
async fn test_download_overwrites_existing_file() {
    let Some(api) = api() else { return };
    let dir = tempfile::tempdir().unwrap();

    let dest = dir.path().join("config.json");
    std::fs::write(&dest, "old content").unwrap();

    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("config.json")
        .local_dir(dir.path().to_path_buf())
        .build();
    api.download_file(&params).await.unwrap();

    let content = std::fs::read_to_string(&dest).unwrap();
    assert_ne!(content, "old content");
    assert!(content.contains("gpt2"));
}
