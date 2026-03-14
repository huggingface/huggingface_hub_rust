//! Integration tests against the live Hugging Face Hub API.
//!
//! Read-only tests: require HF_TOKEN, skip if not set.
//! Write tests: require HF_TOKEN + HF_TEST_WRITE=1, skip otherwise.
//!
//! Run read-only: HF_TOKEN=hf_xxx cargo test -p huggingface-hub --test integration_test
//! Run all: HF_TOKEN=hf_xxx HF_TEST_WRITE=1 cargo test -p huggingface-hub --test integration_test

use futures::StreamExt;
use huggingface_hub::types::*;
use huggingface_hub::{HfApi, HfApiBuilder};

fn api() -> Option<HfApi> {
    if std::env::var("HF_TOKEN").is_err() {
        return None;
    }
    Some(HfApiBuilder::new().build().expect("Failed to create HfApi"))
}

fn write_enabled() -> bool {
    std::env::var("HF_TEST_WRITE")
        .ok()
        .map_or(false, |v| v == "1")
}

#[tokio::test]
async fn test_model_info() {
    let Some(api) = api() else { return };
    let params = ModelInfoParams::builder().repo_id("gpt2").build();
    let info = api.model_info(&params).await.unwrap();
    assert_eq!(info.id, "openai-community/gpt2");
}

#[tokio::test]
async fn test_dataset_info() {
    let Some(api) = api() else { return };
    let params = DatasetInfoParams::builder()
        .repo_id("rajpurkar/squad")
        .build();
    let info = api.dataset_info(&params).await.unwrap();
    assert!(info.id.contains("squad"));
}

#[tokio::test]
async fn test_repo_exists() {
    let Some(api) = api() else { return };
    let params = RepoExistsParams::builder().repo_id("gpt2").build();
    assert!(api.repo_exists(&params).await.unwrap());

    let params = RepoExistsParams::builder()
        .repo_id("this-repo-definitely-does-not-exist-12345")
        .build();
    assert!(!api.repo_exists(&params).await.unwrap());
}

#[tokio::test]
async fn test_file_exists() {
    let Some(api) = api() else { return };
    let params = FileExistsParams::builder()
        .repo_id("gpt2")
        .filename("config.json")
        .build();
    assert!(api.file_exists(&params).await.unwrap());

    let params = FileExistsParams::builder()
        .repo_id("gpt2")
        .filename("nonexistent_file.xyz")
        .build();
    assert!(!api.file_exists(&params).await.unwrap());
}

#[tokio::test]
async fn test_list_models() {
    let Some(api) = api() else { return };
    let params = ListModelsParams::builder()
        .author("openai-community")
        .limit(3_usize)
        .build();
    let stream = api.list_models(&params);
    futures::pin_mut!(stream);

    let mut count = 0;
    while let Some(model) = stream.next().await {
        let model = model.unwrap();
        assert!(model.id.starts_with("openai-community/"));
        count += 1;
        if count >= 3 {
            break;
        }
    }
    assert!(count > 0);
}

#[tokio::test]
async fn test_list_repo_files() {
    let Some(api) = api() else { return };
    let params = ListRepoFilesParams::builder().repo_id("gpt2").build();
    let files = api.list_repo_files(&params).await.unwrap();
    assert!(files.contains(&"config.json".to_string()));
    assert!(files.contains(&"README.md".to_string()));
}

#[tokio::test]
async fn test_list_repo_tree() {
    let Some(api) = api() else { return };
    let params = ListRepoTreeParams::builder().repo_id("gpt2").build();
    let stream = api.list_repo_tree(&params);
    futures::pin_mut!(stream);

    let mut found_config = false;
    while let Some(entry) = stream.next().await {
        let entry = entry.unwrap();
        if let RepoTreeEntry::File { path, .. } = &entry {
            if path == "config.json" {
                found_config = true;
                break;
            }
        }
    }
    assert!(found_config);
}

#[tokio::test]
async fn test_list_repo_commits() {
    let Some(api) = api() else { return };
    let params = ListRepoCommitsParams::builder().repo_id("gpt2").build();
    let stream = api.list_repo_commits(&params);
    futures::pin_mut!(stream);

    let first = stream.next().await.unwrap().unwrap();
    assert!(!first.id.is_empty());
    assert!(!first.title.is_empty());
}

#[tokio::test]
async fn test_list_repo_refs() {
    let Some(api) = api() else { return };
    let params = ListRepoRefsParams::builder().repo_id("gpt2").build();
    let refs = api.list_repo_refs(&params).await.unwrap();
    assert!(!refs.branches.is_empty());
    // "main" branch should exist
    assert!(refs.branches.iter().any(|b| b.name == "main"));
}

#[tokio::test]
async fn test_revision_exists() {
    let Some(api) = api() else { return };
    let params = RevisionExistsParams::builder()
        .repo_id("gpt2")
        .revision("main")
        .build();
    assert!(api.revision_exists(&params).await.unwrap());

    let params = RevisionExistsParams::builder()
        .repo_id("gpt2")
        .revision("nonexistent-branch-xyz")
        .build();
    assert!(!api.revision_exists(&params).await.unwrap());
}

#[tokio::test]
async fn test_download_file() {
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
    assert!(json.get("model_type").is_some());
}

// --- Write operation tests (require HF_TEST_WRITE=1) ---

#[tokio::test]
async fn test_create_and_delete_repo() {
    let Some(api) = api() else { return };
    if !write_enabled() {
        return;
    }

    let repo_id = format!(
        "{}/huggingface-hub-rust-test-{}",
        "assafvayner",
        uuid_v4_short()
    );

    // Create
    let params = CreateRepoParams::builder()
        .repo_id(&repo_id)
        .private(true)
        .exist_ok(true)
        .build();
    let url = api.create_repo(&params).await.unwrap();
    assert!(url.url.contains(&repo_id));

    // Upload a file
    let params = UploadFileParams::builder()
        .repo_id(&repo_id)
        .source(AddSource::Bytes(b"hello world".to_vec()))
        .path_in_repo("test.txt")
        .commit_message("test upload")
        .build();
    let commit = api.upload_file(&params).await.unwrap();
    assert!(commit.oid.is_some());

    // Verify file exists
    let params = FileExistsParams::builder()
        .repo_id(&repo_id)
        .filename("test.txt")
        .build();
    assert!(api.file_exists(&params).await.unwrap());

    // Delete repo
    let params = DeleteRepoParams::builder().repo_id(&repo_id).build();
    api.delete_repo(&params).await.unwrap();
}

fn uuid_v4_short() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    format!("{:x}{:x}", t.as_secs(), t.subsec_nanos())
}
