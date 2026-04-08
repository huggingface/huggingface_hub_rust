use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

// --- Parameter types ---

#[derive(Debug, Clone, TypedBuilder, Serialize)]
pub struct CreateBucketParams {
    #[builder(default, setter(strip_option))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private: Option<bool>,
    #[builder(default, setter(strip_option, into))]
    #[serde(rename = "resourceGroupId", skip_serializing_if = "Option::is_none")]
    pub resource_group_id: Option<String>,
    #[builder(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cdn: Vec<CdnRegion>,
}

#[derive(Debug, Clone, TypedBuilder, Serialize)]
pub struct UpdateBucketParams {
    #[builder(default, setter(strip_option))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private: Option<bool>,
    #[builder(default, setter(strip_option))]
    #[serde(rename = "cdnRegions", skip_serializing_if = "Option::is_none")]
    pub cdn_regions: Option<Vec<CdnRegion>>,
}

#[derive(Debug, Clone, TypedBuilder)]
pub struct ListTreeParams {
    #[builder(default, setter(strip_option))]
    pub limit: Option<u32>,
    #[builder(default)]
    pub recursive: bool,
}

// --- Response types ---

#[derive(Debug, Clone, Deserialize)]
pub struct BucketCreated {
    pub url: String,
    pub name: String,
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BucketInfo {
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub private: bool,
    #[serde(rename = "usedStorage")]
    pub used_storage: u64,
    #[serde(rename = "totalFiles")]
    pub total_files: u64,
    pub cdn: Vec<CdnRegion>,
    pub region: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdnRegion {
    pub provider: String,
    pub region: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BucketOverview {
    #[serde(rename = "_id")]
    pub mongo_id: String,
    pub id: String,
    pub author: String,
    pub private: Option<bool>,
    #[serde(rename = "repoType")]
    pub repo_type: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    pub size: u64,
    #[serde(rename = "totalFiles")]
    pub total_files: u64,
    #[serde(rename = "cdnRegions")]
    pub cdn_regions: Vec<CdnRegion>,
    #[serde(rename = "resourceGroup")]
    pub resource_group: Option<ResourceGroup>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResourceGroup {
    pub id: String,
    pub name: String,
    #[serde(rename = "numUsers")]
    pub num_users: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct XetToken {
    pub token: String,
    #[serde(rename = "casUrl")]
    pub cas_url: String,
    #[serde(rename = "expiresAt")]
    pub expires_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PathInfo {
    pub path: String,
    pub size: u64,
    #[serde(rename = "xetHash")]
    pub xet_hash: String,
    #[serde(rename = "contentType")]
    pub content_type: String,
    pub mtime: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TreeEntry {
    #[serde(rename = "type")]
    pub entry_type: EntryType,
    pub path: String,
    pub size: Option<u64>,
    #[serde(rename = "xetHash")]
    pub xet_hash: Option<String>,
    #[serde(rename = "contentType")]
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryType {
    File,
    Directory,
}

// --- Batch types ---

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum BatchOp {
    #[serde(rename = "addFile")]
    AddFile(AddFileOp),
    #[serde(rename = "deleteFile")]
    DeleteFile(DeleteFileOp),
}

#[derive(Debug, Clone, Serialize)]
pub struct AddFileOp {
    pub path: String,
    #[serde(rename = "xetHash")]
    pub xet_hash: String,
    #[serde(rename = "contentType")]
    pub content_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mtime: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeleteFileOp {
    pub path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BatchResult {
    pub success: bool,
    pub processed: u32,
    pub succeeded: u32,
    pub failed: Vec<BatchFailure>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BatchFailure {
    pub path: String,
    pub error: String,
}

// --- resolve_file types ---

#[derive(Debug, Clone)]
pub struct ResolvedFile {
    pub url: String,
    pub size: Option<u64>,
    pub xet_hash: Option<String>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub xet_auth_url: Option<String>,
    pub xet_reconstruction_url: Option<String>,
}

// --- xet_resolve_file type (feature = "xet") ---

#[cfg(feature = "xet")]
#[derive(Debug, Clone, Deserialize)]
pub struct XetFileInfo {
    pub hash: String,
    #[serde(rename = "refreshUrl")]
    pub refresh_url: String,
    #[serde(rename = "reconstructionUrl")]
    pub reconstruction_url: String,
    pub etag: String,
    pub size: u64,
    #[serde(rename = "contentType")]
    pub content_type: String,
}

// --- Internal pagination helper (not public) ---

#[derive(Deserialize)]
pub(crate) struct TreePage {
    pub entries: Vec<TreeEntry>,
    #[serde(rename = "nextCursor")]
    pub next_cursor: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bucket_info_deserializes() {
        let json = r#"{
            "id": "my-bucket",
            "name": "my-bucket",
            "namespace": "myuser",
            "private": false,
            "usedStorage": 1024,
            "totalFiles": 3,
            "cdn": [],
            "region": "us-east-1"
        }"#;
        let info: BucketInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.used_storage, 1024);
        assert_eq!(info.total_files, 3);
    }

    #[test]
    fn bucket_overview_deserializes() {
        let json = r#"{
            "_id": "66079f1a2e4b3c001a2b3c4d",
            "id": "myuser/my-bucket",
            "author": "myuser",
            "private": false,
            "repoType": "bucket",
            "createdAt": "2024-03-30T12:00:00.000Z",
            "updatedAt": "2024-03-31T08:30:00.000Z",
            "size": 104857600,
            "totalFiles": 42,
            "cdnRegions": [{"provider": "gcp", "region": "us"}],
            "resourceGroup": {"id": "abc", "name": "ml-team", "numUsers": 5}
        }"#;
        let overview: BucketOverview = serde_json::from_str(json).unwrap();
        assert_eq!(overview.id, "myuser/my-bucket");
        assert_eq!(overview.total_files, 42);
        assert_eq!(overview.resource_group.unwrap().name, "ml-team");
    }

    #[test]
    fn batch_op_serializes_with_type_tag() {
        let op = BatchOp::AddFile(AddFileOp {
            path: "data/train.parquet".to_string(),
            xet_hash: "abc123".to_string(),
            content_type: "application/octet-stream".to_string(),
            mtime: Some(1711900000),
        });
        let s = serde_json::to_string(&op).unwrap();
        assert!(s.contains(r#""type":"addFile""#));
        assert!(s.contains(r#""xetHash":"abc123""#));
    }

    #[test]
    fn delete_op_serializes_with_type_tag() {
        let op = BatchOp::DeleteFile(DeleteFileOp {
            path: "old.parquet".to_string(),
        });
        let s = serde_json::to_string(&op).unwrap();
        assert!(s.contains(r#""type":"deleteFile""#));
    }

    #[test]
    fn tree_entry_deserializes_file() {
        let json = r#"{
            "type": "file",
            "path": "data/train.parquet",
            "size": 52428800,
            "xetHash": "abc123",
            "contentType": "application/octet-stream"
        }"#;
        let entry: TreeEntry = serde_json::from_str(json).unwrap();
        assert!(matches!(entry.entry_type, EntryType::File));
        assert_eq!(entry.xet_hash.unwrap(), "abc123");
    }
}
