//! Integration tests for downloading files from the Hub.
//!
//! Tests regular (non-xet) HTTP downloads of small files.
//! Requires HF_TOKEN environment variable.
//!
//! Run: source ~/hf/prod_token && cargo test -p huggingface-hub --test download_test

use futures::StreamExt;
use huggingface_hub::repository::HFRepository;
use huggingface_hub::{HFClient, HFClientBuilder, RepoDownloadFileParams, RepoDownloadFileStreamParams};
use sha2::{Digest, Sha256};

fn api() -> Option<HFClient> {
    if std::env::var("HF_TOKEN").is_err() {
        return None;
    }
    Some(HFClientBuilder::new().build().expect("Failed to create HFClient"))
}

fn is_hub_ci() -> bool {
    std::env::var("HF_ENDPOINT")
        .ok()
        .is_some_and(|v| v.contains("hub-ci.huggingface.co"))
}

fn test_model_parts() -> (&'static str, &'static str) {
    if is_hub_ci() {
        ("huggingface-hub-rust-test-user", "gpt2")
    } else {
        ("openai-community", "gpt2")
    }
}

fn test_dataset_parts() -> (&'static str, &'static str) {
    if is_hub_ci() {
        ("huggingface-hub-rust-test-user", "hacker-news")
    } else {
        ("rajpurkar", "squad")
    }
}

fn model(api: &HFClient, owner: &str, name: &str) -> HFRepository {
    api.model(owner, name)
}

fn dataset(api: &HFClient, owner: &str, name: &str) -> HFRepository {
    api.dataset(owner, name)
}

#[tokio::test]
async fn test_download_small_json_file() {
    let Some(api) = api() else { return };
    let dir = tempfile::tempdir().unwrap();
    let (owner, name) = test_model_parts();

    let path = model(&api, owner, name)
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
    assert!(json.get("model_type").is_some());
}

#[tokio::test]
async fn test_download_preserves_subdirectory_structure() {
    let Some(api) = api() else { return };
    let dir = tempfile::tempdir().unwrap();
    let (owner, name) = test_model_parts();

    let path = model(&api, owner, name)
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
    let (owner, name) = test_model_parts();

    let path = model(&api, owner, name)
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
    assert!(json.get("model_type").is_some());
}

#[tokio::test]
async fn test_download_dataset_file() {
    let Some(api) = api() else { return };
    let dir = tempfile::tempdir().unwrap();
    let (owner, name) = test_dataset_parts();

    let path = dataset(&api, owner, name)
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
    assert!(!content.is_empty());
}

#[tokio::test]
async fn test_download_nonexistent_file_returns_error() {
    let Some(api) = api() else { return };
    let dir = tempfile::tempdir().unwrap();
    let (owner, name) = test_model_parts();

    let result = model(&api, owner, name)
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
    let (owner, name) = test_model_parts();
    let repo = model(&api, owner, name);

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
    let (owner, name) = test_model_parts();
    let repo = model(&api, owner, name);

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
    let (owner, name) = test_model_parts();

    let dest = dir.path().join("config.json");
    std::fs::write(&dest, "old content").unwrap();

    model(&api, owner, name)
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
    assert!(content.contains("model_type"));
}

// --- Range / partial download tests (non-xet) ---

#[tokio::test]
async fn test_download_stream_full_file() {
    let Some(api) = api() else { return };
    let (owner, name) = test_model_parts();
    let repo = model(&api, owner, name);

    let (content_length, stream) = repo
        .download_file_stream(&RepoDownloadFileStreamParams::builder().filename("config.json").build())
        .await
        .unwrap();

    assert!(content_length.is_some());

    futures::pin_mut!(stream);
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        bytes.extend_from_slice(&chunk.unwrap());
    }

    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(json.get("model_type").is_some());
}

#[tokio::test]
async fn test_download_stream_range_first_bytes() {
    let Some(api) = api() else { return };
    let (owner, name) = test_model_parts();
    let repo = model(&api, owner, name);

    // Download just the first 20 bytes
    let (content_length, stream) = repo
        .download_file_stream(
            &RepoDownloadFileStreamParams::builder()
                .filename("config.json")
                .range(0..20u64)
                .build(),
        )
        .await
        .unwrap();

    assert!(content_length.unwrap() <= 20);

    futures::pin_mut!(stream);
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        bytes.extend_from_slice(&chunk.unwrap());
    }
    assert_eq!(bytes.len(), 20);
}

#[tokio::test]
async fn test_download_stream_range_middle_bytes() {
    let Some(api) = api() else { return };
    let (owner, name) = test_model_parts();
    let repo = model(&api, owner, name);

    // First download the full file for comparison
    let (_len, full_stream) = repo
        .download_file_stream(&RepoDownloadFileStreamParams::builder().filename("config.json").build())
        .await
        .unwrap();
    futures::pin_mut!(full_stream);
    let mut full_bytes = Vec::new();
    while let Some(chunk) = full_stream.next().await {
        full_bytes.extend_from_slice(&chunk.unwrap());
    }

    // Now download a range from the middle
    let start = 10u64;
    let end = 50u64;
    let (_len, range_stream) = repo
        .download_file_stream(
            &RepoDownloadFileStreamParams::builder()
                .filename("config.json")
                .range(start..end)
                .build(),
        )
        .await
        .unwrap();

    futures::pin_mut!(range_stream);
    let mut range_bytes = Vec::new();
    while let Some(chunk) = range_stream.next().await {
        range_bytes.extend_from_slice(&chunk.unwrap());
    }

    assert_eq!(range_bytes.len(), (end - start) as usize);
    assert_eq!(range_bytes, &full_bytes[start as usize..end as usize]);
}

#[tokio::test]
async fn test_download_stream_range_content_matches_full_download() {
    let Some(api) = api() else { return };
    let (owner, name) = test_model_parts();
    let repo = model(&api, owner, name);
    let dir = tempfile::tempdir().unwrap();

    // Download full file to disk for reference
    let path = repo
        .download_file(
            &RepoDownloadFileParams::builder()
                .filename("config.json")
                .local_dir(dir.path().to_path_buf())
                .build(),
        )
        .await
        .unwrap();
    let full_bytes = std::fs::read(&path).unwrap();

    // Stream the first 100 bytes
    let range_end = 100u64.min(full_bytes.len() as u64);
    let (_len, stream) = repo
        .download_file_stream(
            &RepoDownloadFileStreamParams::builder()
                .filename("config.json")
                .range(0..range_end)
                .build(),
        )
        .await
        .unwrap();

    futures::pin_mut!(stream);
    let mut streamed = Vec::new();
    while let Some(chunk) = stream.next().await {
        streamed.extend_from_slice(&chunk.unwrap());
    }

    assert_eq!(streamed, &full_bytes[..range_end as usize]);
}

// --- Progress tracking tests ---

use std::sync::{Arc, Mutex};

use huggingface_hub::{DownloadEvent, FileStatus, ProgressEvent, ProgressHandler};

struct RecordingHandler {
    events: Mutex<Vec<ProgressEvent>>,
}

impl RecordingHandler {
    fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
        }
    }

    fn events(&self) -> Vec<ProgressEvent> {
        self.events.lock().unwrap().clone()
    }
}

impl ProgressHandler for RecordingHandler {
    fn on_progress(&self, event: &ProgressEvent) {
        self.events.lock().unwrap().push(event.clone());
    }
}

#[tokio::test]
async fn test_download_file_with_progress_to_local_dir() {
    let Some(api) = api() else { return };
    let (owner, name) = test_model_parts();
    let repo = model(&api, owner, name);

    let handler = Arc::new(RecordingHandler::new());

    let dir = tempfile::tempdir().unwrap();
    let params = RepoDownloadFileParams::builder()
        .filename("config.json")
        .local_dir(dir.path().to_path_buf())
        .progress(Some(handler.clone()))
        .build();

    let path = repo.download_file(&params).await.unwrap();
    assert!(path.exists());

    let events = handler.events();
    assert!(!events.is_empty(), "should have received progress events");

    // First event should be Download(Start)
    assert!(
        matches!(&events[0], ProgressEvent::Download(DownloadEvent::Start { total_files: 1, .. })),
        "first event should be Download(Start), got {:?}",
        &events[0]
    );

    // Last event should be Download(Complete)
    assert!(
        matches!(events.last().unwrap(), ProgressEvent::Download(DownloadEvent::Complete)),
        "last event should be Download(Complete)"
    );

    // Should have at least one Progress event with InProgress or Complete
    let has_progress = events
        .iter()
        .any(|e| matches!(e, ProgressEvent::Download(DownloadEvent::Progress { .. })));
    assert!(has_progress, "should have at least one Progress event");

    // Should have a Complete file status
    let has_file_complete = events.iter().any(|e| {
        if let ProgressEvent::Download(DownloadEvent::Progress { files }) = e {
            files.iter().any(|f| f.status == FileStatus::Complete)
        } else {
            false
        }
    });
    assert!(has_file_complete, "should have a file Complete status event");
}

#[tokio::test]
async fn test_download_file_with_progress_to_cache() {
    let Some(api) = api() else { return };
    let (owner, name) = test_model_parts();
    let repo = model(&api, owner, name);

    let handler = Arc::new(RecordingHandler::new());

    let params = RepoDownloadFileParams::builder()
        .filename("config.json")
        .force_download(true)
        .progress(Some(handler.clone()))
        .build();

    let path = repo.download_file(&params).await.unwrap();
    assert!(path.exists());

    let events = handler.events();
    assert!(!events.is_empty(), "should have received progress events");

    assert!(
        matches!(&events[0], ProgressEvent::Download(DownloadEvent::Start { total_files: 1, .. })),
        "first event should be Download(Start)"
    );
    assert!(
        matches!(events.last().unwrap(), ProgressEvent::Download(DownloadEvent::Complete)),
        "last event should be Download(Complete)"
    );
}

#[tokio::test]
async fn test_download_with_no_progress_handler() {
    let Some(api) = api() else { return };
    let (owner, name) = test_model_parts();
    let repo = model(&api, owner, name);

    let dir = tempfile::tempdir().unwrap();
    let params = RepoDownloadFileParams::builder()
        .filename("config.json")
        .local_dir(dir.path().to_path_buf())
        .build();

    let path = repo.download_file(&params).await.unwrap();
    assert!(path.exists());
}
