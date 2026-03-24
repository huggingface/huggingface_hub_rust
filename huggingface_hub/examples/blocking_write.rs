//! Synchronous write operations using HfApiSync.
//!
//! Creates a temporary repo, uploads files, manages branches, and cleans up.
//!
//! Requires HF_TOKEN environment variable.
//! Run: cargo run -p huggingface-hub --features blocking --example blocking_write

use huggingface_hub::{
    AddSource, CommitOperation, CreateBranchParams, CreateCommitParams, CreateRepoParams, CreateTagParams,
    DeleteBranchParams, DeleteFileParams, DeleteRepoParams, DeleteTagParams, DownloadFileParams, FileExistsParams,
    HfApiSync, ListRepoFilesParams, ListRepoRefsParams, UploadFileParams, UploadFolderParams,
};

fn main() -> huggingface_hub::Result<()> {
    let api = HfApiSync::new()?;
    let user = api.whoami()?;
    let unique = std::process::id();
    let repo_name = format!("{}/sync-example-{unique}", user.username);

    // --- Create repo ---

    let repo_url = api.create_repo(
        &CreateRepoParams::builder()
            .repo_id(&repo_name)
            .private(true)
            .exist_ok(true)
            .build(),
    )?;
    println!("Created repo: {}", repo_url.url);

    // --- Upload a single file ---

    let commit = api.upload_file(
        &UploadFileParams::builder()
            .repo_id(&repo_name)
            .source(AddSource::Bytes(b"Hello from HfApiSync!".to_vec()))
            .path_in_repo("hello.txt")
            .commit_message("Add hello.txt")
            .build(),
    )?;
    println!("Uploaded hello.txt: {:?}", commit.commit_url);

    // --- Create a multi-file commit ---

    let commit = api.create_commit(
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
            .commit_message("Add data files")
            .build(),
    )?;
    println!("Created commit: {:?}", commit.commit_oid);

    // --- Upload a folder ---

    let tmp_dir = tempfile::tempdir().expect("failed to create tempdir");
    std::fs::write(tmp_dir.path().join("root.txt"), "root file").unwrap();
    std::fs::create_dir_all(tmp_dir.path().join("subdir")).unwrap();
    std::fs::write(tmp_dir.path().join("subdir/nested.txt"), "nested file").unwrap();

    let commit = api.upload_folder(
        &UploadFolderParams::builder()
            .repo_id(&repo_name)
            .folder_path(tmp_dir.path().to_path_buf())
            .path_in_repo("uploaded")
            .commit_message("Upload folder")
            .build(),
    )?;
    println!("Uploaded folder: {:?}", commit.commit_oid);

    // --- List files ---

    let files = api.list_repo_files(&ListRepoFilesParams::builder().repo_id(&repo_name).build())?;
    println!("\nAll files in repo:");
    for f in &files {
        println!("  - {f}");
    }

    // --- Download a file ---

    let download_dir = tempfile::tempdir().expect("failed to create tempdir");
    let path = api.download_file(
        &DownloadFileParams::builder()
            .repo_id(&repo_name)
            .filename("hello.txt")
            .local_dir(download_dir.path().to_path_buf())
            .build(),
    )?;
    let content = std::fs::read_to_string(&path).unwrap();
    println!("\nDownloaded hello.txt: {content:?}");

    // --- Branch and tag management ---

    api.create_branch(&CreateBranchParams::builder().repo_id(&repo_name).branch("dev").build())?;
    println!("\nCreated branch 'dev'");

    api.create_tag(
        &CreateTagParams::builder()
            .repo_id(&repo_name)
            .tag("v1.0")
            .message("First release")
            .build(),
    )?;
    println!("Created tag 'v1.0'");

    let refs = api.list_repo_refs(&ListRepoRefsParams::builder().repo_id(&repo_name).build())?;
    println!("Branches: {:?}", refs.branches.iter().map(|b| &b.name).collect::<Vec<_>>());
    println!("Tags: {:?}", refs.tags.iter().map(|t| &t.name).collect::<Vec<_>>());

    api.delete_tag(&DeleteTagParams::builder().repo_id(&repo_name).tag("v1.0").build())?;
    api.delete_branch(&DeleteBranchParams::builder().repo_id(&repo_name).branch("dev").build())?;
    println!("Cleaned up branch and tag");

    // --- Delete a file ---

    api.delete_file(
        &DeleteFileParams::builder()
            .repo_id(&repo_name)
            .path_in_repo("hello.txt")
            .build(),
    )?;
    let gone = !api.file_exists(&FileExistsParams::builder().repo_id(&repo_name).filename("hello.txt").build())?;
    println!("\nhello.txt deleted: {gone}");

    // --- Clean up ---

    api.delete_repo(&DeleteRepoParams::builder().repo_id(&repo_name).missing_ok(true).build())?;
    println!("Deleted repo");

    Ok(())
}
