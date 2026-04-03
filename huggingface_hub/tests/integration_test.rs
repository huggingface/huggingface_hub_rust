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
use huggingface_hub::repository::{
    HFRepository, RepoCreateBranchParams, RepoCreateCommitParams, RepoCreateTagParams, RepoDeleteBranchParams,
    RepoDeleteFileParams, RepoDeleteFolderParams, RepoDeleteTagParams, RepoDownloadFileParams, RepoFileExistsParams,
    RepoGetCommitDiffParams, RepoGetPathsInfoParams, RepoGetRawDiffParams, RepoInfoParams, RepoListCommitsParams,
    RepoListFilesParams, RepoListRefsParams, RepoListTreeParams, RepoRevisionExistsParams, RepoUpdateSettingsParams,
    RepoUploadFileParams, RepoUploadFolderParams,
};
#[cfg(feature = "discussions")]
use huggingface_hub::repository::{
    RepoCreateDiscussionParams, RepoCreatePullRequestParams, RepoDiscussionDetailsParams, RepoListDiscussionsParams,
};
use huggingface_hub::types::*;
use huggingface_hub::{HFClient, HFClientBuilder};
#[cfg(feature = "spaces")]
use huggingface_hub::{SpaceSecretDeleteParams, SpaceSecretParams, SpaceVariableDeleteParams, SpaceVariableParams};

fn api() -> Option<HFClient> {
    if std::env::var("HF_TOKEN").is_err() {
        return None;
    }
    Some(HFClientBuilder::new().build().expect("Failed to create HFClient"))
}

fn write_enabled() -> bool {
    std::env::var("HF_TEST_WRITE").ok().is_some_and(|v| v == "1")
}

/// Cached whoami username, fetched once and reused across all tests.
async fn cached_username() -> &'static str {
    static USERNAME: tokio::sync::OnceCell<String> = tokio::sync::OnceCell::const_new();
    USERNAME
        .get_or_init(|| async {
            let client = HFClientBuilder::new().build().expect("Failed to create HFClient for whoami");
            client.whoami().await.expect("whoami failed").username
        })
        .await
}

/// Create an HFRepository handle from a full `owner/name` repo_id string.
fn repo(api: &HFClient, repo_id: &str) -> HFRepository {
    let parts: Vec<&str> = repo_id.splitn(2, '/').collect();
    if parts.len() == 2 {
        api.model(parts[0], parts[1])
    } else {
        api.model("", repo_id)
    }
}

/// Create an HFRepository handle with a specific repo type.
fn repo_typed(api: &HFClient, repo_id: &str, repo_type: RepoType) -> HFRepository {
    let parts: Vec<&str> = repo_id.splitn(2, '/').collect();
    let (owner, name) = if parts.len() == 2 {
        (parts[0], parts[1])
    } else {
        ("", repo_id)
    };
    api.repo(repo_type, owner, name)
}

#[tokio::test]
async fn test_model_info() {
    let Some(api) = api() else { return };
    let info = repo(&api, "gpt2").info(&RepoInfoParams::default()).await.unwrap();
    match info {
        RepoInfo::Model(model) => assert!(model.id.contains("gpt2")),
        _ => panic!("expected model info"),
    }
}

#[tokio::test]
async fn test_repo_handle_info_and_file_exists() {
    let Some(api) = api() else { return };
    let repo = api.model("openai-community", "gpt2");

    let info = repo.info(&RepoInfoParams::default()).await.unwrap();
    match info {
        RepoInfo::Model(model) => assert_eq!(model.id, "openai-community/gpt2"),
        _ => panic!("expected model info"),
    }

    let exists = repo
        .file_exists(&RepoFileExistsParams::builder().filename("config.json").build())
        .await
        .unwrap();
    assert!(exists);
}

#[cfg(feature = "discussions")]
#[tokio::test]
async fn test_repo_handle_list_discussions() {
    let Some(api) = api() else { return };
    let repo = api.model("openai-community", "gpt2");

    let response = repo.list_discussions(&RepoListDiscussionsParams::default()).await.unwrap();
    assert!(response.count.unwrap_or(0) as usize >= response.discussions.len());
}

#[cfg(feature = "likes")]
#[tokio::test]
async fn test_repo_handle_list_likers() {
    let Some(api) = api() else { return };
    let repo = api.model("openai-community", "gpt2");

    let stream = repo.list_likers(None).unwrap();
    futures::pin_mut!(stream);
    let mut seen = 0usize;
    while let Some(user) = stream.next().await {
        user.unwrap();
        seen += 1;
        if seen >= 3 {
            break;
        }
    }
}

#[tokio::test]
async fn test_dataset_info() {
    let Some(api) = api() else { return };
    let info = repo_typed(&api, "rajpurkar/squad", RepoType::Dataset)
        .info(&RepoInfoParams::default())
        .await
        .unwrap();
    match info {
        RepoInfo::Dataset(ds) => assert!(ds.id.contains("squad")),
        _ => panic!("expected dataset info"),
    }
}

#[tokio::test]
async fn test_repo_exists() {
    let Some(api) = api() else { return };
    assert!(repo(&api, "gpt2").exists().await.unwrap());
    assert!(!repo(&api, "this-repo-definitely-does-not-exist-12345").exists().await.unwrap());
}

#[tokio::test]
async fn test_file_exists() {
    let Some(api) = api() else { return };
    assert!(repo(&api, "gpt2")
        .file_exists(&RepoFileExistsParams::builder().filename("config.json").build())
        .await
        .unwrap());

    assert!(!repo(&api, "gpt2")
        .file_exists(&RepoFileExistsParams::builder().filename("nonexistent_file.xyz").build())
        .await
        .unwrap());
}

#[tokio::test]
async fn test_list_models() {
    let Some(api) = api() else { return };
    let params = ListModelsParams::builder().author("openai-community").limit(3_usize).build();
    let stream = api.list_models(&params).unwrap();
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
    let files = repo(&api, "gpt2").list_files(&RepoListFilesParams::default()).await.unwrap();
    assert!(files.contains(&"config.json".to_string()));
    assert!(files.contains(&"README.md".to_string()));
}

#[tokio::test]
async fn test_list_repo_tree() {
    let Some(api) = api() else { return };
    let r = repo(&api, "gpt2");
    let stream = r.list_tree(&RepoListTreeParams::default()).unwrap();
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
    let r = repo(&api, "gpt2");
    let stream = r.list_commits(&RepoListCommitsParams::default()).unwrap();
    futures::pin_mut!(stream);

    let first = stream.next().await.unwrap().unwrap();
    assert!(!first.id.is_empty());
    assert!(!first.title.is_empty());
}

#[tokio::test]
async fn test_list_repo_refs() {
    let Some(api) = api() else { return };
    let refs = repo(&api, "gpt2").list_refs(&RepoListRefsParams::default()).await.unwrap();
    assert!(!refs.branches.is_empty());
    // "main" branch should exist
    assert!(refs.branches.iter().any(|b| b.name == "main"));
}

#[tokio::test]
async fn test_revision_exists() {
    let Some(api) = api() else { return };
    assert!(repo(&api, "gpt2")
        .revision_exists(&RepoRevisionExistsParams::builder().revision("main").build())
        .await
        .unwrap());

    assert!(!repo(&api, "gpt2")
        .revision_exists(&RepoRevisionExistsParams::builder().revision("nonexistent-branch-xyz").build())
        .await
        .unwrap());
}

#[tokio::test]
async fn test_download_file() {
    let Some(api) = api() else { return };
    let dir = tempfile::tempdir().unwrap();
    let path = repo(&api, "gpt2")
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
    let stream = api.list_user_followers("julien-c", None).unwrap();
    futures::pin_mut!(stream);
    let first = stream.next().await;
    assert!(first.is_some());
    first.unwrap().unwrap();
}

#[tokio::test]
async fn test_list_user_following() {
    let Some(api) = api() else { return };
    let stream = api.list_user_following("julien-c", None).unwrap();
    futures::pin_mut!(stream);
    let first = stream.next().await;
    assert!(first.is_some());
    first.unwrap().unwrap();
}

#[tokio::test]
async fn test_list_organization_members() {
    let Some(api) = api() else { return };
    let stream = api.list_organization_members("huggingface", None).unwrap();
    futures::pin_mut!(stream);
    let first = stream.next().await;
    assert!(first.is_some());
    first.unwrap().unwrap();
}

// --- Additional repo info tests ---

#[tokio::test]
async fn test_space_info() {
    let Some(api) = api() else { return };
    let info = repo_typed(&api, "HuggingFaceFW/blogpost-fineweb-v1", RepoType::Space)
        .info(&RepoInfoParams::default())
        .await
        .unwrap();
    match info {
        RepoInfo::Space(space) => assert!(space.id.contains("blogpost-fineweb-v1")),
        _ => panic!("expected space info"),
    }
}

#[tokio::test]
async fn test_list_datasets() {
    let Some(api) = api() else { return };
    let params = ListDatasetsParams::builder().author("huggingface").limit(3_usize).build();
    let stream = api.list_datasets(&params).unwrap();
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
    let params = ListSpacesParams::builder().author("huggingface").limit(3_usize).build();
    let stream = api.list_spaces(&params).unwrap();
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
    let entries = repo(&api, "gpt2")
        .get_paths_info(
            &RepoGetPathsInfoParams::builder()
                .paths(vec!["config.json".to_string(), "README.md".to_string()])
                .build(),
        )
        .await
        .unwrap();
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

    let gpt2 = repo(&api, "openai-community/gpt2");
    let stream = gpt2.list_commits(&RepoListCommitsParams::default()).unwrap();
    futures::pin_mut!(stream);

    let first = stream.next().await.unwrap().unwrap();
    let second = stream.next().await.unwrap().unwrap();

    let diff = gpt2
        .get_commit_diff(
            &RepoGetCommitDiffParams::builder()
                .compare(format!("{}..{}", second.id, first.id))
                .build(),
        )
        .await
        .unwrap();
    assert!(!diff.is_empty());
}

#[tokio::test]
async fn test_get_raw_diff() {
    let Some(api) = api() else { return };

    let gpt2 = repo(&api, "openai-community/gpt2");
    let stream = gpt2.list_commits(&RepoListCommitsParams::default()).unwrap();
    futures::pin_mut!(stream);

    let first = stream.next().await.unwrap().unwrap();
    let second = stream.next().await.unwrap().unwrap();

    let raw = gpt2
        .get_raw_diff(
            &RepoGetRawDiffParams::builder()
                .compare(format!("{}..{}", second.id, first.id))
                .build(),
        )
        .await
        .unwrap();
    assert!(!raw.is_empty());
}

// --- Write operation tests (require HF_TEST_WRITE=1) ---

#[tokio::test]
async fn test_create_and_delete_repo() {
    let Some(api) = api() else { return };
    if !write_enabled() {
        return;
    }

    let username = cached_username().await;

    let repo_id = format!("{}/huggingface-hub-rust-test-{}", username, uuid_v4_short());

    // Create
    let params = CreateRepoParams::builder()
        .repo_id(&repo_id)
        .private(true)
        .exist_ok(true)
        .build();
    let url = api.create_repo(&params).await.unwrap();
    assert!(url.url.contains(&repo_id));

    // Upload a file
    let test_repo = repo(&api, &repo_id);
    let commit = test_repo
        .upload_file(
            &RepoUploadFileParams::builder()
                .source(AddSource::Bytes(b"hello world".to_vec()))
                .path_in_repo("test.txt")
                .commit_message("test upload")
                .build(),
        )
        .await
        .unwrap();
    assert!(commit.commit_oid.is_some());

    // Verify file exists
    assert!(test_repo
        .file_exists(&RepoFileExistsParams::builder().filename("test.txt").build())
        .await
        .unwrap());

    // Delete repo
    let params = DeleteRepoParams::builder().repo_id(&repo_id).build();
    api.delete_repo(&params).await.unwrap();
}

fn uuid_v4_short() -> String {
    format!("{:016x}", rand::random::<u64>())
}

async fn create_test_repo(api: &HFClient) -> String {
    let username = cached_username().await;
    let repo_id = format!("{}/huggingface-hub-rust-test-{}", username, uuid_v4_short());
    let params = CreateRepoParams::builder()
        .repo_id(&repo_id)
        .private(true)
        .exist_ok(false)
        .build();
    api.create_repo(&params).await.expect("create_repo failed");

    let test_repo = repo(api, &repo_id);
    test_repo
        .upload_file(
            &RepoUploadFileParams::builder()
                .source(AddSource::Bytes(b"initial content".to_vec()))
                .path_in_repo("README.md")
                .commit_message("initial commit")
                .build(),
        )
        .await
        .expect("seed upload failed");

    repo_id
}

async fn delete_test_repo(api: &HFClient, repo_id: &str) {
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

    let test_repo = repo(&api, &repo_id);
    let commit = test_repo
        .create_commit(
            &RepoCreateCommitParams::builder()
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
                .build(),
        )
        .await
        .unwrap();
    assert!(commit.commit_oid.is_some());

    let files = test_repo.list_files(&RepoListFilesParams::default()).await.unwrap();
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

    let test_repo = repo(&api, &repo_id);
    let commit = test_repo
        .upload_folder(
            &RepoUploadFolderParams::builder()
                .folder_path(dir.path().to_path_buf())
                .commit_message("upload folder")
                .build(),
        )
        .await
        .unwrap();
    assert!(commit.commit_oid.is_some());

    let files = test_repo.list_files(&RepoListFilesParams::default()).await.unwrap();
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

    let test_repo = repo(&api, &repo_id);
    test_repo
        .upload_file(
            &RepoUploadFileParams::builder()
                .source(AddSource::Bytes(b"to delete".to_vec()))
                .path_in_repo("deleteme.txt")
                .commit_message("add file to delete")
                .build(),
        )
        .await
        .unwrap();

    test_repo
        .delete_file(
            &RepoDeleteFileParams::builder()
                .path_in_repo("deleteme.txt")
                .commit_message("delete file")
                .build(),
        )
        .await
        .unwrap();

    assert!(!test_repo
        .file_exists(&RepoFileExistsParams::builder().filename("deleteme.txt").build())
        .await
        .unwrap());

    delete_test_repo(&api, &repo_id).await;
}

#[tokio::test]
async fn test_delete_folder() {
    let Some(api) = api() else { return };
    if !write_enabled() {
        return;
    }
    let repo_id = create_test_repo(&api).await;

    let test_repo = repo(&api, &repo_id);
    test_repo
        .create_commit(
            &RepoCreateCommitParams::builder()
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
                .build(),
        )
        .await
        .unwrap();

    test_repo
        .delete_folder(
            &RepoDeleteFolderParams::builder()
                .path_in_repo("folder")
                .commit_message("delete folder")
                .build(),
        )
        .await
        .unwrap();

    assert!(!test_repo
        .file_exists(&RepoFileExistsParams::builder().filename("folder/a.txt").build())
        .await
        .unwrap());

    delete_test_repo(&api, &repo_id).await;
}

#[tokio::test]
async fn test_create_and_delete_branch() {
    let Some(api) = api() else { return };
    if !write_enabled() {
        return;
    }
    let repo_id = create_test_repo(&api).await;

    let test_repo = repo(&api, &repo_id);
    test_repo
        .create_branch(&RepoCreateBranchParams::builder().branch("test-branch").build())
        .await
        .unwrap();

    let refs = test_repo.list_refs(&RepoListRefsParams::default()).await.unwrap();
    assert!(refs.branches.iter().any(|b| b.name == "test-branch"));

    test_repo
        .delete_branch(&RepoDeleteBranchParams::builder().branch("test-branch").build())
        .await
        .unwrap();

    let refs = test_repo.list_refs(&RepoListRefsParams::default()).await.unwrap();
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

    let test_repo = repo(&api, &repo_id);
    test_repo
        .create_tag(&RepoCreateTagParams::builder().tag("v1.0").build())
        .await
        .unwrap();

    let refs = test_repo.list_refs(&RepoListRefsParams::default()).await.unwrap();
    assert!(refs.tags.iter().any(|t| t.name == "v1.0"));

    test_repo
        .delete_tag(&RepoDeleteTagParams::builder().tag("v1.0").build())
        .await
        .unwrap();

    let refs = test_repo.list_refs(&RepoListRefsParams::default()).await.unwrap();
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

    let test_repo = repo(&api, &repo_id);
    test_repo
        .update_settings(
            &RepoUpdateSettingsParams::builder()
                .description("test description from integration test")
                .build(),
        )
        .await
        .unwrap();

    // Verify we can still get info after update
    let _info = test_repo.info(&RepoInfoParams::default()).await.unwrap();

    delete_test_repo(&api, &repo_id).await;
}

#[tokio::test]
async fn test_move_repo() {
    let Some(api) = api() else { return };
    if !write_enabled() {
        return;
    }
    let username = cached_username().await;
    let original_name = format!("{}/huggingface-hub-rust-move-src-{}", username, uuid_v4_short());
    let new_name = format!("{}/huggingface-hub-rust-move-dst-{}", username, uuid_v4_short());

    let create_params = CreateRepoParams::builder().repo_id(&original_name).private(true).build();
    api.create_repo(&create_params).await.unwrap();

    let move_params = MoveRepoParams::builder().from_id(&original_name).to_id(&new_name).build();
    api.move_repo(&move_params).await.unwrap();

    assert!(repo(&api, &new_name).exists().await.unwrap());

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
    let space = api.space("huggingface-projects", "diffusers-gallery");
    let runtime = space.runtime().await.unwrap();
    assert!(runtime.stage.is_some());
}

#[cfg(feature = "spaces")]
#[tokio::test]
async fn test_duplicate_space() {
    let Some(api) = api() else { return };
    if !write_enabled() {
        return;
    }
    let username = cached_username().await;
    let to_id = format!("{}/hub-rust-test-dup-space-{}", username, uuid_v4_short());

    let params = DuplicateSpaceParams::builder()
        .to_id(&to_id)
        .private(true)
        .hardware("cpu-basic")
        .build();
    let source = api.space("huggingface-projects", "diffusers-gallery");
    let result = source.duplicate(&params).await.unwrap();
    assert!(result.url.contains(&to_id));

    let delete_params = DeleteRepoParams::builder().repo_id(&to_id).repo_type(RepoType::Space).build();
    let _ = api.delete_repo(&delete_params).await;
}

#[cfg(feature = "spaces")]
#[tokio::test]
async fn test_space_secrets_and_variables() {
    let Some(api) = api() else { return };
    if !write_enabled() {
        return;
    }
    let username = cached_username().await;
    let space = api.space(username, format!("hub-rust-test-space-{}", uuid_v4_short()));
    let create_params = CreateRepoParams::builder()
        .repo_id(space.repo_path())
        .repo_type(RepoType::Space)
        .private(true)
        .space_sdk("static")
        .build();
    api.create_repo(&create_params).await.unwrap();

    let add_secret = SpaceSecretParams::builder().key("TEST_SECRET").value("secret_value").build();
    space.add_secret(&add_secret).await.unwrap();

    let del_secret = SpaceSecretDeleteParams::builder().key("TEST_SECRET").build();
    space.delete_secret(&del_secret).await.unwrap();

    let add_var = SpaceVariableParams::builder().key("TEST_VAR").value("var_value").build();
    space.add_variable(&add_var).await.unwrap();

    let del_var = SpaceVariableDeleteParams::builder().key("TEST_VAR").build();
    space.delete_variable(&del_var).await.unwrap();

    let delete_params = DeleteRepoParams::builder()
        .repo_id(space.repo_path())
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
    let params = ListCollectionsParams::builder().owner("huggingface").limit(3_usize).build();
    let collections = api.list_collections(&params).await.unwrap();
    assert!(!collections.is_empty());
    assert!(collections[0].slug.contains("huggingface"));
}

#[cfg(feature = "collections")]
#[tokio::test]
async fn test_get_collection() {
    let Some(api) = api() else { return };
    let list_params = ListCollectionsParams::builder().owner("huggingface").limit(1_usize).build();
    let collections = api.list_collections(&list_params).await.unwrap();
    assert!(!collections.is_empty());

    let params = GetCollectionParams::builder().slug(&collections[0].slug).build();
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

    let username = cached_username().await;
    let title = format!("hub-rust-test-collection-{}", uuid_v4_short());
    let create_params = CreateCollectionParams::builder()
        .title(&title)
        .namespace(username)
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
    let response = repo(&api, "openai-community/gpt2")
        .list_discussions(&RepoListDiscussionsParams::default())
        .await
        .unwrap();
    assert!(!response.discussions.is_empty());
    assert!(response.discussions[0].num > 0);
}

#[cfg(feature = "discussions")]
#[tokio::test]
async fn test_get_discussion_details() {
    let Some(api) = api() else { return };
    let details = repo(&api, "openai-community/gpt2")
        .discussion_details(&RepoDiscussionDetailsParams::builder().discussion_num(1_u64).build())
        .await
        .unwrap();
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
    let test_repo = repo(&api, &repo_id);

    let disc_response = test_repo.list_discussions(&RepoListDiscussionsParams::default()).await.unwrap();
    let initial_count = disc_response.discussions.len();

    let _disc = test_repo
        .create_discussion(
            &RepoCreateDiscussionParams::builder()
                .title("Test discussion from integration test")
                .description("This is a test")
                .build(),
        )
        .await
        .unwrap();

    let disc_response = test_repo.list_discussions(&RepoListDiscussionsParams::default()).await.unwrap();
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
    let test_repo = repo(&api, &repo_id);

    let _pr = test_repo
        .create_pull_request(
            &RepoCreatePullRequestParams::builder()
                .title("Test PR from integration test")
                .description("")
                .build(),
        )
        .await
        .unwrap();

    let disc_response = test_repo.list_discussions(&RepoListDiscussionsParams::default()).await.unwrap();
    assert!(disc_response.discussions.iter().any(|d| d.is_pull_request == Some(true)));

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

    let username = cached_username().await;
    let create_params = CreateWebhookParams::builder()
        .url("https://example.com/test-webhook")
        .watched(vec![serde_json::json!({"type": "user", "name": username})])
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
                w.url.as_deref() == Some("https://example.com/test-webhook") && !initial.iter().any(|i| i.id == w.id)
            })
            .expect("should find newly created webhook");
        api.delete_webhook(newest.id.as_ref().unwrap()).await.unwrap();
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

    let test_repo = repo(&api, &repo_id);
    test_repo
        .update_settings(&RepoUpdateSettingsParams::builder().gated("auto").build())
        .await
        .unwrap();

    let pending = test_repo.list_pending_access_requests().await.unwrap();
    assert!(pending.is_empty());

    let accepted = test_repo.list_accepted_access_requests().await.unwrap();
    // auto-approved gated repos may have entries, but no error
    let _ = accepted;

    let rejected = test_repo.list_rejected_access_requests().await.unwrap();
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
    let gpt2 = repo(&api, "openai-community/gpt2");
    let stream = gpt2.list_likers(None).unwrap();
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
    let test_repo = repo(&api, &repo_id);
    if let Err(e) = test_repo.like().await {
        eprintln!("like failed (token may lack write scope for likes): {e}");
        delete_test_repo(&api, &repo_id).await;
        return;
    }

    let username = cached_username().await;
    let list_params = ListLikedReposParams::builder().username(username).build();
    let likes = api.list_liked_repos(&list_params).await.unwrap();
    assert!(likes.iter().any(|l| {
        l.repo
            .as_ref()
            .and_then(|r| r.name.as_deref())
            .is_some_and(|n| n.contains(&repo_id))
    }));

    test_repo.unlike().await.unwrap();
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
    let params = ListPapersParams::builder().query("attention").limit(5_usize).build();
    let results = api.list_papers(&params).await.unwrap();
    assert!(!results.is_empty());
    assert!(results[0].paper.is_some());
}

#[cfg(feature = "papers")]
#[tokio::test]
async fn test_list_daily_papers() {
    let Some(api) = api() else { return };
    let params = ListDailyPapersParams::builder().date("2024-10-29").limit(5_usize).build();
    let papers = api.list_daily_papers(&params).await.unwrap();
    assert!(!papers.is_empty());
    assert!(papers[0].paper.is_some());
}
