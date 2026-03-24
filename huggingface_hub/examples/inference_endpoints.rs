//! Inference Endpoint operations: list, create, update, pause, resume, and delete.
//!
//! WARNING: Creating inference endpoints costs money. The create/delete operations
//! are guarded behind the HF_TEST_IE=1 environment variable.
//!
//! Requires HF_TOKEN and the "inference_endpoints" feature.
//! Run: cargo run -p huggingface-hub --features inference_endpoints --example inference_endpoints

use huggingface_hub::{
    CreateInferenceEndpointParams, DeleteInferenceEndpointParams, GetInferenceEndpointParams, HfApi,
    ListInferenceEndpointsParams, PauseInferenceEndpointParams, ResumeInferenceEndpointParams,
    ScaleToZeroInferenceEndpointParams, UpdateInferenceEndpointParams,
};

#[tokio::main]
async fn main() -> huggingface_hub::Result<()> {
    let api = HfApi::new()?;

    let endpoints = api
        .list_inference_endpoints(&ListInferenceEndpointsParams::builder().build())
        .await?;
    println!("Inference endpoints: {}", endpoints.len());
    for ep in &endpoints {
        println!("  - {:?}", ep);
    }

    if std::env::var("HF_TEST_IE").is_err() {
        println!("\nSet HF_TEST_IE=1 to run create/update/delete operations (costs money)");
        return Ok(());
    }

    let unique = std::process::id();
    let ep_name = format!("example-ep-{unique}");

    let created = api
        .create_inference_endpoint(
            &CreateInferenceEndpointParams::builder()
                .name(&ep_name)
                .repository("gpt2")
                .framework("pytorch")
                .task("text-generation")
                .accelerator("cpu")
                .instance_size("x1")
                .instance_type("c6i")
                .region("us-east-1")
                .vendor("aws")
                .min_replica(0_u32)
                .max_replica(1_u32)
                .build(),
        )
        .await?;
    println!("\nCreated endpoint: {:?}", created);

    let fetched = api
        .get_inference_endpoint(&GetInferenceEndpointParams::builder().name(&ep_name).build())
        .await?;
    println!("Fetched endpoint: {:?}", fetched);

    let updated = api
        .update_inference_endpoint(
            &UpdateInferenceEndpointParams::builder()
                .name(&ep_name)
                .min_replica(0_u32)
                .build(),
        )
        .await?;
    println!("Updated endpoint: {:?}", updated);

    let paused = api
        .pause_inference_endpoint(&PauseInferenceEndpointParams::builder().name(&ep_name).build())
        .await?;
    println!("Paused endpoint: {:?}", paused);

    let resumed = api
        .resume_inference_endpoint(&ResumeInferenceEndpointParams::builder().name(&ep_name).build())
        .await?;
    println!("Resumed endpoint: {:?}", resumed);

    let scaled = api
        .scale_to_zero_inference_endpoint(&ScaleToZeroInferenceEndpointParams::builder().name(&ep_name).build())
        .await?;
    println!("Scaled to zero: {:?}", scaled);

    api.delete_inference_endpoint(&DeleteInferenceEndpointParams::builder().name(&ep_name).build())
        .await?;
    println!("Deleted endpoint");

    Ok(())
}
