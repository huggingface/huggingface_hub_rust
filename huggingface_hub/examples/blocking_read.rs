//! Synchronous read operations using HFClientSync.
//!
//! Demonstrates repo info, file listing, downloads, user info, and
//! paginated endpoints — all without an async runtime.
//!
//! Requires HF_TOKEN environment variable.
//! Run: cargo run -p huggingface-hub --features blocking --example blocking_read

use huggingface_hub::{
    DatasetInfoParams, DownloadFileParams, GetPathsInfoParams, HFClientSync, ListDatasetsParams, ListModelsParams,
    ListRepoCommitsParams, ListRepoFilesParams, ListRepoTreeParams, ModelInfoParams, RepoExistsParams, RepoTreeEntry,
    SpaceInfoParams,
};

fn main() -> huggingface_hub::Result<()> {
    let api = HFClientSync::new()?;

    // --- Repo info ---

    let model = api.model_info(&ModelInfoParams::builder().repo_id("gpt2").build())?;
    println!("Model: {} (downloads: {:?})", model.id, model.downloads);

    let dataset = api.dataset_info(&DatasetInfoParams::builder().repo_id("rajpurkar/squad").build())?;
    println!("Dataset: {} (downloads: {:?})", dataset.id, dataset.downloads);

    let space = api.space_info(
        &SpaceInfoParams::builder()
            .repo_id("huggingface/transformers-benchmarks")
            .build(),
    )?;
    println!("Space: {} (sdk: {:?})", space.id, space.sdk);

    let exists = api.repo_exists(&RepoExistsParams::builder().repo_id("gpt2").build())?;
    println!("gpt2 exists: {exists}");

    // --- Listing (streams collected to Vec) ---

    let models = api.list_models(&ListModelsParams::builder().author("openai").build())?;
    println!("\nModels by openai ({} total):", models.len());
    for m in models.iter().take(3) {
        println!("  - {}", m.id);
    }

    let datasets = api.list_datasets(&ListDatasetsParams::builder().search("squad").build())?;
    println!("\nDatasets matching 'squad' ({} total):", datasets.len());
    for ds in datasets.iter().take(3) {
        println!("  - {}", ds.id);
    }

    // --- Files ---

    let files = api.list_repo_files(&ListRepoFilesParams::builder().repo_id("gpt2").build())?;
    println!("\nFiles in gpt2: {}", files.len());
    for f in files.iter().take(5) {
        println!("  - {f}");
    }

    let tree = api.list_repo_tree(&ListRepoTreeParams::builder().repo_id("gpt2").recursive(true).build())?;
    println!("\nTree entries in gpt2:");
    for entry in tree.iter().take(5) {
        match entry {
            RepoTreeEntry::File { path, size, .. } => println!("  file: {path} ({size} bytes)"),
            RepoTreeEntry::Directory { path, .. } => println!("  dir:  {path}"),
        }
    }

    let paths_info = api.get_paths_info(
        &GetPathsInfoParams::builder()
            .repo_id("gpt2")
            .paths(vec!["config.json".to_string(), "README.md".to_string()])
            .build(),
    )?;
    println!("\nPaths info ({} entries):", paths_info.len());

    let tmp_dir = tempfile::tempdir().expect("failed to create tempdir");
    let downloaded = api.download_file(
        &DownloadFileParams::builder()
            .repo_id("gpt2")
            .filename("config.json")
            .local_dir(tmp_dir.path().to_path_buf())
            .build(),
    )?;
    println!("\nDownloaded gpt2/config.json to: {}", downloaded.display());

    // --- Commits ---

    let commits = api.list_repo_commits(&ListRepoCommitsParams::builder().repo_id("openai-community/gpt2").build())?;
    println!("\nRecent commits on gpt2 ({} total):", commits.len());
    for c in commits.iter().take(3) {
        println!("  - {} {}", &c.id[..8], c.title);
    }

    // --- Users ---

    let me = api.whoami()?;
    println!("\nLogged in as: {}", me.username);

    let user = api.get_user_overview("julien-c")?;
    println!("User: {} (fullname: {:?})", user.username, user.fullname);

    let org = api.get_organization_overview("huggingface")?;
    println!("Org: {} (fullname: {:?})", org.name, org.fullname);

    let followers = api.list_user_followers("julien-c")?;
    println!("\nFollowers of julien-c ({} total):", followers.len());
    for u in followers.iter().take(3) {
        println!("  - {}", u.username);
    }

    let members = api.list_organization_members("huggingface")?;
    println!("\nMembers of huggingface ({} total):", members.len());
    for m in members.iter().take(3) {
        println!("  - {}", m.username);
    }

    Ok(())
}
