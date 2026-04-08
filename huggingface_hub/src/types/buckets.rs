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
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "casUrl")]
    pub cas_url: String,
    /// Epoch time (s)
    #[serde(rename = "exp")]
    pub expires_at: u64,
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
    /// ISO 8601 Datetime
    #[serde(rename = "uploadedAt")]
    pub uploaded_at: String,
    /// ISO 8601 Datetime
    pub mtime: Option<String>,
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
