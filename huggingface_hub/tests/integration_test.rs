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
