use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub slug: String,
    pub title: Option<String>,
    pub owner: Option<serde_json::Value>,
    #[serde(default)]
    pub items: Vec<CollectionItem>,
    pub last_updated: Option<String>,
    pub position: Option<i64>,
    pub private: Option<bool>,
    pub gating: Option<bool>,
    pub theme: Option<String>,
    pub upvotes: Option<u64>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionItem {
    #[serde(rename = "_id")]
    pub item_object_id: Option<String>,
    #[serde(rename = "type")]
    pub item_type: Option<String>,
    pub author: Option<String>,
    pub author_data: Option<serde_json::Value>,
    pub position: Option<i64>,
    pub note: Option<String>,
    pub id: Option<String>,
    pub downloads: Option<u64>,
    pub likes: Option<u64>,
    pub gated: Option<serde_json::Value>,
    #[serde(rename = "private")]
    pub is_private: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collection_deserialize() {
        let json = r#"{
            "slug": "user/my-collection-abc123",
            "title": "My Collection",
            "owner": {"name": "user", "type": "org"},
            "items": [],
            "private": false,
            "upvotes": 5
        }"#;
        let coll: Collection = serde_json::from_str(json).unwrap();
        assert_eq!(coll.slug, "user/my-collection-abc123");
        assert_eq!(coll.title.as_deref(), Some("My Collection"));
        assert_eq!(coll.upvotes, Some(5));
    }

    #[test]
    fn test_collection_item_deserialize() {
        let json = r#"{
            "_id": "item123",
            "type": "model",
            "author": "distilbert",
            "position": 0,
            "downloads": 100
        }"#;
        let item: CollectionItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.item_object_id.as_deref(), Some("item123"));
        assert_eq!(item.item_type.as_deref(), Some("model"));
        assert_eq!(item.author.as_deref(), Some("distilbert"));
    }
}
