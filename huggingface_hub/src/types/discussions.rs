use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscussionsResponse {
    #[serde(default)]
    pub discussions: Vec<Discussion>,
    pub count: Option<u64>,
    pub start: Option<u64>,
    pub num_closed_discussions: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscussionAuthor {
    pub name: String,
    #[serde(rename = "_id")]
    pub id: Option<String>,
    pub avatar_url: Option<String>,
    pub fullname: Option<String>,
    #[serde(rename = "type")]
    pub author_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Discussion {
    pub num: u64,
    pub author: DiscussionAuthor,
    pub title: String,
    pub status: String,
    pub is_pull_request: bool,
    pub created_at: String,
    pub repo: Option<serde_json::Value>,
    #[serde(default)]
    pub pinned: bool,
    pub num_comments: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscussionChanges {
    pub base: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscussionWithDetails {
    pub num: u64,
    pub author: DiscussionAuthor,
    pub title: String,
    pub status: String,
    pub is_pull_request: bool,
    pub created_at: String,
    #[serde(default)]
    pub events: Vec<DiscussionEvent>,
    #[serde(default)]
    pub files_with_conflicts: Vec<String>,
    pub changes: Option<DiscussionChanges>,
    pub merge_commit_oid: Option<String>,
    pub diff_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscussionEvent {
    pub id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub author: DiscussionAuthor,
    pub created_at: Option<String>,
    #[serde(default)]
    pub data: serde_json::Value,
}

/// Response from creating a discussion or pull request.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscussionCreated {
    pub num: u64,
    pub url: Option<String>,
    pub pull_request: Option<bool>,
}

/// Response from adding a comment to a discussion.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscussionCommentResponse {
    pub new_message: DiscussionEvent,
}

/// Response from changing discussion status (close/reopen).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscussionStatusResponse {
    pub new_status: DiscussionEvent,
}

/// Response from renaming a discussion.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscussionTitleResponse {
    pub new_title: DiscussionEvent,
}

/// Response from merging a pull request.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscussionMergeResponse {
    pub new_status: DiscussionEvent,
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
            "isPullRequest": true,
            "createdAt": "2024-01-01T00:00:00.000Z"
        }"#;
        let disc: Discussion = serde_json::from_str(json).unwrap();
        assert_eq!(disc.num, 5);
        assert!(disc.is_pull_request);
        assert_eq!(disc.author.name, "user1");
    }

    #[test]
    fn test_discussions_response_deserialize() {
        let json = r#"{
            "discussions": [
                {"num": 1, "title": "Test", "status": "open", "isPullRequest": false,
                 "author": {"name": "user1"}, "createdAt": "2024-01-01T00:00:00.000Z"}
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
            "author": {"name": "user1"},
            "createdAt": "2024-01-01T00:00:00.000Z",
            "events": [{"id": "abc", "type": "comment", "author": {"name": "user1"}}],
            "filesWithConflicts": [],
            "changes": {"base": "refs/heads/main"}
        }"#;
        let disc: DiscussionWithDetails = serde_json::from_str(json).unwrap();
        assert_eq!(disc.num, 3);
        assert_eq!(disc.events.len(), 1);
        assert_eq!(disc.changes.unwrap().base, Some("refs/heads/main".to_string()));
    }
}
