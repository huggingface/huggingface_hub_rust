//! Paper operations: search, daily papers, and paper info.
//!
//! Requires HF_TOKEN and the "papers" feature.
//! Run: cargo run -p huggingface-hub --features papers --example papers

use huggingface_hub::{HfApi, ListDailyPapersParams, ListPapersParams, PaperInfoParams};

#[tokio::main]
async fn main() -> huggingface_hub::Result<()> {
    let api = HfApi::new()?;

    let results = api
        .list_papers(
            &ListPapersParams::builder()
                .query("transformers")
                .limit(3_usize)
                .build(),
        )
        .await?;
    println!("Paper search results for 'transformers':");
    for paper in &results {
        println!("  - {:?}", paper);
    }

    let daily = api
        .list_daily_papers(&ListDailyPapersParams::builder().limit(3_usize).build())
        .await?;
    println!("\nRecent daily papers:");
    for paper in &daily {
        println!("  - {:?}", paper);
    }

    if let Some(first) = results.first().and_then(|r| r.paper.as_ref()) {
        let info = api
            .paper_info(&PaperInfoParams::builder().paper_id(&first.id).build())
            .await?;
        println!("\nPaper info: {:?}", info);
    }

    Ok(())
}
