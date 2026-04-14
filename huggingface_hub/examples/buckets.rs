//! Bucket workflows: creating a bucket, listing buckets in a namespace,
//! and fetching bucket info via an HFBucket handle.
//!
//! Requires a valid HF_TOKEN with write access.
//! Run: HF_TOKEN=hf_xxx cargo run -p huggingface-hub --example buckets

use futures::TryStreamExt;
use huggingface_hub::{BucketInfo, CreateBucketParams, HFClient, ListBucketTreeParams};

#[tokio::main]
async fn main() -> huggingface_hub::Result<()> {
    let client = HFClient::new()?;

    let whoami = client.whoami().await?;
    let namespace = &whoami.username;
    let bucket_name = "example-bucket";

    let created = client
        .create_bucket(
            &CreateBucketParams::builder()
                .namespace(namespace)
                .name(bucket_name)
                .private(true)
                .exist_ok(true)
                .build(),
        )
        .await?;
    println!("Bucket URL: {}", created.url);

    let bucket = client.bucket(namespace, bucket_name);
    println!("Bucket handle: owner={}, name={}", bucket.owner(), bucket.name());

    let info = bucket.info().await?;
    println!(
        "Bucket info: id={}, private={}, files={}, size={}",
        info.id, info.private, info.total_files, info.size
    );

    let entries: Vec<_> = bucket.list_tree(&ListBucketTreeParams::default())?.try_collect().await?;
    println!("Files in bucket: {}", entries.len());

    let buckets: Vec<BucketInfo> = client.list_buckets(namespace)?.try_collect().await?;
    println!("Buckets in {namespace}: {}", buckets.len());
    for b in &buckets {
        println!("  {}", b.id);
    }

    Ok(())
}
