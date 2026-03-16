use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LikedRepo {
    pub repo: Option<LikedRepoRef>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LikedRepoRef {
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub repo_type: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_liked_repo_deserialize() {
        let json = r#"{
            "createdAt": "2024-01-01T00:00:00.000Z",
            "repo": {"name": "openai-community/gpt2", "type": "model"}
        }"#;
        let liked: LikedRepo = serde_json::from_str(json).unwrap();
        let repo = liked.repo.unwrap();
        assert_eq!(repo.name.as_deref(), Some("openai-community/gpt2"));
        assert_eq!(repo.repo_type.as_deref(), Some("model"));
    }

    #[test]
    fn test_liked_repo_minimal() {
        let json = r#"{"repo": {"name": "test/repo", "type": "dataset"}}"#;
        let liked: LikedRepo = serde_json::from_str(json).unwrap();
        assert!(liked.created_at.is_none());
        assert_eq!(liked.repo.unwrap().repo_type.as_deref(), Some("dataset"));
    }
}
