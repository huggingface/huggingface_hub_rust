use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessRequest {
    pub username: Option<String>,
    pub fullname: Option<String>,
    pub email: Option<String>,
    pub status: Option<String>,
    pub timestamp: Option<String>,
    pub fields: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_access_request_deserialize() {
        let json = r#"{
            "username": "user1",
            "email": "user1@example.com",
            "status": "pending",
            "timestamp": "2024-01-01T00:00:00Z"
        }"#;
        let req: AccessRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.username.as_deref(), Some("user1"));
        assert_eq!(req.status.as_deref(), Some("pending"));
    }

    #[test]
    fn test_access_request_with_fields() {
        let json = r#"{
            "username": "user2",
            "status": "accepted",
            "fields": {"reason": "research"}
        }"#;
        let req: AccessRequest = serde_json::from_str(json).unwrap();
        assert!(req.fields.is_some());
    }
}
