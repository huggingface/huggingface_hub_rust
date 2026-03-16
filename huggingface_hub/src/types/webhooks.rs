use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookInfo {
    pub id: Option<String>,
    pub url: Option<String>,
    #[serde(default)]
    pub watched: Vec<WebhookWatchedItem>,
    #[serde(default)]
    pub domains: Vec<String>,
    pub secret: Option<String>,
    pub disabled: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookWatchedItem {
    #[serde(rename = "type")]
    pub item_type: Option<String>,
    pub name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_info_deserialize() {
        let json = r#"{
            "id": "wh-abc123",
            "url": "https://example.com/hook",
            "watched": [{"type": "user", "name": "john"}],
            "domains": ["repo"],
            "disabled": false
        }"#;
        let wh: WebhookInfo = serde_json::from_str(json).unwrap();
        assert_eq!(wh.id.as_deref(), Some("wh-abc123"));
        assert_eq!(wh.watched.len(), 1);
    }

    #[test]
    fn test_webhook_info_deserialize_with_string_disabled() {
        let json = r#"{
            "id": "wh-abc123",
            "url": "https://example.com/hook",
            "watched": [],
            "domains": [],
            "disabled": "suspended-after-failure"
        }"#;
        let wh: WebhookInfo = serde_json::from_str(json).unwrap();
        assert!(wh.disabled.is_some());
    }

    #[test]
    fn test_webhook_watched_item_deserialize() {
        let json = r#"{"type": "org", "name": "HuggingFace"}"#;
        let item: WebhookWatchedItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.item_type.as_deref(), Some("org"));
        assert_eq!(item.name.as_deref(), Some("HuggingFace"));
    }
}
