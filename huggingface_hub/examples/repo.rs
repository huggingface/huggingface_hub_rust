//! Repository operations: info, listing, existence checks, and CRUD.
//!
//! Requires HF_TOKEN environment variable.
//! Run: cargo run -p huggingface-hub --example repo

use futures::StreamExt;
use huggingface_hub::{
    CreateRepoParams, DatasetInfoParams, DeleteRepoParams, FileExistsParams, HfApi,
    ListDatasetsParams, ListModelsParams, ListSpacesParams, ModelInfoParams, MoveRepoParams,
    RepoExistsParams, RevisionExistsParams, SpaceInfoParams, UpdateRepoParams,
};

#[tokio::main]
async fn main() -> huggingface_hub::Result<()> {
    let api = HfApi::new()?;

    // --- Read operations ---

    let model = api
        .model_info(&ModelInfoParams::builder().repo_id("gpt2").build())
        .await?;
    println!("Model: {} (downloads: {:?})", model.id, model.downloads);

    let dataset = api
        .dataset_info(
            &DatasetInfoParams::builder()
                .repo_id("rajpurkar/squad")
                .build(),
        )
        .await?;
    println!(
        "Dataset: {} (downloads: {:?})",
        dataset.id, dataset.downloads
    );

    let space = api
        .space_info(
            &SpaceInfoParams::builder()
                .repo_id("huggingface/transformers-benchmarks")
                .build(),
        )
        .await?;
    println!("Space: {} (sdk: {:?})", space.id, space.sdk);

    let exists = api
        .repo_exists(&RepoExistsParams::builder().repo_id("gpt2").build())
        .await?;
    println!("gpt2 exists: {exists}");

    let rev_exists = api
        .revision_exists(
            &RevisionExistsParams::builder()
                .repo_id("gpt2")
                .revision("main")
                .build(),
        )
        .await?;
    println!("gpt2@main exists: {rev_exists}");

    let file_exists = api
        .file_exists(
            &FileExistsParams::builder()
                .repo_id("gpt2")
                .filename("config.json")
                .build(),
        )
        .await?;
    println!("gpt2/config.json exists: {file_exists}");

    let models_stream = api.list_models(&ListModelsParams::builder().author("openai").build());
    futures::pin_mut!(models_stream);
    println!("\nModels by openai:");
    let mut count = 0;
    while let Some(Ok(model)) = models_stream.next().await {
        println!("  - {}", model.id);
        count += 1;
        if count >= 3 {
            break;
        }
    }

    let datasets_stream = api.list_datasets(&ListDatasetsParams::builder().search("squad").build());
    futures::pin_mut!(datasets_stream);
    println!("\nDatasets matching 'squad':");
    let mut count = 0;
    while let Some(Ok(ds)) = datasets_stream.next().await {
        println!("  - {}", ds.id);
        count += 1;
        if count >= 3 {
            break;
        }
    }

    let spaces_stream = api.list_spaces(&ListSpacesParams::builder().author("huggingface").build());
    futures::pin_mut!(spaces_stream);
    println!("\nSpaces by huggingface:");
    let mut count = 0;
    while let Some(Ok(sp)) = spaces_stream.next().await {
        println!("  - {}", sp.id);
        count += 1;
        if count >= 3 {
            break;
        }
    }

    // --- Write operations (creates real resources on the Hub) ---

    let user = api.whoami().await?;
    let unique = std::process::id();
    let repo_name = format!("{}/example-repo-{unique}", user.username);

    let repo_url = api
        .create_repo(
            &CreateRepoParams::builder()
                .repo_id(&repo_name)
                .private(true)
                .exist_ok(true)
                .build(),
        )
        .await?;
    println!("\nCreated repo: {}", repo_url.url);

    api.update_repo_settings(
        &UpdateRepoParams::builder()
            .repo_id(&repo_name)
            .description("Temporary example repo")
            .build(),
    )
    .await?;
    println!("Updated repo description");

    let new_name = format!("{}/example-repo-renamed-{unique}", user.username);
    let moved = api
        .move_repo(
            &MoveRepoParams::builder()
                .from_id(&repo_name)
                .to_id(&new_name)
                .build(),
        )
        .await?;
    println!("Moved repo to: {}", moved.url);

    api.delete_repo(
        &DeleteRepoParams::builder()
            .repo_id(&new_name)
            .missing_ok(true)
            .build(),
    )
    .await?;
    println!("Deleted repo");

    Ok(())
}
