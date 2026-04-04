use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

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

#[derive(Debug, Clone)]
pub struct DiscussionEvent {
    pub id: String,
    pub event_type: String,
    pub author: DiscussionAuthor,
    pub created_at: Option<String>,
    pub data: DiscussionEventData,
}

impl<'de> Deserialize<'de> for DiscussionEvent {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DiscussionEventVisitor;

        impl<'de> Visitor<'de> for DiscussionEventVisitor {
            type Value = DiscussionEvent;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a discussion event object")
            }

            fn visit_map<M>(self, mut map: M) -> std::result::Result<DiscussionEvent, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut id: Option<String> = None;
                let mut event_type: Option<String> = None;
                let mut author: Option<DiscussionAuthor> = None;
                let mut created_at: Option<String> = None;
                let mut raw_data: Option<serde_json::Value> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "id" => id = Some(map.next_value()?),
                        "type" => event_type = Some(map.next_value()?),
                        "author" => author = Some(map.next_value()?),
                        "createdAt" => created_at = map.next_value()?,
                        "data" => raw_data = Some(map.next_value()?),
                        _ => {
                            let _ = map.next_value::<serde_json::Value>()?;
                        },
                    }
                }

                let id = id.ok_or_else(|| de::Error::missing_field("id"))?;
                let event_type = event_type.ok_or_else(|| de::Error::missing_field("type"))?;
                let author = author.ok_or_else(|| de::Error::missing_field("author"))?;
                let raw_data = raw_data.unwrap_or(serde_json::Value::Null);

                let data = if raw_data.is_null() || raw_data.as_object().is_some_and(|o| o.is_empty()) {
                    match event_type.as_str() {
                        "comment" => DiscussionEventData::Comment(CommentData::default()),
                        "status-change" | "title-change" | "commit" => DiscussionEventData::Unknown(raw_data),
                        _ => DiscussionEventData::Unknown(raw_data),
                    }
                } else {
                    match event_type.as_str() {
                        "comment" => serde_json::from_value::<CommentData>(raw_data.clone())
                            .map(DiscussionEventData::Comment)
                            .unwrap_or(DiscussionEventData::Unknown(raw_data)),
                        "status-change" => serde_json::from_value::<StatusChangeData>(raw_data.clone())
                            .map(DiscussionEventData::StatusChange)
                            .unwrap_or(DiscussionEventData::Unknown(raw_data)),
                        "title-change" => serde_json::from_value::<TitleChangeData>(raw_data.clone())
                            .map(DiscussionEventData::TitleChange)
                            .unwrap_or(DiscussionEventData::Unknown(raw_data)),
                        "commit" => serde_json::from_value::<CommitData>(raw_data.clone())
                            .map(DiscussionEventData::Commit)
                            .unwrap_or(DiscussionEventData::Unknown(raw_data)),
                        _ => DiscussionEventData::Unknown(raw_data),
                    }
                };

                Ok(DiscussionEvent {
                    id,
                    event_type,
                    author,
                    created_at,
                    data,
                })
            }
        }

        deserializer.deserialize_map(DiscussionEventVisitor)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentLatest {
    pub raw: Option<String>,
    pub html: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentReaction {
    pub reaction: String,
    #[serde(default)]
    pub users: Vec<String>,
    pub count: Option<u64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentData {
    #[serde(default)]
    pub edited: bool,
    #[serde(default)]
    pub hidden: bool,
    pub hidden_by: Option<String>,
    pub latest: Option<CommentLatest>,
    pub num_edits: Option<u64>,
    #[serde(default)]
    pub editors: Vec<String>,
    #[serde(default)]
    pub reactions: Vec<CommentReaction>,
    pub is_report: Option<bool>,
    pub related_event_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusChangeData {
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TitleChangeData {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitData {
    pub subject: Option<String>,
    pub oid: Option<String>,
    #[serde(default)]
    pub parents: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum DiscussionEventData {
    Comment(CommentData),
    StatusChange(StatusChangeData),
    TitleChange(TitleChangeData),
    Commit(CommitData),
    Unknown(serde_json::Value),
}

impl Default for DiscussionEventData {
    fn default() -> Self {
        Self::Unknown(serde_json::Value::Null)
    }
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
    fn test_comment_event_no_data() {
        let json = r#"{"id": "abc", "type": "comment", "author": {"name": "user1"}}"#;
        let event: DiscussionEvent = serde_json::from_str(json).unwrap();
        assert!(matches!(event.data, DiscussionEventData::Comment(_)));
    }

    #[test]
    fn test_unknown_event_type() {
        let json = r#"{"id": "a", "type": "new-future-event", "author": {"name": "u"}, "data": {"x": 1}}"#;
        let event: DiscussionEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.event_type, "new-future-event");
        assert!(matches!(event.data, DiscussionEventData::Unknown(_)));
    }

    #[test]
    fn test_title_change_event() {
        let json = r#"{
            "id": "a1", "type": "title-change", "author": {"name": "u"},
            "createdAt": "2024-01-01T00:00:00.000Z",
            "data": {"from": "Old Title", "to": "New Title"}
        }"#;
        let event: DiscussionEvent = serde_json::from_str(json).unwrap();
        match &event.data {
            DiscussionEventData::TitleChange(d) => {
                assert_eq!(d.from, "Old Title");
                assert_eq!(d.to, "New Title");
            },
            _ => panic!("expected TitleChange"),
        }
    }

    #[test]
    fn test_real_discussions_list_response() {
        let json = r#"{
            "discussions": [{
                "num": 152,
                "author": {
                    "_id": "67c82e44cae39495ef71d325",
                    "avatarUrl": "/avatars/1cdc535dbff676a4b5069965399d4388.svg",
                    "fullname": "sarthak saxena",
                    "name": "sarthak-saxena",
                    "type": "user",
                    "isPro": false,
                    "isHf": false,
                    "isHfAdmin": false,
                    "isMod": false,
                    "followerCount": 0,
                    "isUserFollowing": false
                },
                "repo": {"name": "openai-community/gpt2", "type": "model"},
                "title": "Install & run openai-community/gpt2 easily using llmpm",
                "status": "open",
                "createdAt": "2026-03-11T15:34:52.000Z",
                "isPullRequest": false,
                "numComments": 1,
                "topReactions": [],
                "numReactionUsers": 0,
                "pinned": false,
                "repoOwner": {
                    "name": "openai-community",
                    "isParticipating": false,
                    "type": "org",
                    "isDiscussionAuthor": false
                }
            }],
            "count": 145,
            "start": 0,
            "numClosedDiscussions": 60
        }"#;
        let resp: DiscussionsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.count, Some(145));
        assert_eq!(resp.start, Some(0));
        assert_eq!(resp.num_closed_discussions, Some(60));
        assert_eq!(resp.discussions.len(), 1);
        let disc = &resp.discussions[0];
        assert_eq!(disc.num, 152);
        assert_eq!(disc.author.name, "sarthak-saxena");
        assert_eq!(disc.author.id.as_deref(), Some("67c82e44cae39495ef71d325"));
        assert_eq!(disc.author.fullname.as_deref(), Some("sarthak saxena"));
        assert_eq!(disc.author.author_type.as_deref(), Some("user"));
        assert_eq!(disc.title, "Install & run openai-community/gpt2 easily using llmpm");
        assert_eq!(disc.status, "open");
        assert!(!disc.is_pull_request);
        assert!(!disc.pinned);
        assert_eq!(disc.num_comments, Some(1));
    }

    #[test]
    fn test_real_discussion_with_details() {
        let json = r#"{
            "num": 1,
            "author": {
                "_id": "627525b258ea49d110628e0e",
                "avatarUrl": "/avatars/73b216ad6afaa5bb6e3b477370bfadcc.svg",
                "fullname": "kk",
                "name": "mastermile",
                "type": "user",
                "isPro": false,
                "isHf": false,
                "isHfAdmin": false,
                "isMod": false,
                "isUserFollowing": false
            },
            "org": {
                "avatarUrl": "https://cdn-avatars.huggingface.co/v1/production/uploads/example.png",
                "fullname": "OpenAI community",
                "name": "openai-community",
                "type": "org",
                "isHf": false
            },
            "repo": {"name": "openai-community/gpt2", "type": "model"},
            "title": "\u5bf9",
            "status": "closed",
            "createdAt": "2022-05-27T03:51:50.000Z",
            "pinned": false,
            "locked": false,
            "collection": "discussions",
            "isPullRequest": true,
            "changes": {
                "base": "refs/heads/main",
                "cachedStorageEstimate": {"size": 0, "baseSha": "607a30d783dfa663caf39e06633721c8d4cfcd7e"}
            },
            "filesWithConflicts": [],
            "diffUrl": "https://huggingface.co/openai-community/gpt2/discussions/1/files.diff",
            "events": [
                {
                    "id": "62904ad60ea7e76254c12713",
                    "author": {
                        "_id": "627525b258ea49d110628e0e",
                        "avatarUrl": "/avatars/73b216ad6afaa5bb6e3b477370bfadcc.svg",
                        "fullname": "kk",
                        "name": "mastermile",
                        "type": "user",
                        "isPro": false, "isHf": false, "isHfAdmin": false, "isMod": false,
                        "isUserFollowing": false, "isOwner": false, "isOrgMember": false
                    },
                    "createdAt": "2022-05-27T03:51:50.000Z",
                    "type": "comment",
                    "data": {
                        "edited": false,
                        "hidden": false,
                        "latest": {
                            "raw": "dd",
                            "html": "<p>dd</p>\n",
                            "updatedAt": "2022-05-27T03:51:50.000Z",
                            "author": {
                                "_id": "627525b258ea49d110628e0e",
                                "name": "mastermile", "type": "user"
                            }
                        },
                        "numEdits": 0,
                        "editors": ["mastermile"],
                        "editorAvatarUrls": ["/avatars/73b216ad6afaa5bb6e3b477370bfadcc.svg"],
                        "reactions": [],
                        "isReport": false
                    }
                },
                {
                    "id": "62904ad70000000000000000",
                    "author": {
                        "_id": "627525b258ea49d110628e0e",
                        "name": "mastermile", "type": "user"
                    },
                    "createdAt": "2022-05-27T03:51:51.000Z",
                    "type": "commit",
                    "data": {
                        "subject": "\u5bf9",
                        "oid": "81fd1d6e7847c99f5862c9fb81387956d99ec7aa",
                        "parents": ["6c0e6080953db56375760c0471a8c5f2929baf11"]
                    }
                },
                {
                    "id": "6290526809a2a46059e8e62c",
                    "author": {
                        "_id": "627525b258ea49d110628e0e",
                        "name": "mastermile", "type": "user"
                    },
                    "createdAt": "2022-05-27T04:24:08.000Z",
                    "type": "status-change",
                    "data": {"status": "closed"}
                }
            ]
        }"#;
        let disc: DiscussionWithDetails = serde_json::from_str(json).unwrap();
        assert_eq!(disc.num, 1);
        assert_eq!(disc.author.name, "mastermile");
        assert_eq!(disc.title, "\u{5bf9}");
        assert_eq!(disc.status, "closed");
        assert!(disc.is_pull_request);
        assert_eq!(disc.changes.as_ref().unwrap().base.as_deref(), Some("refs/heads/main"));
        assert!(disc.files_with_conflicts.is_empty());
        assert_eq!(
            disc.diff_url.as_deref(),
            Some("https://huggingface.co/openai-community/gpt2/discussions/1/files.diff")
        );

        assert_eq!(disc.events.len(), 3);

        let comment = &disc.events[0];
        assert_eq!(comment.id, "62904ad60ea7e76254c12713");
        assert_eq!(comment.event_type, "comment");
        assert_eq!(comment.author.name, "mastermile");
        assert_eq!(comment.created_at.as_deref(), Some("2022-05-27T03:51:50.000Z"));
        match &comment.data {
            DiscussionEventData::Comment(d) => {
                assert!(!d.edited);
                assert!(!d.hidden);
                assert_eq!(d.num_edits, Some(0));
                assert_eq!(d.editors, vec!["mastermile"]);
                assert!(d.reactions.is_empty());
                assert_eq!(d.is_report, Some(false));
                let latest = d.latest.as_ref().unwrap();
                assert_eq!(latest.raw.as_deref(), Some("dd"));
                assert_eq!(latest.html.as_deref(), Some("<p>dd</p>\n"));
            },
            other => panic!("expected Comment, got {other:?}"),
        }

        let commit = &disc.events[1];
        assert_eq!(commit.event_type, "commit");
        match &commit.data {
            DiscussionEventData::Commit(d) => {
                assert_eq!(d.subject.as_deref(), Some("\u{5bf9}"));
                assert_eq!(d.oid.as_deref(), Some("81fd1d6e7847c99f5862c9fb81387956d99ec7aa"));
                assert_eq!(d.parents, vec!["6c0e6080953db56375760c0471a8c5f2929baf11"]);
            },
            other => panic!("expected Commit, got {other:?}"),
        }

        let status = &disc.events[2];
        assert_eq!(status.event_type, "status-change");
        match &status.data {
            DiscussionEventData::StatusChange(d) => {
                assert_eq!(d.status, "closed");
            },
            other => panic!("expected StatusChange, got {other:?}"),
        }
    }

    #[test]
    fn test_real_hidden_comment_event() {
        let json = r#"{
            "id": "62cfe8a4ad741b94f5c53914",
            "author": {
                "_id": "628ba40d488ff0ea1c71d3cd",
                "avatarUrl": "/avatars/b6b99150dca9006aef54516347d06166.svg",
                "fullname": "daianbo",
                "name": "daianbo",
                "type": "user",
                "isPro": false, "isHf": false, "isHfAdmin": false, "isMod": false,
                "isUserFollowing": false, "isOwner": false, "isOrgMember": false
            },
            "createdAt": "2022-07-14T09:57:56.000Z",
            "type": "comment",
            "data": {
                "edited": true,
                "hidden": true,
                "hiddenBy": "",
                "latest": {
                    "raw": "This comment has been hidden",
                    "html": "This comment has been hidden",
                    "updatedAt": "2022-07-14T09:58:36.829Z",
                    "author": {
                        "_id": "628ba40d488ff0ea1c71d3cd",
                        "name": "daianbo", "type": "user"
                    }
                },
                "numEdits": 0,
                "editors": [],
                "editorAvatarUrls": [],
                "reactions": []
            }
        }"#;
        let event: DiscussionEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.event_type, "comment");
        assert_eq!(event.author.name, "daianbo");
        match &event.data {
            DiscussionEventData::Comment(d) => {
                assert!(d.edited);
                assert!(d.hidden);
                assert_eq!(d.hidden_by.as_deref(), Some(""));
                assert_eq!(d.latest.as_ref().unwrap().raw.as_deref(), Some("This comment has been hidden"));
            },
            other => panic!("expected Comment, got {other:?}"),
        }
    }
}
