# HuggingFace Buckets Support — Design Spec

**Date:** 2026-04-09
**Scope:** Core CRUD, file operations, CLI (no sync)
**Approach:** Mirror HFRepository pattern with parallel HFBucket type

## Overview

Add bucket support to the Rust SDK, mirroring the Python `huggingface_hub` library's bucket API (minus sync). Buckets are xet-backed mutable object storage with no git history — fundamentally different from repositories.

The implementation adds:
- `HFBucket` handle type (parallel to `HFRepository`)
- Bucket API methods on `HFClient` (lifecycle) and `HFBucket` (scoped operations)
- `HFBucketSync` blocking wrapper
- `hfrs buckets` CLI commands matching the Python `hf buckets` interface
- New error variants for improved HTTP status handling

## Types

### New file: `src/types/buckets.rs`

```rust
/// Metadata about a bucket on the Hub.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketInfo {
    /// Full bucket identifier, e.g. "namespace/bucket_name".
    pub id: String,
    /// Whether the bucket is private.
    pub private: bool,
    /// ISO 8601 creation timestamp.
    pub created_at: String,
    /// Total size of all files in bytes.
    pub size: u64,
    /// Number of files in the bucket.
    pub total_files: u64,
}

/// URL returned after creating a bucket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketUrl {
    /// Full URL to the bucket on the Hub.
    pub url: String,
}

/// A file or directory entry in a bucket tree listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum BucketTreeEntry {
    /// A file entry with content hash and size.
    File {
        /// Path within the bucket.
        path: String,
        /// File size in bytes.
        size: u64,
        /// Xet content-addressable hash.
        #[serde(rename = "xetHash")]
        xet_hash: String,
        /// Last modification time (ISO 8601).
        mtime: Option<String>,
        /// Upload timestamp (ISO 8601).
        uploaded_at: Option<String>,
    },
    /// A directory entry.
    Directory {
        /// Directory path within the bucket.
        path: String,
        /// Upload timestamp (ISO 8601).
        uploaded_at: Option<String>,
    },
}

/// Metadata for a single file in a bucket, retrieved via HEAD request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketFileMetadata {
    /// File size in bytes.
    pub size: u64,
    /// Xet content-addressable hash.
    pub xet_hash: String,
}
```

### New file: `src/types/bucket_params.rs`

All param structs use `#[derive(TypedBuilder)]` and have doc comments.

```rust
/// Parameters for creating a new bucket.
#[derive(Debug, Clone, TypedBuilder)]
pub struct CreateBucketParams {
    /// Namespace (user or organization) that owns the bucket.
    pub namespace: String,
    /// Bucket name within the namespace.
    pub name: String,
    /// Whether the bucket should be private.
    #[builder(default = false)]
    pub private: bool,
    /// Enterprise resource group ID (optional).
    #[builder(default)]
    pub resource_group_id: Option<String>,
    /// If true, do not error when the bucket already exists.
    #[builder(default = false)]
    pub exist_ok: bool,
}

/// Parameters for listing files in a bucket tree.
#[derive(Debug, Clone, Default, TypedBuilder)]
pub struct ListBucketTreeParams {
    /// Filter results to entries under this prefix.
    #[builder(default)]
    pub prefix: Option<String>,
    /// If true, list entries recursively under the prefix.
    #[builder(default)]
    pub recursive: Option<bool>,
}

/// Parameters for batch operations on bucket files.
#[derive(Debug, Clone, Default, TypedBuilder)]
pub struct BatchBucketFilesParams {
    /// Files to add (register) in the bucket.
    #[builder(default)]
    pub add: Vec<BucketAddFile>,
    /// Paths of files to delete from the bucket.
    #[builder(default)]
    pub delete: Vec<String>,
    /// Files to copy (server-side) into the bucket.
    #[builder(default)]
    pub copy: Vec<BucketCopyFile>,
}

/// A file to register in a bucket via the batch endpoint.
#[derive(Debug, Clone)]
pub struct BucketAddFile {
    /// Destination path in the bucket.
    pub path: String,
    /// Xet content hash from a prior upload.
    pub xet_hash: String,
    /// File size in bytes.
    pub size: u64,
    /// Last modification time as a Unix timestamp (seconds).
    pub mtime: Option<u64>,
    /// MIME content type.
    pub content_type: Option<String>,
}

/// A server-side copy operation for the batch endpoint.
#[derive(Debug, Clone)]
pub struct BucketCopyFile {
    /// Destination path in the bucket.
    pub path: String,
    /// Xet content hash to copy.
    pub xet_hash: String,
    /// Source repo type (e.g. "bucket", "model").
    pub source_repo_type: String,
    /// Source repo or bucket ID.
    pub source_repo_id: String,
}

/// Parameters for downloading files from a bucket.
#[derive(Debug, Clone, TypedBuilder)]
pub struct BucketDownloadFilesParams {
    /// List of (remote_path, local_path) pairs to download.
    pub files: Vec<(String, std::path::PathBuf)>,
}
```

### Module registration

`src/types/mod.rs` gets:
```rust
pub mod buckets;
pub mod bucket_params;
pub use buckets::*;
pub use bucket_params::*;
```

## HFBucket Handle

### New file: `src/bucket.rs`

Parallel to `repository.rs`. Wraps a client reference and binds `(owner, name)`.

```rust
#[derive(Clone)]
pub struct HFBucket {
    pub(crate) hf_client: HFClient,
    owner: String,
    name: String,
}
```

**Factory method on HFClient:**
```rust
impl HFClient {
    pub fn bucket(&self, owner: impl Into<String>, name: impl Into<String>) -> HFBucket;
}
```

**Methods on HFBucket:**
- `new(client, owner, name)` — constructor
- `client()` — reference to underlying HFClient
- `owner()` — namespace accessor
- `name()` — bucket name accessor
- `bucket_id()` — returns `"owner/name"`

**URL helpers on HFClient:**
```rust
impl HFClient {
    /// {endpoint}/api/buckets/{bucket_id}
    pub(crate) fn bucket_api_url(&self, bucket_id: &str) -> String;
    /// {endpoint}/buckets/{bucket_id}/resolve/{path}
    pub(crate) fn bucket_resolve_url(&self, bucket_id: &str, path: &str) -> String;
}
```

**Exports:** `HFBucket` re-exported from `lib.rs`.

## API Methods

### New file: `src/api/buckets.rs`

#### On HFClient (bucket lifecycle)

| Method | HTTP | Endpoint |
|--------|------|----------|
| `create_bucket(&self, params: &CreateBucketParams) -> Result<BucketUrl>` | POST | `/api/buckets/{namespace}/{name}` |
| `delete_bucket(&self, bucket_id: &str, missing_ok: bool) -> Result<()>` | DELETE | `/api/buckets/{bucket_id}` |
| `list_buckets(&self, namespace: &str) -> Result<impl Stream<Item = Result<BucketInfo>>>` | GET | `/api/buckets/{namespace}` (paginated) |
| `move_bucket(&self, from_id: &str, to_id: &str) -> Result<()>` | POST | `/api/repos/move` with `type: "bucket"` |

#### On HFBucket (scoped operations)

| Method | HTTP | Endpoint |
|--------|------|----------|
| `info(&self) -> Result<BucketInfo>` | GET | `/api/buckets/{bucket_id}` |
| `list_tree(&self, params: &ListBucketTreeParams) -> Result<impl Stream<Item = Result<BucketTreeEntry>>>` | GET | `/api/buckets/{bucket_id}/tree/{prefix}` (paginated) |
| `get_paths_info(&self, paths: &[String]) -> Result<Vec<BucketTreeEntry>>` | POST | `/api/buckets/{bucket_id}/paths-info` (batched, 1000/req) |
| `get_file_metadata(&self, remote_path: &str) -> Result<BucketFileMetadata>` | HEAD | `/buckets/{bucket_id}/resolve/{path}` |
| `batch(&self, params: &BatchBucketFilesParams) -> Result<()>` | POST | `/api/buckets/{bucket_id}/batch` (NDJSON) |
| `upload_files(&self, files: &[(PathBuf, String)]) -> Result<()>` | xet upload + batch | Upload to xet, then register via batch |
| `download_files(&self, params: &BucketDownloadFilesParams) -> Result<()>` | paths-info + xet download | Resolve hashes, then download via xet |
| `delete_files(&self, paths: &[String]) -> Result<()>` | POST | Convenience wrapper around `batch` with delete-only |

### Batch endpoint NDJSON format

```jsonl
{"type": "addFile", "path": "file.txt", "xetHash": "abc...", "size": 1234, "mtime": 1234567890, "contentType": "text/plain"}
{"type": "deleteFile", "path": "old.txt"}
{"type": "copyFile", "path": "copied.txt", "xetHash": "def...", "sourceRepoType": "bucket", "sourceRepoId": "user/src-bucket"}
```

Operations are chunked at 1000 entries per request (matching Python behavior).

### Upload flow

1. Upload file contents to xet via xet session (reusing existing xet infrastructure) to get `xet_hash` per file
2. Build NDJSON payload with `addFile` entries (path, xet_hash, size, mtime, content_type)
3. `POST /api/buckets/{bucket_id}/batch`

### Download flow

1. Call `get_paths_info` to resolve xet hashes for the requested remote paths
2. Download via xet using batch download (reusing existing `xet_download_batch` pattern)

## Xet Integration

### Refactored token URL handling

The existing `fetch_xet_connection_info` and `xet_token_url` in `xet.rs` are refactored to accept a pre-built token URL instead of constructing it internally:

```rust
async fn fetch_xet_connection_info(api: &HFClient, token_url: &str) -> Result<XetConnectionInfo>;
```

Callers build their own URLs:
- **Repo:** `{endpoint}/api/{segment}/{repo_id}/xet-{read|write}-token/{revision}`
- **Bucket:** `{endpoint}/api/buckets/{bucket_id}/xet-{read|write}-token`

The xet session management (`xet_session()`, `replace_xet_session()`) remains on `HFClient` and is shared by both `HFRepository` and `HFBucket`.

### Bucket xet methods

`HFBucket` gets its own xet upload/download methods in `xet.rs` (behind the `xet` feature flag), following the same patterns as the existing `impl HFRepository` methods but using the bucket token URL.

## Error Handling

### New HFError variants

```rust
#[error("Bucket not found: {bucket_id}")]
BucketNotFound { bucket_id: String },

#[error("Forbidden")]
Forbidden,

#[error("Conflict: {0}")]
Conflict(String),

#[error("Rate limited")]
RateLimited,
```

### New NotFoundContext variant

```rust
pub(crate) enum NotFoundContext {
    Repo,
    Bucket,         // new
    Entry { path: String },
    Revision { revision: String },
    Generic,
}
```

### Updated check_response mapping

```rust
match status.as_u16() {
    401 => Err(HFError::AuthRequired),
    403 => Err(HFError::Forbidden),
    404 => match not_found_ctx {
        NotFoundContext::Bucket => Err(HFError::BucketNotFound { bucket_id: repo_id_str }),
        // ... existing variants
    },
    409 => Err(HFError::Conflict(body)),
    429 => Err(HFError::RateLimited),
    _ => Err(HFError::Http { status, url, body }),
}
```

The new 403/409/429 mappings improve error handling for all existing operations as well.

## Blocking API

### New type in `blocking.rs`

```rust
#[derive(Clone)]
pub struct HFBucketSync {
    pub(crate) inner: Arc<HFBucket>,
    pub(crate) runtime: Arc<tokio::runtime::Runtime>,
}
```

**Factory method on HFClientSync:**
```rust
impl HFClientSync {
    pub fn bucket(&self, owner: impl Into<String>, name: impl Into<String>) -> HFBucketSync;
}
```

**Blocking wrappers via macros:**
- `sync_api!` for `HFBucket -> HFBucketSync`: info, get_file_metadata, get_paths_info, batch, upload_files, download_files, delete_files
- `sync_api_stream!` for `HFBucket -> HFBucketSync`: list_tree
- `sync_api!` for `HFClient -> HFClientSync`: create_bucket, delete_bucket, move_bucket
- `sync_api_stream!` for `HFClient -> HFClientSync`: list_buckets

**Export:** `HFBucketSync` from `lib.rs` behind `blocking` feature flag.

## CLI

### New directory: `src/bin/hfrs/commands/buckets/`

Matches the Python `hf buckets` interface (minus sync).

### Command: `create`

```
hfrs buckets create <BUCKET_ID> [--private] [--exist-ok] [-q/--quiet]
```

- `BUCKET_ID`: positional, accepts `namespace/name` or `hf://buckets/namespace/name`
- Default output: `Bucket created: <URL> (handle: <HANDLE>)`
- Quiet: handle only

### Command: `list` (alias: `ls`)

```
hfrs buckets list [ARGUMENT] [-h/--human-readable] [--tree] [-R/--recursive] [--format {table,json}] [-q/--quiet]
```

- Namespace argument (no `/`): list buckets in that namespace. Table output: `id | private | size | total_files | created_at`. Argument is required (no implicit current-user default).
- `namespace/bucket_name[/prefix]`: list files in bucket. Default output: `<SIZE>  <DATE>  <PATH>`.
- `--tree`: ASCII tree format (files only).
- `-R/--recursive`: recursive listing (files only).
- `--tree` and `-R` invalid when listing buckets.
- `--tree` invalid with `--format json`.

### Command: `info`

```
hfrs buckets info <BUCKET_ID> [-q/--quiet]
```

- Default: JSON-formatted bucket details.
- Quiet: bucket ID only.

### Command: `delete`

```
hfrs buckets delete <BUCKET_ID> [-y/--yes] [--missing-ok] [-q/--quiet]
```

- Confirmation prompt unless `--yes`.

### Command: `remove` (alias: `rm`)

```
hfrs buckets remove <ARGUMENT> [-R/--recursive] [-y/--yes] [--dry-run] [--include PATTERN]... [--exclude PATTERN]... [-q/--quiet]
```

- `ARGUMENT`: `namespace/bucket_name/path` or `hf://buckets/...`
- `--include`/`--exclude` require `--recursive`.
- Shows count and total size before deletion.

### Command: `move`

```
hfrs buckets move <FROM_ID> <TO_ID>
```

- Output: `Bucket moved: <FROM_ID> -> <TO_ID>`

### Command: `cp`

```
hfrs buckets cp <SRC> [DST] [-q/--quiet]
```

- Supports: local-to-bucket, bucket-to-local, bucket-to-bucket, stdin (`-`), stdout (`-`)
- One of src/dst must be a bucket path (`hf://buckets/...`)
- Single files only (directories require sync, which is deferred)
- Bucket-to-bucket: server-side copy via batch endpoint's `copyFile` operation (by xet hash, no data transfer)
- Local-to-bucket: upload via xet, then register via batch `addFile`
- Bucket-to-local: resolve xet hash via `get_file_metadata`, download via xet
- Output: `Downloaded: ...` / `Uploaded: ...` / `Copied: ...`

### Registration

`Command` enum in `cli.rs`:
```rust
/// Interact with buckets on the Hub
Buckets(crate::commands::buckets::Args),
```

`commands/mod.rs`:
```rust
pub mod buckets;
```

## Files Changed (Summary)

### New files
- `huggingface_hub/src/bucket.rs` — HFBucket handle
- `huggingface_hub/src/types/buckets.rs` — bucket data types
- `huggingface_hub/src/types/bucket_params.rs` — bucket param structs
- `huggingface_hub/src/api/buckets.rs` — bucket API methods
- `huggingface_hub/src/bin/hfrs/commands/buckets/mod.rs` — CLI args + dispatch
- `huggingface_hub/src/bin/hfrs/commands/buckets/create.rs`
- `huggingface_hub/src/bin/hfrs/commands/buckets/list.rs`
- `huggingface_hub/src/bin/hfrs/commands/buckets/info.rs`
- `huggingface_hub/src/bin/hfrs/commands/buckets/delete.rs`
- `huggingface_hub/src/bin/hfrs/commands/buckets/remove.rs`
- `huggingface_hub/src/bin/hfrs/commands/buckets/move_bucket.rs`
- `huggingface_hub/src/bin/hfrs/commands/buckets/cp.rs`

### Modified files
- `huggingface_hub/src/lib.rs` — add `pub mod bucket;`, export `HFBucket`, `HFBucketSync`
- `huggingface_hub/src/client.rs` — add `bucket()` factory, `bucket_api_url()`, `bucket_resolve_url()`
- `huggingface_hub/src/error.rs` — add `BucketNotFound`, `Forbidden`, `Conflict`, `RateLimited` variants; update `NotFoundContext`
- `huggingface_hub/src/xet.rs` — refactor `fetch_xet_connection_info` to accept pre-built URL; add `impl HFBucket` xet methods
- `huggingface_hub/src/blocking.rs` — add `HFBucketSync`, factory method on `HFClientSync`
- `huggingface_hub/src/types/mod.rs` — add `pub mod buckets; pub mod bucket_params;` and re-exports
- `huggingface_hub/src/api/mod.rs` — add `pub mod buckets;`
- `huggingface_hub/src/bin/hfrs/cli.rs` — add `Buckets` variant to `Command` enum
- `huggingface_hub/src/bin/hfrs/commands/mod.rs` — add `pub mod buckets;`
- `huggingface_hub/src/macros.rs` — no changes (existing macros sufficient)

## Out of Scope

- `sync_bucket` — bidirectional sync with plan/apply, include/exclude, mtime comparison
- `copy_files` cross-repo/bucket copy (the `cp` CLI command handles single-file copies; the full `copy_files` API with cross-repo xet hash resolution is deferred)
- Caching (buckets have no revisions, so the existing cache system doesn't apply)
