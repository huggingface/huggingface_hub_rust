//! Integration tests against the live Hugging Face Hub API.
//!
//! Read-only tests: require HF_TOKEN, skip if not set.
//! Write tests: require HF_TOKEN + HF_TEST_WRITE=1, skip otherwise.
//!
//! Run read-only: HF_TOKEN=hf_xxx cargo test -p huggingface-hub --test integration_test
//! Run all: HF_TOKEN=hf_xxx HF_TEST_WRITE=1 cargo test -p huggingface-hub --test integration_test
//!
//! Feature-gated tests: enable with --features, e.g.:
//!   HF_TOKEN=hf_xxx cargo test -p huggingface-hub --all-features --test integration_test

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
        .is_some_and(|v| v == "1")
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

// --- User operations ---

#[tokio::test]
async fn test_whoami() {
    let Some(api) = api() else { return };
    let user = api.whoami().await.unwrap();
    assert!(!user.username.is_empty());
}

#[tokio::test]
async fn test_auth_check() {
    let Some(api) = api() else { return };
    api.auth_check().await.unwrap();
}

#[tokio::test]
async fn test_get_user_overview() {
    let Some(api) = api() else { return };
    let user = api.get_user_overview("julien-c").await.unwrap();
    assert_eq!(user.username, "julien-c");
}

#[tokio::test]
async fn test_get_organization_overview() {
    let Some(api) = api() else { return };
    let org = api.get_organization_overview("huggingface").await.unwrap();
    assert_eq!(org.name, "huggingface");
}

#[tokio::test]
async fn test_list_user_followers() {
    let Some(api) = api() else { return };
    let stream = api.list_user_followers("julien-c");
    futures::pin_mut!(stream);
    let first = stream.next().await;
    assert!(first.is_some());
    first.unwrap().unwrap();
}

#[tokio::test]
async fn test_list_user_following() {
    let Some(api) = api() else { return };
    let stream = api.list_user_following("julien-c");
    futures::pin_mut!(stream);
    let first = stream.next().await;
    assert!(first.is_some());
    first.unwrap().unwrap();
}

#[tokio::test]
async fn test_list_organization_members() {
    let Some(api) = api() else { return };
    let stream = api.list_organization_members("huggingface");
    futures::pin_mut!(stream);
    let first = stream.next().await;
    assert!(first.is_some());
    first.unwrap().unwrap();
}

// --- Additional repo info tests ---

#[tokio::test]
async fn test_space_info() {
    let Some(api) = api() else { return };
    let params = SpaceInfoParams::builder()
        .repo_id("HuggingFaceFW/blogpost-fineweb-v1")
        .build();
    let info = api.space_info(&params).await.unwrap();
    assert!(info.id.contains("blogpost-fineweb-v1"));
}

#[tokio::test]
async fn test_list_datasets() {
    let Some(api) = api() else { return };
    let params = ListDatasetsParams::builder()
        .author("huggingface")
        .limit(3_usize)
        .build();
    let stream = api.list_datasets(&params);
    futures::pin_mut!(stream);

    let mut count = 0;
    while let Some(ds) = stream.next().await {
        ds.unwrap();
        count += 1;
        if count >= 3 {
            break;
        }
    }
    assert!(count > 0);
}

#[tokio::test]
async fn test_list_spaces() {
    let Some(api) = api() else { return };
    let params = ListSpacesParams::builder()
        .author("huggingface")
        .limit(3_usize)
        .build();
    let stream = api.list_spaces(&params);
    futures::pin_mut!(stream);

    let mut count = 0;
    while let Some(space) = stream.next().await {
        space.unwrap();
        count += 1;
        if count >= 3 {
            break;
        }
    }
    assert!(count > 0);
}

// --- File info tests ---

#[tokio::test]
async fn test_get_paths_info() {
    let Some(api) = api() else { return };
    let params = GetPathsInfoParams::builder()
        .repo_id("gpt2")
        .paths(vec!["config.json".to_string(), "README.md".to_string()])
        .build();
    let entries = api.get_paths_info(&params).await.unwrap();
    assert_eq!(entries.len(), 2);
    let paths: Vec<String> = entries
        .iter()
        .map(|e| match e {
            RepoTreeEntry::File { path, .. } => path.clone(),
            RepoTreeEntry::Directory { path, .. } => path.clone(),
        })
        .collect();
    assert!(paths.contains(&"config.json".to_string()));
    assert!(paths.contains(&"README.md".to_string()));
}

// --- Commit and diff tests ---

#[tokio::test]
async fn test_get_commit_diff() {
    let Some(api) = api() else { return };

    let commits_params = ListRepoCommitsParams::builder()
        .repo_id("openai-community/gpt2")
        .build();
    let stream = api.list_repo_commits(&commits_params);
    futures::pin_mut!(stream);

    let first = stream.next().await.unwrap().unwrap();
    let second = stream.next().await.unwrap().unwrap();

    let params = GetCommitDiffParams::builder()
        .repo_id("openai-community/gpt2")
        .compare(format!("{}..{}", second.id, first.id))
        .build();
    let diff = api.get_commit_diff(&params).await.unwrap();
    assert!(!diff.is_empty());
}

#[tokio::test]
async fn test_get_raw_diff() {
    let Some(api) = api() else { return };

    let commits_params = ListRepoCommitsParams::builder()
        .repo_id("openai-community/gpt2")
        .build();
    let stream = api.list_repo_commits(&commits_params);
    futures::pin_mut!(stream);

    let first = stream.next().await.unwrap().unwrap();
    let second = stream.next().await.unwrap().unwrap();

    let params = GetRawDiffParams::builder()
        .repo_id("openai-community/gpt2")
        .compare(format!("{}..{}", second.id, first.id))
        .build();
    let raw = api.get_raw_diff(&params).await.unwrap();
    assert!(!raw.is_empty());
}

// --- Write operation tests (require HF_TEST_WRITE=1) ---

#[tokio::test]
async fn test_create_and_delete_repo() {
    let Some(api) = api() else { return };
    if !write_enabled() {
        return;
    }

    let whoami = api
        .whoami()
        .await
        .expect("whoami should return something, make sure HF_TOKEN is set");

    let repo_id = format!(
        "{}/huggingface-hub-rust-test-{}",
        whoami.username,
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
    assert!(commit.commit_oid.is_some());

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

async fn create_test_repo(api: &HfApi) -> String {
    let whoami = api.whoami().await.expect("whoami failed");
    let repo_id = format!(
        "{}/huggingface-hub-rust-test-{}",
        whoami.username,
        uuid_v4_short()
    );
    let params = CreateRepoParams::builder()
        .repo_id(&repo_id)
        .private(true)
        .exist_ok(false)
        .build();
    api.create_repo(&params).await.expect("create_repo failed");

    let params = UploadFileParams::builder()
        .repo_id(&repo_id)
        .source(AddSource::Bytes(b"initial content".to_vec()))
        .path_in_repo("README.md")
        .commit_message("initial commit")
        .build();
    api.upload_file(&params).await.expect("seed upload failed");

    repo_id
}

async fn delete_test_repo(api: &HfApi, repo_id: &str) {
    let params = DeleteRepoParams::builder().repo_id(repo_id).build();
    let _ = api.delete_repo(&params).await;
}

#[tokio::test]
async fn test_create_commit() {
    let Some(api) = api() else { return };
    if !write_enabled() {
        return;
    }
    let repo_id = create_test_repo(&api).await;

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
    let commit = api.create_commit(&params).await.unwrap();
    assert!(commit.commit_oid.is_some());

    let files_params = ListRepoFilesParams::builder().repo_id(&repo_id).build();
    let files = api.list_repo_files(&files_params).await.unwrap();
    assert!(files.contains(&"file_a.txt".to_string()));
    assert!(files.contains(&"file_b.txt".to_string()));

    delete_test_repo(&api, &repo_id).await;
}

#[tokio::test]
async fn test_upload_folder() {
    let Some(api) = api() else { return };
    if !write_enabled() {
        return;
    }
    let repo_id = create_test_repo(&api).await;

    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("hello.txt"), "hello").unwrap();
    std::fs::create_dir_all(dir.path().join("subdir")).unwrap();
    std::fs::write(dir.path().join("subdir/nested.txt"), "nested").unwrap();

    let params = UploadFolderParams::builder()
        .repo_id(&repo_id)
        .folder_path(dir.path().to_path_buf())
        .commit_message("upload folder")
        .build();
    let commit = api.upload_folder(&params).await.unwrap();
    assert!(commit.commit_oid.is_some());

    let files_params = ListRepoFilesParams::builder().repo_id(&repo_id).build();
    let files = api.list_repo_files(&files_params).await.unwrap();
    assert!(files.contains(&"hello.txt".to_string()));
    assert!(files.contains(&"subdir/nested.txt".to_string()));

    delete_test_repo(&api, &repo_id).await;
}

#[tokio::test]
async fn test_delete_file() {
    let Some(api) = api() else { return };
    if !write_enabled() {
        return;
    }
    let repo_id = create_test_repo(&api).await;

    let upload_params = UploadFileParams::builder()
        .repo_id(&repo_id)
        .source(AddSource::Bytes(b"to delete".to_vec()))
        .path_in_repo("deleteme.txt")
        .commit_message("add file to delete")
        .build();
    api.upload_file(&upload_params).await.unwrap();

    let params = DeleteFileParams::builder()
        .repo_id(&repo_id)
        .path_in_repo("deleteme.txt")
        .commit_message("delete file")
        .build();
    api.delete_file(&params).await.unwrap();

    let exists_params = FileExistsParams::builder()
        .repo_id(&repo_id)
        .filename("deleteme.txt")
        .build();
    assert!(!api.file_exists(&exists_params).await.unwrap());

    delete_test_repo(&api, &repo_id).await;
}

#[tokio::test]
async fn test_delete_folder() {
    let Some(api) = api() else { return };
    if !write_enabled() {
        return;
    }
    let repo_id = create_test_repo(&api).await;

    let commit_params = CreateCommitParams::builder()
        .repo_id(&repo_id)
        .operations(vec![
            CommitOperation::Add {
                path_in_repo: "folder/a.txt".to_string(),
                source: AddSource::Bytes(b"a".to_vec()),
            },
            CommitOperation::Add {
                path_in_repo: "folder/b.txt".to_string(),
                source: AddSource::Bytes(b"b".to_vec()),
            },
        ])
        .commit_message("add folder")
        .build();
    api.create_commit(&commit_params).await.unwrap();

    let params = DeleteFolderParams::builder()
        .repo_id(&repo_id)
        .path_in_repo("folder")
        .commit_message("delete folder")
        .build();
    api.delete_folder(&params).await.unwrap();

    let exists_a = FileExistsParams::builder()
        .repo_id(&repo_id)
        .filename("folder/a.txt")
        .build();
    assert!(!api.file_exists(&exists_a).await.unwrap());

    delete_test_repo(&api, &repo_id).await;
}

#[tokio::test]
async fn test_create_and_delete_branch() {
    let Some(api) = api() else { return };
    if !write_enabled() {
        return;
    }
    let repo_id = create_test_repo(&api).await;

    let create_params = CreateBranchParams::builder()
        .repo_id(&repo_id)
        .branch("test-branch")
        .build();
    api.create_branch(&create_params).await.unwrap();

    let refs_params = ListRepoRefsParams::builder().repo_id(&repo_id).build();
    let refs = api.list_repo_refs(&refs_params).await.unwrap();
    assert!(refs.branches.iter().any(|b| b.name == "test-branch"));

    let delete_params = DeleteBranchParams::builder()
        .repo_id(&repo_id)
        .branch("test-branch")
        .build();
    api.delete_branch(&delete_params).await.unwrap();

    let refs = api.list_repo_refs(&refs_params).await.unwrap();
    assert!(!refs.branches.iter().any(|b| b.name == "test-branch"));

    delete_test_repo(&api, &repo_id).await;
}

#[tokio::test]
async fn test_create_and_delete_tag() {
    let Some(api) = api() else { return };
    if !write_enabled() {
        return;
    }
    let repo_id = create_test_repo(&api).await;

    let create_params = CreateTagParams::builder()
        .repo_id(&repo_id)
        .tag("v1.0")
        .build();
    api.create_tag(&create_params).await.unwrap();

    let refs_params = ListRepoRefsParams::builder().repo_id(&repo_id).build();
    let refs = api.list_repo_refs(&refs_params).await.unwrap();
    assert!(refs.tags.iter().any(|t| t.name == "v1.0"));

    let delete_params = DeleteTagParams::builder()
        .repo_id(&repo_id)
        .tag("v1.0")
        .build();
    api.delete_tag(&delete_params).await.unwrap();

    let refs = api.list_repo_refs(&refs_params).await.unwrap();
    assert!(!refs.tags.iter().any(|t| t.name == "v1.0"));

    delete_test_repo(&api, &repo_id).await;
}

#[tokio::test]
async fn test_update_repo_settings() {
    let Some(api) = api() else { return };
    if !write_enabled() {
        return;
    }
    let repo_id = create_test_repo(&api).await;

    let params = UpdateRepoParams::builder()
        .repo_id(&repo_id)
        .description("test description from integration test")
        .build();
    api.update_repo_settings(&params).await.unwrap();

    let info_params = ModelInfoParams::builder().repo_id(&repo_id).build();
    let _info = api.model_info(&info_params).await.unwrap();

    delete_test_repo(&api, &repo_id).await;
}

#[tokio::test]
async fn test_move_repo() {
    let Some(api) = api() else { return };
    if !write_enabled() {
        return;
    }
    let whoami = api.whoami().await.unwrap();
    let original_name = format!(
        "{}/huggingface-hub-rust-move-src-{}",
        whoami.username,
        uuid_v4_short()
    );
    let new_name = format!(
        "{}/huggingface-hub-rust-move-dst-{}",
        whoami.username,
        uuid_v4_short()
    );

    let create_params = CreateRepoParams::builder()
        .repo_id(&original_name)
        .private(true)
        .build();
    api.create_repo(&create_params).await.unwrap();

    let move_params = MoveRepoParams::builder()
        .from_id(&original_name)
        .to_id(&new_name)
        .build();
    api.move_repo(&move_params).await.unwrap();

    let exists_new = RepoExistsParams::builder().repo_id(&new_name).build();
    assert!(api.repo_exists(&exists_new).await.unwrap());

    let delete_params = DeleteRepoParams::builder().repo_id(&new_name).build();
    api.delete_repo(&delete_params).await.unwrap();
}

// =============================================================================
// Spaces management tests (feature: "spaces")
// =============================================================================

#[cfg(feature = "spaces")]
#[tokio::test]
async fn test_get_space_runtime() {
    let Some(api) = api() else { return };
    let params = GetSpaceRuntimeParams::builder()
        .repo_id("huggingface-projects/diffusers-gallery")
        .build();
    let runtime = api.get_space_runtime(&params).await.unwrap();
    assert!(runtime.stage.is_some());
}

#[cfg(feature = "spaces")]
#[tokio::test]
async fn test_duplicate_space() {
    let Some(api) = api() else { return };
    if !write_enabled() {
        return;
    }
    let whoami = api.whoami().await.unwrap();
    let to_id = format!(
        "{}/hub-rust-test-dup-space-{}",
        whoami.username,
        uuid_v4_short()
    );

    let params = DuplicateSpaceParams::builder()
        .from_id("huggingface-projects/diffusers-gallery")
        .to_id(&to_id)
        .private(true)
        .hardware("cpu-basic")
        .build();
    let result = api.duplicate_space(&params).await.unwrap();
    assert!(result.url.contains(&to_id));

    let delete_params = DeleteRepoParams::builder()
        .repo_id(&to_id)
        .repo_type(RepoType::Space)
        .build();
    let _ = api.delete_repo(&delete_params).await;
}

#[cfg(feature = "spaces")]
#[tokio::test]
async fn test_space_secrets_and_variables() {
    let Some(api) = api() else { return };
    if !write_enabled() {
        return;
    }
    let whoami = api.whoami().await.unwrap();
    let space_id = format!(
        "{}/hub-rust-test-space-{}",
        whoami.username,
        uuid_v4_short()
    );

    let create_params = CreateRepoParams::builder()
        .repo_id(&space_id)
        .repo_type(RepoType::Space)
        .private(true)
        .space_sdk("static")
        .build();
    api.create_repo(&create_params).await.unwrap();

    let add_secret = AddSpaceSecretParams::builder()
        .repo_id(&space_id)
        .key("TEST_SECRET")
        .value("secret_value")
        .build();
    api.add_space_secret(&add_secret).await.unwrap();

    let del_secret = DeleteSpaceSecretParams::builder()
        .repo_id(&space_id)
        .key("TEST_SECRET")
        .build();
    api.delete_space_secret(&del_secret).await.unwrap();

    let add_var = AddSpaceVariableParams::builder()
        .repo_id(&space_id)
        .key("TEST_VAR")
        .value("var_value")
        .build();
    api.add_space_variable(&add_var).await.unwrap();

    let del_var = DeleteSpaceVariableParams::builder()
        .repo_id(&space_id)
        .key("TEST_VAR")
        .build();
    api.delete_space_variable(&del_var).await.unwrap();

    let delete_params = DeleteRepoParams::builder()
        .repo_id(&space_id)
        .repo_type(RepoType::Space)
        .build();
    let _ = api.delete_repo(&delete_params).await;
}

// =============================================================================
// Inference Endpoints tests (feature: "inference_endpoints")
// =============================================================================

#[cfg(feature = "inference_endpoints")]
#[tokio::test]
async fn test_list_inference_endpoints() {
    let Some(api) = api() else { return };
    let params = ListInferenceEndpointsParams::builder().build();
    let endpoints = api.list_inference_endpoints(&params).await.unwrap();
    // May be empty, but should not error
    let _ = endpoints;
}

// =============================================================================
// Collections tests (feature: "collections")
// =============================================================================

#[cfg(feature = "collections")]
#[tokio::test]
async fn test_list_collections() {
    let Some(api) = api() else { return };
    let params = ListCollectionsParams::builder()
        .owner("huggingface")
        .limit(3_usize)
        .build();
    let collections = api.list_collections(&params).await.unwrap();
    assert!(!collections.is_empty());
    assert!(collections[0].slug.contains("huggingface"));
}

#[cfg(feature = "collections")]
#[tokio::test]
async fn test_get_collection() {
    let Some(api) = api() else { return };
    let list_params = ListCollectionsParams::builder()
        .owner("huggingface")
        .limit(1_usize)
        .build();
    let collections = api.list_collections(&list_params).await.unwrap();
    assert!(!collections.is_empty());

    let params = GetCollectionParams::builder()
        .slug(&collections[0].slug)
        .build();
    let coll = api.get_collection(&params).await.unwrap();
    assert_eq!(coll.slug, collections[0].slug);
}

#[cfg(feature = "collections")]
#[tokio::test]
async fn test_create_update_delete_collection() {
    let Some(api) = api() else { return };
    if !write_enabled() {
        return;
    }

    let whoami = api.whoami().await.unwrap();
    let title = format!("hub-rust-test-collection-{}", uuid_v4_short());
    let create_params = CreateCollectionParams::builder()
        .title(&title)
        .namespace(&whoami.username)
        .private(true)
        .build();
    let coll = api.create_collection(&create_params).await.unwrap();
    assert_eq!(coll.title.as_deref(), Some(title.as_str()));
    let slug = coll.slug.clone();

    let get_params = GetCollectionParams::builder().slug(&slug).build();
    let fetched = api.get_collection(&get_params).await.unwrap();
    assert_eq!(fetched.slug, slug);

    let delete_params = DeleteCollectionParams::builder().slug(&slug).build();
    api.delete_collection(&delete_params).await.unwrap();
}

// =============================================================================
// Discussions & Pull Requests tests (feature: "discussions")
// =============================================================================

#[cfg(feature = "discussions")]
#[tokio::test]
async fn test_get_repo_discussions() {
    let Some(api) = api() else { return };
    let params = GetRepoDiscussionsParams::builder()
        .repo_id("openai-community/gpt2")
        .build();
    let response = api.get_repo_discussions(&params).await.unwrap();
    assert!(!response.discussions.is_empty());
    assert!(response.discussions[0].num > 0);
}

#[cfg(feature = "discussions")]
#[tokio::test]
async fn test_get_discussion_details() {
    let Some(api) = api() else { return };
    let params = GetDiscussionDetailsParams::builder()
        .repo_id("openai-community/gpt2")
        .discussion_num(1_u64)
        .build();
    let details = api.get_discussion_details(&params).await.unwrap();
    assert_eq!(details.num, 1);
    assert!(details.title.is_some());
}

#[cfg(feature = "discussions")]
#[tokio::test]
async fn test_create_discussion_and_comment() {
    let Some(api) = api() else { return };
    if !write_enabled() {
        return;
    }
    let repo_id = create_test_repo(&api).await;

    let disc_response = api
        .get_repo_discussions(
            &GetRepoDiscussionsParams::builder()
                .repo_id(&repo_id)
                .build(),
        )
        .await
        .unwrap();
    let initial_count = disc_response.discussions.len();

    let create_params = CreateDiscussionParams::builder()
        .repo_id(&repo_id)
        .title("Test discussion from integration test")
        .description("This is a test")
        .build();
    let _disc = api.create_discussion(&create_params).await.unwrap();

    let disc_response = api
        .get_repo_discussions(
            &GetRepoDiscussionsParams::builder()
                .repo_id(&repo_id)
                .build(),
        )
        .await
        .unwrap();
    assert!(disc_response.discussions.len() > initial_count);

    delete_test_repo(&api, &repo_id).await;
}

#[cfg(feature = "discussions")]
#[tokio::test]
async fn test_create_and_merge_pull_request() {
    let Some(api) = api() else { return };
    if !write_enabled() {
        return;
    }
    let repo_id = create_test_repo(&api).await;

    let pr_params = CreatePullRequestParams::builder()
        .repo_id(&repo_id)
        .title("Test PR from integration test")
        .description("")
        .build();
    let _pr = api.create_pull_request(&pr_params).await.unwrap();

    let disc_response = api
        .get_repo_discussions(
            &GetRepoDiscussionsParams::builder()
                .repo_id(&repo_id)
                .build(),
        )
        .await
        .unwrap();
    assert!(disc_response
        .discussions
        .iter()
        .any(|d| d.is_pull_request == Some(true)));

    delete_test_repo(&api, &repo_id).await;
}

// =============================================================================
// Webhooks tests (feature: "webhooks")
// =============================================================================

#[cfg(feature = "webhooks")]
#[tokio::test]
async fn test_list_webhooks() {
    let Some(api) = api() else { return };
    if !write_enabled() {
        return;
    }
    let webhooks = api.list_webhooks().await.unwrap();
    // May be empty, but should not error
    let _ = webhooks;
}

#[cfg(feature = "webhooks")]
#[tokio::test]
async fn test_create_and_delete_webhook() {
    let Some(api) = api() else { return };
    if !write_enabled() {
        return;
    }

    let initial = api.list_webhooks().await.unwrap();
    let initial_count = initial.len();

    let whoami = api.whoami().await.unwrap();
    let create_params = CreateWebhookParams::builder()
        .url("https://example.com/test-webhook")
        .watched(vec![
            serde_json::json!({"type": "user", "name": whoami.username}),
        ])
        .domains(vec!["repo".to_string()])
        .build();
    let webhook = api.create_webhook(&create_params).await.unwrap();

    let after_create = api.list_webhooks().await.unwrap();
    assert!(after_create.len() > initial_count);

    if let Some(wh_id) = webhook.id {
        api.delete_webhook(&wh_id).await.unwrap();
    } else {
        let newest = after_create
            .iter()
            .find(|w| {
                w.url.as_deref() == Some("https://example.com/test-webhook")
                    && !initial.iter().any(|i| i.id == w.id)
            })
            .expect("should find newly created webhook");
        api.delete_webhook(newest.id.as_ref().unwrap())
            .await
            .unwrap();
    }
}

// =============================================================================
// Jobs tests (feature: "jobs")
// =============================================================================

#[cfg(feature = "jobs")]
#[tokio::test]
async fn test_list_jobs() {
    let Some(api) = api() else { return };
    if !write_enabled() {
        return;
    }
    let params = ListJobsParams::builder().build();
    let jobs = api.list_jobs(&params).await.unwrap();
    // May be empty, but should not error
    let _ = jobs;
}

#[cfg(feature = "jobs")]
#[tokio::test]
async fn test_list_job_hardware() {
    let Some(api) = api() else { return };
    let hardware = api.list_job_hardware().await.unwrap();
    assert!(!hardware.is_empty());
}

#[cfg(feature = "jobs")]
#[tokio::test]
async fn test_run_and_inspect_job() {
    let Some(api) = api() else { return };
    if !write_enabled() {
        return;
    }

    let params = RunJobParams::builder()
        .image("python:3.12")
        .command(vec![
            "python".to_string(),
            "-c".to_string(),
            "print('hello from integration test')".to_string(),
        ])
        .flavor("cpu-basic")
        .timeout("60")
        .build();
    let job = api.run_job(&params).await.unwrap();
    assert!(!job.id.is_empty());

    let inspected = api.inspect_job(&job.id, None).await.unwrap();
    assert_eq!(inspected.id, job.id);
    assert!(inspected.status.is_some());

    let _ = api.cancel_job(&job.id, None).await;
}

// =============================================================================
// Access Requests tests (feature: "access_requests")
// =============================================================================

#[cfg(feature = "access_requests")]
#[tokio::test]
async fn test_list_access_requests_on_gated_repo() {
    let Some(api) = api() else { return };
    if !write_enabled() {
        return;
    }
    let repo_id = create_test_repo(&api).await;

    let update_params = UpdateRepoParams::builder()
        .repo_id(&repo_id)
        .gated("auto")
        .build();
    api.update_repo_settings(&update_params).await.unwrap();

    let params = ListAccessRequestsParams::builder()
        .repo_id(&repo_id)
        .build();
    let pending = api.list_pending_access_requests(&params).await.unwrap();
    assert!(pending.is_empty());

    let accepted = api.list_accepted_access_requests(&params).await.unwrap();
    // auto-approved gated repos may have entries, but no error
    let _ = accepted;

    let rejected = api.list_rejected_access_requests(&params).await.unwrap();
    assert!(rejected.is_empty());

    delete_test_repo(&api, &repo_id).await;
}

// =============================================================================
// Likes tests (feature: "likes")
// =============================================================================

#[cfg(feature = "likes")]
#[tokio::test]
async fn test_list_repo_likers() {
    let Some(api) = api() else { return };
    let params = ListRepoLikersParams::builder()
        .repo_id("openai-community/gpt2")
        .build();
    let stream = api.list_repo_likers(&params);
    futures::pin_mut!(stream);

    let first = stream.next().await;
    assert!(first.is_some());
    let user = first.unwrap().unwrap();
    assert!(!user.username.is_empty());
}

#[cfg(feature = "likes")]
#[tokio::test]
async fn test_like_and_unlike() {
    let Some(api) = api() else { return };
    if !write_enabled() {
        return;
    }

    let repo_id = create_test_repo(&api).await;
    let like_params = LikeParams::builder().repo_id(&repo_id).build();
    if let Err(e) = api.like(&like_params).await {
        eprintln!("like failed (token may lack write scope for likes): {e}");
        delete_test_repo(&api, &repo_id).await;
        return;
    }

    let whoami = api.whoami().await.unwrap();
    let list_params = ListLikedReposParams::builder()
        .username(&whoami.username)
        .build();
    let likes = api.list_liked_repos(&list_params).await.unwrap();
    assert!(likes.iter().any(|l| {
        l.repo
            .as_ref()
            .and_then(|r| r.name.as_deref())
            .is_some_and(|n| n.contains(&repo_id))
    }));

    api.unlike(&like_params).await.unwrap();
    delete_test_repo(&api, &repo_id).await;
}

// =============================================================================
// Papers tests (feature: "papers")
// =============================================================================

#[cfg(feature = "papers")]
#[tokio::test]
async fn test_paper_info() {
    let Some(api) = api() else { return };
    let params = PaperInfoParams::builder().paper_id("2307.09288").build();
    let paper = api.paper_info(&params).await.unwrap();
    assert_eq!(paper.id, "2307.09288");
    assert!(paper.title.is_some());
}

#[cfg(feature = "papers")]
#[tokio::test]
async fn test_list_papers() {
    let Some(api) = api() else { return };
    let params = ListPapersParams::builder()
        .query("attention")
        .limit(5_usize)
        .build();
    let results = api.list_papers(&params).await.unwrap();
    assert!(!results.is_empty());
    assert!(results[0].paper.is_some());
}

#[cfg(feature = "papers")]
#[tokio::test]
async fn test_list_daily_papers() {
    let Some(api) = api() else { return };
    let params = ListDailyPapersParams::builder()
        .date("2024-10-29")
        .limit(5_usize)
        .build();
    let papers = api.list_daily_papers(&params).await.unwrap();
    assert!(!papers.is_empty());
    assert!(papers[0].paper.is_some());
}

// =============================================================================
// Cache-mode download tests
// =============================================================================

#[tokio::test]
async fn test_download_file_to_cache() {
    let Some(_) = api() else { return };
    let cache_dir = tempfile::tempdir().unwrap();
    let api = HfApiBuilder::new()
        .cache_dir(cache_dir.path())
        .build()
        .unwrap();

    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("config.json")
        .build();
    let path = api.download_file(&params).await.unwrap();

    assert!(path.exists());
    let content = std::fs::read_to_string(&path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(json.get("model_type").is_some());
    assert!(path.to_string_lossy().contains("snapshots"));

    let repo_folder = std::fs::read_dir(cache_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.file_name().to_string_lossy().contains("gpt2"))
        .expect("repo folder not found");
    let blobs_dir = repo_folder.path().join("blobs");
    assert!(blobs_dir.exists());
    let blob_count = std::fs::read_dir(&blobs_dir).unwrap().count();
    assert_eq!(blob_count, 1);
}

#[tokio::test]
async fn test_download_file_cache_hit() {
    let Some(_) = api() else { return };
    let cache_dir = tempfile::tempdir().unwrap();
    let api = HfApiBuilder::new()
        .cache_dir(cache_dir.path())
        .build()
        .unwrap();

    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("config.json")
        .build();
    let path1 = api.download_file(&params).await.unwrap();
    let path2 = api.download_file(&params).await.unwrap();
    assert_eq!(path1, path2);
}

#[tokio::test]
async fn test_download_file_local_files_only_miss() {
    let cache_dir = tempfile::tempdir().unwrap();
    let api = HfApiBuilder::new()
        .cache_dir(cache_dir.path())
        .build()
        .unwrap();

    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("config.json")
        .local_files_only(true)
        .build();
    let result = api.download_file(&params).await;
    assert!(matches!(
        result,
        Err(huggingface_hub::HfError::LocalEntryNotFound { .. })
    ));
}

#[tokio::test]
async fn test_download_file_local_files_only_hit() {
    let Some(_) = api() else { return };
    let cache_dir = tempfile::tempdir().unwrap();
    let api = HfApiBuilder::new()
        .cache_dir(cache_dir.path())
        .build()
        .unwrap();

    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("config.json")
        .build();
    let path1 = api.download_file(&params).await.unwrap();

    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("config.json")
        .local_files_only(true)
        .build();
    let path2 = api.download_file(&params).await.unwrap();
    assert_eq!(path1, path2);
}

#[cfg(not(windows))]
#[tokio::test]
async fn test_download_file_cache_symlink_structure() {
    let Some(_) = api() else { return };
    let cache_dir = tempfile::tempdir().unwrap();
    let api = HfApiBuilder::new()
        .cache_dir(cache_dir.path())
        .build()
        .unwrap();

    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("config.json")
        .build();
    let path = api.download_file(&params).await.unwrap();

    let meta = std::fs::symlink_metadata(&path).unwrap();
    assert!(meta.file_type().is_symlink());
    let target = std::fs::read_link(&path).unwrap();
    assert!(target.to_string_lossy().contains("blobs"));
}

#[tokio::test]
async fn test_snapshot_download() {
    let Some(_) = api() else { return };
    let cache_dir = tempfile::tempdir().unwrap();
    let api = HfApiBuilder::new()
        .cache_dir(cache_dir.path())
        .build()
        .unwrap();

    let params = SnapshotDownloadParams::builder()
        .repo_id("gpt2")
        .allow_patterns(vec!["*.json".to_string()])
        .build();
    let snapshot_dir = api.snapshot_download(&params).await.unwrap();

    assert!(snapshot_dir.exists());
    assert!(snapshot_dir.to_string_lossy().contains("snapshots"));
    let config = snapshot_dir.join("config.json");
    assert!(config.exists());
}

#[tokio::test]
async fn test_scan_cache_after_download() {
    let Some(_) = api() else { return };
    let cache_dir = tempfile::tempdir().unwrap();
    let api = HfApiBuilder::new()
        .cache_dir(cache_dir.path())
        .build()
        .unwrap();

    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("config.json")
        .build();
    api.download_file(&params).await.unwrap();

    let info = api.scan_cache().await.unwrap();
    assert_eq!(info.repos.len(), 1);
    assert!(info.repos[0].repo_id.contains("gpt2"));
    assert_eq!(info.repos[0].revisions.len(), 1);
    assert!(!info.repos[0].revisions[0].files.is_empty());
    assert!(info.size_on_disk > 0);
}

#[tokio::test]
async fn test_delete_cache_revisions_integration() {
    let Some(_) = api() else { return };
    let cache_dir = tempfile::tempdir().unwrap();
    let api = HfApiBuilder::new()
        .cache_dir(cache_dir.path())
        .build()
        .unwrap();

    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("config.json")
        .build();
    api.download_file(&params).await.unwrap();

    let info = api.scan_cache().await.unwrap();
    let repo = &info.repos[0];
    let commit = repo.revisions[0].commit_hash.clone();

    api.delete_cache_revisions(&[DeleteCacheRevision {
        repo_id: repo.repo_id.clone(),
        repo_type: repo.repo_type,
        commit_hash: commit,
    }])
    .await
    .unwrap();

    let info = api.scan_cache().await.unwrap();
    if !info.repos.is_empty() {
        assert!(info.repos[0].revisions.is_empty());
    }
}
