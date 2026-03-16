use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperInfo {
    pub id: String,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub authors: Option<Vec<PaperAuthor>>,
    pub published_at: Option<String>,
    pub upvotes: Option<u64>,
    pub num_comments: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperAuthor {
    pub name: Option<String>,
    #[serde(rename = "_id")]
    pub id: Option<String>,
    pub user: Option<serde_json::Value>,
    pub hidden: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperSearchResult {
    pub paper: Option<PaperInfo>,
    pub published_at: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyPaper {
    pub paper: Option<PaperInfo>,
    pub published_at: Option<String>,
    pub title: Option<String>,
    pub submitter: Option<serde_json::Value>,
    pub upvotes: Option<u64>,
    pub num_comments: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paper_info_deserialize() {
        let json = r#"{
            "id": "2307.09288",
            "title": "Llama 2",
            "upvotes": 100,
            "authors": [{"name": "Author One", "_id": "abc", "hidden": false}]
        }"#;
        let paper: PaperInfo = serde_json::from_str(json).unwrap();
        assert_eq!(paper.id, "2307.09288");
        assert_eq!(paper.upvotes, Some(100));
        assert_eq!(paper.authors.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_daily_paper_deserialize() {
        let json = r#"{
            "paper": {"id": "2307.09288", "title": "Test"},
            "publishedAt": "2024-01-01",
            "submitter": {"name": "user1"},
            "upvotes": 5
        }"#;
        let dp: DailyPaper = serde_json::from_str(json).unwrap();
        assert_eq!(dp.paper.as_ref().unwrap().id, "2307.09288");
        assert_eq!(dp.upvotes, Some(5));
    }

    #[test]
    fn test_paper_author_with_user_object() {
        let json = r#"{"name": "John", "_id": "abc123", "user": {"name": "johndoe", "type": "user"}, "hidden": false}"#;
        let author: PaperAuthor = serde_json::from_str(json).unwrap();
        assert_eq!(author.name.as_deref(), Some("John"));
        assert_eq!(author.id.as_deref(), Some("abc123"));
        assert!(author.user.is_some());
    }

    #[test]
    fn test_paper_search_result_deserialize() {
        let json = r#"{
            "paper": {"id": "1706.03762", "title": "Attention Is All You Need"},
            "publishedAt": "2017-06-12",
            "title": "Attention Is All You Need"
        }"#;
        let result: PaperSearchResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.paper.as_ref().unwrap().id, "1706.03762");
    }
}
