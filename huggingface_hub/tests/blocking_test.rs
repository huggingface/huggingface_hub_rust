#![cfg(feature = "blocking")]

//! Integration tests for the synchronous HfApiSync wrapper.
//!
//! These mirror a subset of the async integration tests to verify that the
//! blocking API works correctly end-to-end.
//!
//! Read-only tests: require HF_TOKEN, skip if not set.
//! Write tests: require HF_TOKEN + HF_TEST_WRITE=1, skip otherwise.
//!
//! Run: HF_TOKEN=hf_xxx cargo test -p huggingface-hub --features blocking --test blocking_test

use huggingface_hub::types::*;
use huggingface_hub::{HFClientBuilder, HFClientSync};

fn sync_api() -> Option<HFClientSync> {
    if std::env::var("HF_TOKEN").is_err() {
        return None;
    }
    let api = HFClientBuilder::new().build().expect("Failed to create HFClient");
    Some(HFClientSync::from_api(api).expect("Failed to create HFClientSync"))
}

fn write_enabled() -> bool {
    std::env::var("HF_TEST_WRITE").ok().is_some_and(|v| v == "1")
}

// --- Repo info ---

#[test]
fn test_sync_model_info() {
    let Some(api) = sync_api() else { return };
    let params = ModelInfoParams::builder().repo_id("gpt2").build();
    let info = api.model_info(&params).unwrap();
    assert_eq!(info.id, "openai-community/gpt2");
}

#[test]
fn test_sync_dataset_info() {
    let Some(api) = sync_api() else { return };
    let params = DatasetInfoParams::builder().repo_id("rajpurkar/squad").build();
    let info = api.dataset_info(&params).unwrap();
    assert!(info.id.contains("squad"));
}

#[test]
fn test_sync_repo_exists() {
    let Some(api) = sync_api() else { return };
    let params = RepoExistsParams::builder().repo_id("gpt2").build();
    assert!(api.repo_exists(&params).unwrap());

    let params = RepoExistsParams::builder()
        .repo_id("this-repo-definitely-does-not-exist-12345")
        .build();
    assert!(!api.repo_exists(&params).unwrap());
}

#[test]
fn test_sync_file_exists() {
    let Some(api) = sync_api() else { return };
    let params = FileExistsParams::builder().repo_id("gpt2").filename("config.json").build();
    assert!(api.file_exists(&params).unwrap());

    let params = FileExistsParams::builder()
        .repo_id("gpt2")
        .filename("nonexistent_file.xyz")
        .build();
    assert!(!api.file_exists(&params).unwrap());
}

// --- Listing (stream methods collected to Vec) ---

#[test]
fn test_sync_list_models() {
    let Some(api) = sync_api() else { return };
    let params = ListModelsParams::builder().author("openai-community").limit(3_usize).build();
    let models = api.list_models(&params).unwrap();
    assert!(!models.is_empty());
    assert!(models[0].id.starts_with("openai-community/"));
}

#[test]
fn test_sync_list_datasets() {
    let Some(api) = sync_api() else { return };
    let params = ListDatasetsParams::builder().author("huggingface").limit(3_usize).build();
    let datasets = api.list_datasets(&params).unwrap();
    assert!(!datasets.is_empty());
}

#[test]
fn test_sync_list_repo_files() {
    let Some(api) = sync_api() else { return };
    let params = ListRepoFilesParams::builder().repo_id("gpt2").build();
    let files = api.list_repo_files(&params).unwrap();
    assert!(files.contains(&"config.json".to_string()));
    assert!(files.contains(&"README.md".to_string()));
}

#[test]
fn test_sync_list_repo_tree() {
    let Some(api) = sync_api() else { return };
    let params = ListRepoTreeParams::builder().repo_id("gpt2").build();
    let entries = api.list_repo_tree(&params).unwrap();
    let has_config = entries
        .iter()
        .any(|e| matches!(e, RepoTreeEntry::File { path, .. } if path == "config.json"));
    assert!(has_config);
}

#[test]
fn test_sync_list_repo_commits() {
    let Some(api) = sync_api() else { return };
    let params = ListRepoCommitsParams::builder().repo_id("gpt2").build();
    let commits = api.list_repo_commits(&params).unwrap();
    assert!(!commits.is_empty());
    assert!(!commits[0].id.is_empty());
    assert!(!commits[0].title.is_empty());
}

// --- Refs ---

#[test]
fn test_sync_list_repo_refs() {
    let Some(api) = sync_api() else { return };
    let params = ListRepoRefsParams::builder().repo_id("gpt2").build();
    let refs = api.list_repo_refs(&params).unwrap();
    assert!(!refs.branches.is_empty());
    assert!(refs.branches.iter().any(|b| b.name == "main"));
}

#[test]
fn test_sync_revision_exists() {
    let Some(api) = sync_api() else { return };
    let params = RevisionExistsParams::builder().repo_id("gpt2").revision("main").build();
    assert!(api.revision_exists(&params).unwrap());

    let params = RevisionExistsParams::builder()
        .repo_id("gpt2")
        .revision("nonexistent-branch-xyz")
        .build();
    assert!(!api.revision_exists(&params).unwrap());
}

// --- Download ---

#[test]
fn test_sync_download_file() {
    let Some(api) = sync_api() else { return };
    let dir = tempfile::tempdir().unwrap();
    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("config.json")
        .local_dir(dir.path().to_path_buf())
        .build();
    let path = api.download_file(&params).unwrap();
    assert!(path.exists());
    let content = std::fs::read_to_string(&path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(json.get("model_type").is_some());
}

// --- Users ---

#[test]
fn test_sync_whoami() {
    let Some(api) = sync_api() else { return };
    let user = api.whoami().unwrap();
    assert!(!user.username.is_empty());
}

#[test]
fn test_sync_auth_check() {
    let Some(api) = sync_api() else { return };
    api.auth_check().unwrap();
}

#[test]
fn test_sync_get_user_overview() {
    let Some(api) = sync_api() else { return };
    let user = api.get_user_overview("julien-c").unwrap();
    assert_eq!(user.username, "julien-c");
}

#[test]
fn test_sync_get_organization_overview() {
    let Some(api) = sync_api() else { return };
    let org = api.get_organization_overview("huggingface").unwrap();
    assert_eq!(org.name, "huggingface");
}

#[test]
fn test_sync_list_user_followers() {
    let Some(api) = sync_api() else { return };
    let followers = api.list_user_followers("julien-c").unwrap();
    assert!(!followers.is_empty());
}

#[test]
fn test_sync_list_user_following() {
    let Some(api) = sync_api() else { return };
    let following = api.list_user_following("julien-c").unwrap();
    assert!(!following.is_empty());
}

#[test]
fn test_sync_list_organization_members() {
    let Some(api) = sync_api() else { return };
    let members = api.list_organization_members("huggingface").unwrap();
    assert!(!members.is_empty());
}

// --- Diffs ---

#[test]
fn test_sync_get_commit_diff() {
    let Some(api) = sync_api() else { return };
    let commits_params = ListRepoCommitsParams::builder().repo_id("openai-community/gpt2").build();
    let commits = api.list_repo_commits(&commits_params).unwrap();
    assert!(commits.len() >= 2);

    let params = GetCommitDiffParams::builder()
        .repo_id("openai-community/gpt2")
        .compare(format!("{}..{}", commits[1].id, commits[0].id))
        .build();
    let diff = api.get_commit_diff(&params).unwrap();
    assert!(!diff.is_empty());
}

// --- Write operations ---

fn uuid_v4_short() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    format!("{:x}{:x}", t.as_secs(), t.subsec_nanos())
}

fn create_test_repo(api: &HfApiSync) -> String {
    let whoami = api.whoami().expect("whoami failed");
    let repo_id = format!("{}/huggingface-hub-rust-sync-test-{}", whoami.username, uuid_v4_short());
    let params = CreateRepoParams::builder()
        .repo_id(&repo_id)
        .private(true)
        .exist_ok(false)
        .build();
    api.create_repo(&params).expect("create_repo failed");

    let params = UploadFileParams::builder()
        .repo_id(&repo_id)
        .source(AddSource::Bytes(b"initial content".to_vec()))
        .path_in_repo("README.md")
        .commit_message("initial commit")
        .build();
    api.upload_file(&params).expect("seed upload failed");

    repo_id
}

fn delete_test_repo(api: &HfApiSync, repo_id: &str) {
    let params = DeleteRepoParams::builder().repo_id(repo_id).build();
    let _ = api.delete_repo(&params);
}

#[test]
fn test_sync_create_and_delete_repo() {
    let Some(api) = sync_api() else { return };
    if !write_enabled() {
        return;
    }

    let whoami = api.whoami().expect("whoami failed");
    let repo_id = format!("{}/huggingface-hub-rust-sync-test-{}", whoami.username, uuid_v4_short());

    let params = CreateRepoParams::builder()
        .repo_id(&repo_id)
        .private(true)
        .exist_ok(true)
        .build();
    let url = api.create_repo(&params).unwrap();
    assert!(url.url.contains(&repo_id));

    let params = UploadFileParams::builder()
        .repo_id(&repo_id)
        .source(AddSource::Bytes(b"hello world".to_vec()))
        .path_in_repo("test.txt")
        .commit_message("test upload")
        .build();
    let commit = api.upload_file(&params).unwrap();
    assert!(commit.commit_oid.is_some());

    let params = FileExistsParams::builder().repo_id(&repo_id).filename("test.txt").build();
    assert!(api.file_exists(&params).unwrap());

    let params = DeleteRepoParams::builder().repo_id(&repo_id).build();
    api.delete_repo(&params).unwrap();
}

#[test]
fn test_sync_create_commit() {
    let Some(api) = sync_api() else { return };
    if !write_enabled() {
        return;
    }
    let repo_id = create_test_repo(&api);

    let params = CreateCommitParams::builder()
        .repo_id(&repo_id)
        .operations(vec![
            CommitOperation::Add {
                path_in_repo: "file_a.txt".to_string(),
                source: AddSource::Bytes(b"content a".to_vec()),
            },
            CommitOperation::Add {
                path_in_repo: "file_b.txt".to_string(),
                source: AddSource::Bytes(b"content b".to_vec()),
            },
        ])
        .commit_message("add two files")
        .build();
    let commit = api.create_commit(&params).unwrap();
    assert!(commit.commit_oid.is_some());

    let files_params = ListRepoFilesParams::builder().repo_id(&repo_id).build();
    let files = api.list_repo_files(&files_params).unwrap();
    assert!(files.contains(&"file_a.txt".to_string()));
    assert!(files.contains(&"file_b.txt".to_string()));

    delete_test_repo(&api, &repo_id);
}

#[test]
fn test_sync_upload_folder() {
    let Some(api) = sync_api() else { return };
    if !write_enabled() {
        return;
    }
    let repo_id = create_test_repo(&api);

    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("hello.txt"), "hello").unwrap();
    std::fs::create_dir_all(dir.path().join("subdir")).unwrap();
    std::fs::write(dir.path().join("subdir/nested.txt"), "nested").unwrap();

    let params = UploadFolderParams::builder()
        .repo_id(&repo_id)
        .folder_path(dir.path().to_path_buf())
        .commit_message("upload folder")
        .build();
    let commit = api.upload_folder(&params).unwrap();
    assert!(commit.commit_oid.is_some());

    let files_params = ListRepoFilesParams::builder().repo_id(&repo_id).build();
    let files = api.list_repo_files(&files_params).unwrap();
    assert!(files.contains(&"hello.txt".to_string()));
    assert!(files.contains(&"subdir/nested.txt".to_string()));

    delete_test_repo(&api, &repo_id);
}

#[test]
fn test_sync_branch_operations() {
    let Some(api) = sync_api() else { return };
    if !write_enabled() {
        return;
    }
    let repo_id = create_test_repo(&api);

    let create_params = CreateBranchParams::builder().repo_id(&repo_id).branch("test-branch").build();
    api.create_branch(&create_params).unwrap();

    let refs_params = ListRepoRefsParams::builder().repo_id(&repo_id).build();
    let refs = api.list_repo_refs(&refs_params).unwrap();
    assert!(refs.branches.iter().any(|b| b.name == "test-branch"));

    let delete_params = DeleteBranchParams::builder().repo_id(&repo_id).branch("test-branch").build();
    api.delete_branch(&delete_params).unwrap();

    let refs = api.list_repo_refs(&refs_params).unwrap();
    assert!(!refs.branches.iter().any(|b| b.name == "test-branch"));

    delete_test_repo(&api, &repo_id);
}
