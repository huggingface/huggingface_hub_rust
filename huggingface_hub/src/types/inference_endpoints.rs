use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InferenceEndpointInfo {
    pub name: String,
    pub namespace: Option<String>,
    #[serde(default)]
    pub status: InferenceEndpointStatus,
    pub url: Option<String>,
    pub model: Option<InferenceEndpointModel>,
    pub provider: Option<InferenceEndpointProvider>,
    #[serde(rename = "type")]
    pub endpoint_type: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    #[serde(default)]
    pub raw: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InferenceEndpointModel {
    pub repository: Option<String>,
    pub framework: Option<String>,
    pub revision: Option<String>,
    pub task: Option<String>,
    pub image: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InferenceEndpointProvider {
    pub vendor: Option<String>,
    pub region: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct InferenceEndpointStatus {
    pub state: Option<String>,
    pub message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inference_endpoint_info_deserialize() {
        let json = r#"{
            "name": "my-endpoint",
            "status": {"state": "running"},
            "url": "https://my-endpoint.endpoints.huggingface.cloud",
            "type": "protected"
        }"#;
        let info: InferenceEndpointInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.name, "my-endpoint");
        assert_eq!(info.status.state.as_deref(), Some("running"));
        assert_eq!(info.endpoint_type.as_deref(), Some("protected"));
    }

    #[test]
    fn test_inference_endpoint_info_minimal() {
        let json = r#"{"name":"test","status":{}}"#;
        let info: InferenceEndpointInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.name, "test");
        assert!(info.url.is_none());
    }
}
