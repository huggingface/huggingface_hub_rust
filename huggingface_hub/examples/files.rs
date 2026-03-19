//! File operations: listing, downloading, uploading, and committing.
//!
//! Requires HF_TOKEN environment variable.
//! Run: cargo run -p huggingface-hub --example files

use futures::StreamExt;
use huggingface_hub::{
    AddSource, CommitOperation, CreateCommitParams, CreateRepoParams, DeleteFileParams,
    DeleteFolderParams, DeleteRepoParams, DownloadFileParams, GetPathsInfoParams, HfApi,
    ListRepoFilesParams, ListRepoTreeParams, RepoTreeEntry, UploadFileParams, UploadFolderParams,
};
#[tokio::main]
async fn main() -> huggingface_hub::Result<()> {
    let api = HfApi::new()?;

    // --- Read operations ---

    let files = api
        .list_repo_files(&ListRepoFilesParams::builder().repo_id("gpt2").build())
        .await?;
    println!("Files in gpt2: {}", files.len());
    for f in files.iter().take(5) {
        println!("  - {f}");
    }

    let tree_stream = api.list_repo_tree(
        &ListRepoTreeParams::builder()
            .repo_id("gpt2")
            .recursive(true)
            .build(),
    );
    futures::pin_mut!(tree_stream);
    println!("\nTree entries in gpt2:");
    let mut count = 0;
    while let Some(Ok(entry)) = tree_stream.next().await {
        match &entry {
            RepoTreeEntry::File { path, size, .. } => println!("  file: {path} ({size} bytes)"),
            RepoTreeEntry::Directory { path, .. } => println!("  dir:  {path}"),
        }
        count += 1;
        if count >= 5 {
            break;
        }
    }

    let paths_info = api
        .get_paths_info(
            &GetPathsInfoParams::builder()
                .repo_id("gpt2")
                .paths(vec!["config.json".to_string(), "README.md".to_string()])
                .build(),
        )
        .await?;
    println!("\nPaths info for gpt2:");
    for entry in &paths_info {
        println!("  {entry:?}");
    }

    let tmp_dir = tempfile::tempdir().expect("failed to create tempdir");
    let downloaded = api
        .download_file(
            &DownloadFileParams::builder()
                .repo_id("gpt2")
                .filename("config.json")
                .local_dir(tmp_dir.path().to_path_buf())
                .build(),
        )
        .await?;
    println!("\nDownloaded gpt2/config.json to: {}", downloaded.display());

    // --- Write operations (creates real resources on the Hub) ---

    let user = api.whoami().await?;
    let unique = std::process::id();
    let repo_name = format!("{}/example-files-{unique}", user.username);

    api.create_repo(
        &CreateRepoParams::builder()
            .repo_id(&repo_name)
            .private(true)
            .exist_ok(true)
            .build(),
    )
    .await?;
    println!("\nCreated test repo: {repo_name}");

    let commit = api
        .upload_file(
            &UploadFileParams::builder()
                .repo_id(&repo_name)
                .source(AddSource::Bytes(b"Hello from Rust!".to_vec()))
                .path_in_repo("hello.txt")
                .commit_message("Add hello.txt via example")
                .build(),
        )
        .await?;
    println!("Uploaded hello.txt: {:?}", commit.commit_url);

    let commit = api
        .create_commit(
            &CreateCommitParams::builder()
                .repo_id(&repo_name)
                .operations(vec![
                    CommitOperation::Add {
                        path_in_repo: "data/file1.txt".to_string(),
                        source: AddSource::Bytes(b"File 1 content".to_vec()),
                    },
                    CommitOperation::Add {
                        path_in_repo: "data/file2.txt".to_string(),
                        source: AddSource::Bytes(b"File 2 content".to_vec()),
                    },
                ])
                .commit_message("Add data files via create_commit")
                .build(),
        )
        .await?;
    println!("Created commit with 2 files: {:?}", commit.commit_oid);

    let upload_dir = tmp_dir.path().join("upload_folder");
    std::fs::create_dir_all(upload_dir.join("subdir")).expect("failed to create dir");
    std::fs::write(upload_dir.join("root.txt"), "root file").expect("failed to write");
    std::fs::write(upload_dir.join("subdir/nested.txt"), "nested file").expect("failed to write");

    let commit = api
        .upload_folder(
            &UploadFolderParams::builder()
                .repo_id(&repo_name)
                .folder_path(upload_dir)
                .path_in_repo("uploaded")
                .commit_message("Upload folder via example")
                .build(),
        )
        .await?;
    println!("Uploaded folder: {:?}", commit.commit_oid);

    api.delete_file(
        &DeleteFileParams::builder()
            .repo_id(&repo_name)
            .path_in_repo("hello.txt")
            .build(),
    )
    .await?;
    println!("Deleted hello.txt");

    api.delete_folder(
        &DeleteFolderParams::builder()
            .repo_id(&repo_name)
            .path_in_repo("data")
            .build(),
    )
    .await?;
    println!("Deleted data/ folder");

    api.delete_repo(
        &DeleteRepoParams::builder()
            .repo_id(&repo_name)
            .missing_ok(true)
            .build(),
    )
    .await?;
    println!("Cleaned up test repo");

    Ok(())
}
