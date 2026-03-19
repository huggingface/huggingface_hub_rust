//! Collection operations: list, create, manage items, and delete.
//!
//! Requires HF_TOKEN and the "collections" feature.
//! Run: cargo run -p huggingface-hub --features collections --example collections

use huggingface_hub::{
    AddCollectionItemParams, CreateCollectionParams, DeleteCollectionItemParams,
    DeleteCollectionParams, GetCollectionParams, HfApi, ListCollectionsParams,
    UpdateCollectionItemParams, UpdateCollectionMetadataParams,
};

#[tokio::main]
async fn main() -> huggingface_hub::Result<()> {
    let api = HfApi::new()?;

    // --- Read operations ---

    let collections = api
        .list_collections(
            &ListCollectionsParams::builder()
                .owner("huggingface")
                .limit(3_usize)
                .build(),
        )
        .await?;
    println!("Collections by huggingface:");
    for c in &collections {
        println!("  - {} ({:?})", c.slug, c.title);
    }

    if let Some(first) = collections.first() {
        let detail = api
            .get_collection(&GetCollectionParams::builder().slug(&first.slug).build())
            .await?;
        println!(
            "\nCollection detail: {} ({} items)",
            detail.slug,
            detail.items.len()
        );
    }

    // --- Write operations ---

    let collection = api
        .create_collection(
            &CreateCollectionParams::builder()
                .title("Example Collection")
                .description("Created by huggingface-hub Rust example")
                .private(true)
                .build(),
        )
        .await?;
    println!("\nCreated collection: {}", collection.slug);

    let updated = api
        .update_collection_metadata(
            &UpdateCollectionMetadataParams::builder()
                .slug(&collection.slug)
                .description("Updated description")
                .build(),
        )
        .await?;
    println!("Updated collection: {:?}", updated.description);

    let with_item = api
        .add_collection_item(
            &AddCollectionItemParams::builder()
                .slug(&collection.slug)
                .item_id("gpt2")
                .item_type("model")
                .note("Classic GPT-2 model")
                .build(),
        )
        .await?;
    let item_id = with_item
        .items
        .last()
        .and_then(|i| i.item_object_id.clone())
        .expect("item should have an id");
    println!("Added item to collection (id: {item_id})");

    let updated_item = api
        .update_collection_item(
            &UpdateCollectionItemParams::builder()
                .slug(&collection.slug)
                .item_object_id(&item_id)
                .note("Updated note for GPT-2")
                .build(),
        )
        .await?;
    println!("Updated item note: {:?}", updated_item.note);

    api.delete_collection_item(
        &DeleteCollectionItemParams::builder()
            .slug(&collection.slug)
            .item_object_id(&item_id)
            .build(),
    )
    .await?;
    println!("Deleted item from collection");

    api.delete_collection(
        &DeleteCollectionParams::builder()
            .slug(&collection.slug)
            .build(),
    )
    .await?;
    println!("Deleted collection");

    Ok(())
}
