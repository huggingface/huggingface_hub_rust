//! Integration tests for downloading files from the Hub.
//!
//! Tests regular (non-xet) HTTP downloads of small files.
//! Requires HF_TOKEN environment variable.
//!
//! Run: source ~/hf/prod_token && cargo test -p huggingface-hub --test download_test

use huggingface_hub::repository::HFRepository;
use huggingface_hub::{HfApi, HfApiBuilder, RepoDownloadFileParams};
use sha2::{Digest, Sha256};

fn api() -> Option<HfApi> {
    if std::env::var("HF_TOKEN").is_err() {
        return None;
    }
    Some(HfApiBuilder::new().build().expect("Failed to create HfApi"))
}

fn model(api: &HfApi, owner: &str, name: &str) -> HFRepository {
    api.model(owner, name)
}

fn dataset(api: &HfApi, owner: &str, name: &str) -> HFRepository {
    api.dataset(owner, name)
}

#[tokio::test]
async fn test_download_small_json_file() {
    let Some(api) = api() else { return };
    let dir = tempfile::tempdir().unwrap();

    let path = model(&api, "", "gpt2")
        .download_file(
            &RepoDownloadFileParams::builder()
                .filename("config.json")
                .local_dir(dir.path().to_path_buf())
                .build(),
        )
        .await
        .unwrap();

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

    let path = model(&api, "openai-community", "gpt2")
        .download_file(
            &RepoDownloadFileParams::builder()
                .filename("config.json")
                .local_dir(dir.path().to_path_buf())
                .build(),
        )
        .await
        .unwrap();

    assert_eq!(path, dir.path().join("config.json"));
    assert!(path.exists());
}

#[tokio::test]
async fn test_download_with_specific_revision() {
    let Some(api) = api() else { return };
    let dir = tempfile::tempdir().unwrap();

    let path = model(&api, "openai-community", "gpt2")
        .download_file(
            &RepoDownloadFileParams::builder()
                .filename("config.json")
                .local_dir(dir.path().to_path_buf())
                .revision("main")
                .build(),
        )
        .await
        .unwrap();

    assert!(path.exists());
    let content = std::fs::read_to_string(&path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(json["model_type"], "gpt2");
}

#[tokio::test]
async fn test_download_dataset_file() {
    let Some(api) = api() else { return };
    let dir = tempfile::tempdir().unwrap();

    let path = dataset(&api, "rajpurkar", "squad")
        .download_file(
            &RepoDownloadFileParams::builder()
                .filename("README.md")
                .local_dir(dir.path().to_path_buf())
                .build(),
        )
        .await
        .unwrap();

    assert!(path.exists());
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("SQuAD") || content.contains("squad"));
}

#[tokio::test]
async fn test_download_nonexistent_file_returns_error() {
    let Some(api) = api() else { return };
    let dir = tempfile::tempdir().unwrap();

    let result = model(&api, "", "gpt2")
        .download_file(
            &RepoDownloadFileParams::builder()
                .filename("this_file_does_not_exist_at_all.bin")
                .local_dir(dir.path().to_path_buf())
                .build(),
        )
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_download_from_nonexistent_repo_returns_error() {
    let Some(api) = api() else { return };
    let dir = tempfile::tempdir().unwrap();

    let result = model(&api, "this-user-does-not-exist-99999", "this-repo-does-not-exist")
        .download_file(
            &RepoDownloadFileParams::builder()
                .filename("anything.txt")
                .local_dir(dir.path().to_path_buf())
                .build(),
        )
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_download_multiple_files_to_same_dir() {
    let Some(api) = api() else { return };
    let dir = tempfile::tempdir().unwrap();
    let repo = model(&api, "", "gpt2");

    for filename in &["config.json", "README.md"] {
        let path = repo
            .download_file(
                &RepoDownloadFileParams::builder()
                    .filename(*filename)
                    .local_dir(dir.path().to_path_buf())
                    .build(),
            )
            .await
            .unwrap();
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
    let repo = model(&api, "", "gpt2");

    for dir in [&dir1, &dir2] {
        repo.download_file(
            &RepoDownloadFileParams::builder()
                .filename("config.json")
                .local_dir(dir.path().to_path_buf())
                .build(),
        )
        .await
        .unwrap();
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

    model(&api, "", "gpt2")
        .download_file(
            &RepoDownloadFileParams::builder()
                .filename("config.json")
                .local_dir(dir.path().to_path_buf())
                .build(),
        )
        .await
        .unwrap();

    let content = std::fs::read_to_string(&dest).unwrap();
    assert_ne!(content, "old content");
    assert!(content.contains("gpt2"));
}
