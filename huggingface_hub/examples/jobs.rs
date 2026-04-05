//! Job operations: list hardware, run jobs, inspect, cancel, and scheduled jobs.
//!
//! Requires HF_TOKEN and the "jobs" feature.
//! Run: cargo run -p huggingface-hub --features jobs --example jobs

use huggingface_hub::{CreateScheduledJobParams, HFClient, ListJobsParams, RunJobParams};

#[tokio::main]
async fn main() -> huggingface_hub::Result<()> {
    let api = HFClient::new()?;

    let hardware = api.list_job_hardware().await?;
    println!("Available job hardware:");
    for hw in &hardware {
        println!("  - {:?}", hw);
    }

    let jobs = api.list_jobs(&ListJobsParams::builder().build()).await?;
    println!("\nExisting jobs: {}", jobs.len());
    for job in jobs.iter().take(3) {
        println!("  - {:?}", job);
    }

    let job = api
        .run_job(
            &RunJobParams::builder()
                .image("ubuntu:latest")
                .command(vec!["echo".to_string(), "Hello from Rust!".to_string()])
                .flavor("cpu-basic")
                .build(),
        )
        .await?;
    let job_id = &job.id;
    println!("\nStarted job: {job_id}");

    let inspected = api.inspect_job(job_id, None).await?;
    println!("Job status: {:?}", inspected.status);

    let cancelled = api.cancel_job(job_id, None).await?;
    println!("Cancelled job: {:?}", cancelled.status);

    let scheduled = api
        .create_scheduled_job(
            &CreateScheduledJobParams::builder()
                .image("ubuntu:latest")
                .command(vec!["echo".to_string(), "scheduled".to_string()])
                .schedule("0 0 * * *")
                .flavor("cpu-basic")
                .build(),
        )
        .await?;
    let sched_id = &scheduled.id;
    println!("\nCreated scheduled job: {sched_id}");

    let all_scheduled = api.list_scheduled_jobs(None).await?;
    println!("Scheduled jobs: {}", all_scheduled.len());

    let inspected_sched = api.inspect_scheduled_job(sched_id, None).await?;
    println!("Scheduled job: {:?}", inspected_sched);

    api.delete_scheduled_job(sched_id, None).await?;
    println!("Deleted scheduled job");

    Ok(())
}
