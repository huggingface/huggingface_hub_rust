use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

// --- Parameter types ---

/// Parameters for [`HFClient::create_bucket`].
#[derive(Debug, Clone, TypedBuilder, Serialize)]
pub struct CreateBucketParams {
    /// Whether the bucket should be private. Defaults to public when omitted.
    #[builder(default, setter(strip_option))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private: Option<bool>,
    /// Resource group to assign the bucket to.
    #[builder(default, setter(strip_option, into))]
    #[serde(rename = "resourceGroupId", skip_serializing_if = "Option::is_none")]
    pub resource_group_id: Option<String>,
    /// CDN regions to enable for this bucket at creation time.
    #[builder(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cdn: Vec<CdnRegion>,
}

/// Parameters for [`HFBucket::update_settings`].
#[derive(Debug, Clone, TypedBuilder, Serialize)]
pub struct UpdateBucketParams {
    /// Change the bucket's visibility. Pass `true` to make it private, `false` for public.
    #[builder(default, setter(strip_option))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private: Option<bool>,
    /// Replace the full set of CDN regions. Pass an empty vec to remove all CDN regions.
    #[builder(default, setter(strip_option))]
    #[serde(rename = "cdnRegions", skip_serializing_if = "Option::is_none")]
    pub cdn_regions: Option<Vec<CdnRegion>>,
}

/// Parameters for [`HFBucket::list_tree`].
#[derive(Debug, Clone, TypedBuilder)]
pub struct ListTreeParams {
    /// Maximum number of entries to return per page. The server default is 1000; maximum is 10 000.
    #[builder(default, setter(strip_option))]
    pub limit: Option<u32>,
    /// When `true`, return all entries under the prefix recursively.
    /// When `false` (the default), only top-level entries are returned and sub-directories
    /// are collapsed into a single [`EntryType::Directory`] entry.
    #[builder(default)]
    pub recursive: bool,
}

// --- Response types ---

/// Returned by [`HFClient::create_bucket`] on success.
#[derive(Debug, Clone, Deserialize)]
pub struct BucketCreated {
    /// Full URL of the newly created bucket (e.g. `https://huggingface.co/buckets/my-org/my-bucket`).
    pub url: String,
    /// Bucket identifier in `namespace/name` format.
    pub name: String,
    /// Opaque server-side ID for the bucket.
    pub id: String,
}

/// A CDN region specifying a cloud provider and geographic region.
///
/// Used in [`CreateBucketParams`], [`UpdateBucketParams`], and [`BucketOverview`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdnRegion {
    /// Cloud provider (e.g. `"gcp"` or `"aws"`).
    pub provider: String,
    /// Geographic region identifier (e.g. `"us"` or `"eu"`).
    pub region: String,
}

/// Metadata about a Storage Bucket, as returned by [`HFBucket::info`] and [`HFClient::list_buckets`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketOverview {
    /// Internal MongoDB document ID.
    #[serde(rename = "_id")]
    pub mongo_id: String,
    /// Bucket identifier in `namespace/name` format.
    pub id: String,
    /// Namespace (user or organization) that owns the bucket.
    pub author: String,
    /// Whether the bucket is private. `None` means the server did not specify.
    pub private: Option<bool>,
    /// Repository type tag — always `"bucket"`.
    #[serde(rename = "repoType")]
    pub repo_type: String,
    /// ISO 8601 creation timestamp.
    #[serde(rename = "createdAt")]
    pub created_at: String,
    /// ISO 8601 last-updated timestamp.
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    /// Total storage used by the bucket, in bytes.
    pub size: u64,
    /// Number of files currently stored in the bucket.
    #[serde(rename = "totalFiles")]
    pub total_files: u64,
    /// CDN regions configured for this bucket.
    #[serde(rename = "cdnRegions")]
    pub cdn_regions: Vec<CdnRegion>,
    /// Resource group this bucket belongs to, if any.
    #[serde(rename = "resourceGroup")]
    pub resource_group: Option<ResourceGroup>,
}

/// A resource group that a bucket can be associated with.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceGroup {
    /// Opaque resource group ID.
    pub id: String,
    /// Human-readable resource group name.
    pub name: String,
    /// Number of members in the resource group, if returned by the server.
    #[serde(rename = "numUsers")]
    pub num_users: Option<u32>,
}

/// A short-lived token for authenticating directly against the Xet CAS (content-addressable storage).
///
/// Returned by [`HFBucket::get_xet_write_token`] and [`HFBucket::get_xet_read_token`].
#[derive(Debug, Clone, Deserialize)]
pub struct XetToken {
    /// Bearer token to include in requests to the Xet CAS.
    #[serde(rename = "accessToken")]
    pub access_token: String,
    /// Base URL of the Xet CAS server.
    #[serde(rename = "casUrl")]
    pub cas_url: String,
    /// Token expiry as a Unix epoch timestamp (seconds), following the standard JWT `exp` convention.
    #[serde(rename = "exp")]
    pub expires_at: u64,
}

/// A single entry returned by [`HFBucket::list_tree`] and [`HFBucket::get_paths_info`].
///
/// Can represent either a file or a directory. File-only fields (`size`, `xet_hash`,
/// `content_type`, `mtime`) are `None` for directory entries.
#[derive(Debug, Clone, Deserialize)]
pub struct TreeEntry {
    /// Whether this entry is a file or a directory.
    #[serde(rename = "type")]
    pub entry_type: EntryType,
    /// Path of the entry relative to the bucket root.
    pub path: String,
    /// ISO 8601 timestamp of when this entry was added to the bucket.
    #[serde(rename = "uploadedAt")]
    pub uploaded_at: String,
    /// Original file modification time as an ISO 8601 timestamp, if preserved at upload.
    pub mtime: Option<String>,
    /// File size in bytes. `None` for directory entries.
    pub size: Option<u64>,
    /// Content-addressable Xet hash of the file. `None` for directory entries.
    #[serde(rename = "xetHash")]
    pub xet_hash: Option<String>,
    /// MIME content type of the file. `None` for directory entries.
    #[serde(rename = "contentType")]
    pub content_type: Option<String>,
}

/// Whether a [`TreeEntry`] is a file or a directory.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryType {
    /// A regular file stored in the bucket.
    File,
    /// A virtual directory prefix (only appears when `recursive` is `false`).
    Directory,
}

// --- Batch types ---

/// A single operation in a [`HFBucket::batch_files`] call.
///
/// All [`BatchOp::AddFile`] operations must precede all [`BatchOp::DeleteFile`] operations —
/// the client enforces this automatically.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum BatchOp {
    /// Add or overwrite a file at the given path.
    ///
    /// The file contents must already be present in the Xet CAS; obtain a write token
    /// via [`HFBucket::get_xet_write_token`] before uploading.
    #[serde(rename = "addFile")]
    AddFile(AddFileOp),
    /// Remove a file from the bucket by path.
    #[serde(rename = "deleteFile")]
    DeleteFile(DeleteFileOp),
}

/// Payload for a [`BatchOp::AddFile`] operation.
#[derive(Debug, Clone, Serialize)]
pub struct AddFileOp {
    /// Destination path within the bucket.
    pub path: String,
    /// Content-addressable Xet hash of the file, obtained after uploading to the CAS.
    #[serde(rename = "xetHash")]
    pub xet_hash: String,
    /// MIME content type of the file (e.g. `"application/octet-stream"`).
    #[serde(rename = "contentType")]
    pub content_type: String,
    /// Original file modification time as a Unix timestamp in milliseconds, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mtime: Option<i64>,
}

/// Payload for a [`BatchOp::DeleteFile`] operation.
#[derive(Debug, Clone, Serialize)]
pub struct DeleteFileOp {
    /// Path of the file to remove from the bucket.
    pub path: String,
}

/// Result of a [`HFBucket::batch_files`] call.
#[derive(Debug, Clone, Deserialize)]
pub struct BatchResult {
    /// `true` if every operation in the batch succeeded.
    pub success: bool,
    /// Total number of operations attempted.
    pub processed: u32,
    /// Number of operations that completed successfully.
    pub succeeded: u32,
    /// Details of any operations that failed.
    pub failed: Vec<BatchFailure>,
}

/// A single failed operation within a [`BatchResult`].
#[derive(Debug, Clone, Deserialize)]
pub struct BatchFailure {
    /// Path of the file whose operation failed.
    pub path: String,
    /// Server-provided error message.
    pub error: String,
}

// --- resolve_file types ---

/// A resolved direct download URL for a bucket file, returned by [`HFBucket::resolve_file`].
#[derive(Debug, Clone)]
pub struct ResolvedFile {
    /// Direct download URL (the `Location` from the server's 302 redirect).
    pub url: String,
    /// File size in bytes, if provided by the server.
    pub size: Option<u64>,
    /// Content-addressable Xet hash of the file, if provided.
    pub xet_hash: Option<String>,
    /// ETag of the file, if provided.
    pub etag: Option<String>,
    /// `Last-Modified` header value, if provided.
    pub last_modified: Option<String>,
    /// URL to obtain a fresh Xet read token for this file, if provided.
    pub xet_auth_url: Option<String>,
    /// URL pointing to the Xet CAS reconstruction manifest for this file, if provided.
    pub xet_reconstruction_url: Option<String>,
}

// --- xet_resolve_file type (feature = "xet") ---

/// Xet reconstruction metadata for a bucket file, returned by [`HFBucket::xet_resolve_file`].
///
/// Only available with the `xet` feature enabled.
#[cfg(feature = "xet")]
#[derive(Debug, Clone, Deserialize)]
pub struct XetFileInfo {
    /// Content-addressable Xet hash of the file.
    pub hash: String,
    /// URL to obtain a fresh Xet read token.
    #[serde(rename = "refreshUrl")]
    pub refresh_url: String,
    /// URL pointing to the Xet CAS reconstruction manifest.
    #[serde(rename = "reconstructionUrl")]
    pub reconstruction_url: String,
    /// ETag of the file.
    pub etag: String,
    /// File size in bytes.
    pub size: u64,
    /// MIME content type of the file.
    #[serde(rename = "contentType")]
    pub content_type: String,
}
