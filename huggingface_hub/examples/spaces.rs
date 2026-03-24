//! Space operations: runtime info, secrets, variables, and lifecycle management.
//!
//! Requires HF_TOKEN and the "spaces" feature.
//! Run: cargo run -p huggingface-hub --features spaces --example spaces

use huggingface_hub::{
    AddSpaceSecretParams, AddSpaceVariableParams, CreateRepoParams, DeleteRepoParams, DeleteSpaceSecretParams,
    DeleteSpaceVariableParams, GetSpaceRuntimeParams, HfApi, PauseSpaceParams, RepoType, RestartSpaceParams,
};

#[tokio::main]
async fn main() -> huggingface_hub::Result<()> {
    let api = HfApi::new()?;

    // --- Read operations ---

    let runtime = api
        .get_space_runtime(
            &GetSpaceRuntimeParams::builder()
                .repo_id("huggingface/transformers-benchmarks")
                .build(),
        )
        .await?;
    println!("Space runtime: {runtime:?}");

    // --- Write operations (creates real resources on the Hub) ---

    let user = api.whoami().await?;
    let unique = std::process::id();
    let space_name = format!("{}/example-space-{unique}", user.username);

    api.create_repo(
        &CreateRepoParams::builder()
            .repo_id(&space_name)
            .repo_type(RepoType::Space)
            .private(true)
            .space_sdk("static")
            .exist_ok(true)
            .build(),
    )
    .await?;
    println!("\nCreated test space: {space_name}");

    api.add_space_secret(
        &AddSpaceSecretParams::builder()
            .repo_id(&space_name)
            .key("EXAMPLE_SECRET")
            .value("secret-value")
            .build(),
    )
    .await?;
    println!("Added secret: EXAMPLE_SECRET");

    api.delete_space_secret(
        &DeleteSpaceSecretParams::builder()
            .repo_id(&space_name)
            .key("EXAMPLE_SECRET")
            .build(),
    )
    .await?;
    println!("Deleted secret: EXAMPLE_SECRET");

    api.add_space_variable(
        &AddSpaceVariableParams::builder()
            .repo_id(&space_name)
            .key("EXAMPLE_VAR")
            .value("var-value")
            .build(),
    )
    .await?;
    println!("Added variable: EXAMPLE_VAR");

    api.delete_space_variable(
        &DeleteSpaceVariableParams::builder()
            .repo_id(&space_name)
            .key("EXAMPLE_VAR")
            .build(),
    )
    .await?;
    println!("Deleted variable: EXAMPLE_VAR");

    let paused = api
        .pause_space(&PauseSpaceParams::builder().repo_id(&space_name).build())
        .await?;
    println!("Paused space: {paused:?}");

    let restarted = api
        .restart_space(&RestartSpaceParams::builder().repo_id(&space_name).build())
        .await?;
    println!("Restarted space: {restarted:?}");

    api.delete_repo(
        &DeleteRepoParams::builder()
            .repo_id(&space_name)
            .repo_type(RepoType::Space)
            .missing_ok(true)
            .build(),
    )
    .await?;
    println!("Cleaned up test space");

    Ok(())
}
