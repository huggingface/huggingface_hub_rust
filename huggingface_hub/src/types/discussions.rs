use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscussionsResponse {
    #[serde(default)]
    pub discussions: Vec<Discussion>,
    pub count: Option<u64>,
    pub start: Option<u64>,
    pub num_closed_discussions: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Discussion {
    pub num: u64,
    pub author: Option<serde_json::Value>,
    pub title: Option<String>,
    pub status: Option<String>,
    pub is_pull_request: Option<bool>,
    pub created_at: Option<String>,
    pub repo: Option<serde_json::Value>,
    pub pinned: Option<bool>,
    pub num_comments: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscussionWithDetails {
    pub num: u64,
    pub author: Option<serde_json::Value>,
    pub title: Option<String>,
    pub status: Option<String>,
    pub is_pull_request: Option<bool>,
    pub created_at: Option<String>,
    #[serde(default)]
    pub events: Vec<DiscussionEvent>,
    #[serde(default)]
    pub conflicting_files: Vec<String>,
    pub target_branch: Option<String>,
    pub merge_commit_oid: Option<String>,
    pub diff: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscussionEvent {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub event_type: Option<String>,
    pub author: Option<serde_json::Value>,
    pub content: Option<String>,
    pub created_at: Option<String>,
    pub edited: Option<bool>,
    pub hidden: Option<bool>,
    #[serde(default)]
    pub raw: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscussionComment {
    pub id: Option<String>,
    pub author: Option<serde_json::Value>,
    pub content: Option<String>,
    pub created_at: Option<String>,
    pub edited: Option<bool>,
    pub hidden: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discussion_deserialize() {
        let json = r#"{
            "num": 5,
            "author": {"name": "user1", "_id": "abc"},
            "title": "Fix typo",
            "status": "open",
            "isPullRequest": true
        }"#;
        let disc: Discussion = serde_json::from_str(json).unwrap();
        assert_eq!(disc.num, 5);
        assert_eq!(disc.is_pull_request, Some(true));
    }

    #[test]
    fn test_discussions_response_deserialize() {
        let json = r#"{
            "discussions": [
                {"num": 1, "title": "Test", "status": "open"}
            ],
            "count": 1
        }"#;
        let resp: DiscussionsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.discussions.len(), 1);
        assert_eq!(resp.discussions[0].num, 1);
    }

    #[test]
    fn test_discussion_with_details_deserialize() {
        let json = r#"{
            "num": 3,
            "title": "Bug report",
            "status": "open",
            "isPullRequest": false,
            "events": [{"id": "abc", "type": "comment", "content": "hello"}],
            "conflictingFiles": [],
            "targetBranch": "refs/heads/main"
        }"#;
        let disc: DiscussionWithDetails = serde_json::from_str(json).unwrap();
        assert_eq!(disc.num, 3);
        assert_eq!(disc.events.len(), 1);
    }
}
