# HFBucket Rust Client Design

**Date:** 2026-04-08  
**Repo:** `huggingface/huggingface_hub_rust`  
**Scope:** New `HFBucket` type + `HFClient` extensions + supporting types and error variants

## Overview

Add a `HFBucket` type to `huggingface_hub_rust` that exposes the HuggingFace Storage Buckets API (moon-landing). Buckets use content-addressable Xet storage rather than Git, making `HFRepository` the wrong abstraction — `HFBucket` is a separate handle type following the `HFSpace` precedent.

This spec covers the raw API surface only (option A). Higher-level upload abstractions (wrapping the Xet write token + batch commit flow into a single `upload_file` call) are deferred to a follow-up.

**Reference implementation:** `s3-gateway/src/hub_client/` in `huggingface/xet-catalogue`.

---

## Module Structure

Two new files, wired into their respective `mod.rs` files:

```
huggingface_hub/src/
├── api/
│   └── buckets.rs     — HFBucket impl, HFClient::bucket / create_bucket / list_buckets
├── types/
│   └── buckets.rs     — all request/response types
```

`lib.rs` exports `HFBucket` at the crate root. No feature flag — buckets are part of the default library surface.

---

## `HFBucket` Type

```rust
pub struct HFBucket {
    pub(crate) inner: Arc<HFClientInner>,
    pub namespace: String,
    pub repo: String,
}
```

Constructed via `HFClient::bucket()` — no I/O, no allocation beyond the string copies.

### `HFClient` extensions

```rust
// Constructs a bucket handle
pub fn bucket(&self, namespace: impl Into<String>, repo: impl Into<String>) -> HFBucket

// POST /api/buckets/:ns/:repo
pub async fn create_bucket(
    &self,
    namespace: &str,
    repo: &str,
    params: CreateBucketParams,
) -> Result<BucketCreated>

// GET /api/buckets/:ns — Link-header paginated stream
pub fn list_buckets(&self, namespace: &str) -> impl Stream<Item = Result<BucketOverview>>
```

### `HFBucket` methods

```rust
// GET /api/buckets/:ns/:repo
pub async fn get(&self) -> Result<BucketInfo>

// DELETE /api/buckets/:ns/:repo
pub async fn delete(&self) -> Result<()>

// PUT /api/buckets/:ns/:repo/settings
pub async fn update_settings(&self, params: UpdateBucketParams) -> Result<()>

// POST /api/buckets/:ns/:repo/batch (NDJSON)
pub async fn batch_files(&self, ops: Vec<BatchOp>) -> Result<BatchResult>

// GET /api/buckets/:ns/:repo/tree[/:path] — cursor-from-body paginated stream
pub fn list_tree(&self, path: &str, params: ListTreeParams) -> impl Stream<Item = Result<TreeEntry>>

// POST /api/buckets/:ns/:repo/paths-info
pub async fn get_paths_info(&self, paths: Vec<String>) -> Result<Vec<PathInfo>>

// GET /api/buckets/:ns/:repo/xet-write-token
pub async fn get_xet_write_token(&self) -> Result<XetToken>

// GET /api/buckets/:ns/:repo/xet-read-token
pub async fn get_xet_read_token(&self) -> Result<XetToken>

// GET /buckets/:ns/:repo/resolve/:path (no /api/ prefix)
pub async fn resolve_file(&self, path: &str) -> Result<ResolvedFile>

// GET /buckets/:ns/:repo/resolve/:path with Xet Accept header
#[cfg(feature = "xet")]
pub async fn xet_resolve_file(&self, path: &str) -> Result<XetFileInfo>
```

---

## Types (`src/types/buckets.rs`)

### Parameter types

All use `TypedBuilder`. `cursor` is omitted from list params — streaming handles pagination internally.

```rust
#[derive(TypedBuilder, Serialize)]
pub struct CreateBucketParams {
    #[builder(default, setter(strip_option))]
    pub private: Option<bool>,
    #[builder(default, setter(strip_option, into))]
    #[serde(rename = "resourceGroupId", skip_serializing_if = "Option::is_none")]
    pub resource_group_id: Option<String>,
    #[builder(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cdn: Vec<CdnRegion>,
}

#[derive(TypedBuilder, Serialize)]
pub struct UpdateBucketParams {
    #[builder(default, setter(strip_option))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private: Option<bool>,
    #[builder(default, setter(strip_option))]
    #[serde(rename = "cdnRegions", skip_serializing_if = "Option::is_none")]
    pub cdn_regions: Option<Vec<CdnRegion>>,
}

#[derive(TypedBuilder)]
pub struct ListTreeParams {
    #[builder(default, setter(strip_option))]
    pub limit: Option<u32>,
    #[builder(default)]
    pub recursive: bool,
}
```

### Response types

```rust
#[derive(Debug, Deserialize)]
pub struct BucketCreated {
    pub url: String,
    pub name: String,
    pub id: String,
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
pub struct XetToken {
    pub token: String,
    #[serde(rename = "casUrl")]
    pub cas_url: String,
    #[serde(rename = "expiresAt")]
    pub expires_at: String,
}

#[derive(Debug, Deserialize)]
pub struct PathInfo {
    pub path: String,
    pub size: u64,
    #[serde(rename = "xetHash")]
    pub xet_hash: String,
    #[serde(rename = "contentType")]
    pub content_type: String,
    pub mtime: i64,
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryType { File, Directory }
```

### `BucketOverview` (returned by `list_buckets`)

`list_buckets` yields `BucketOverview`, which is distinct from `BucketInfo` returned by `get()`. The `id` field is the full `"namespace/repo"` string.

```rust
#[derive(Debug, Deserialize)]
pub struct BucketOverview {
    #[serde(rename = "_id")]
    pub mongo_id: String,
    pub id: String,                          // "namespace/repo"
    pub author: String,
    pub private: Option<bool>,               // nullable
    #[serde(rename = "repoType")]
    pub repo_type: String,                   // always "bucket"
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

#[derive(Debug, Deserialize)]
pub struct ResourceGroup {
    pub id: String,
    pub name: String,
    #[serde(rename = "numUsers")]
    pub num_users: Option<u32>,
}
```

An internal `TreePage` struct (not public) is used for `list_tree` pagination:

```rust
#[derive(Deserialize)]
struct TreePage {
    entries: Vec<TreeEntry>,
    #[serde(rename = "nextCursor")]
    next_cursor: Option<String>,
}
```

### Batch types

The protocol requires all `addFile` entries to precede any `deleteFile` entries. Enforced in `batch_files` via partition before serialization.

```rust
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum BatchOp {
    #[serde(rename = "addFile")]
    AddFile(AddFileOp),
    #[serde(rename = "deleteFile")]
    DeleteFile(DeleteFileOp),
}

#[derive(Debug, Serialize)]
pub struct AddFileOp {
    pub path: String,
    #[serde(rename = "xetHash")]
    pub xet_hash: String,
    #[serde(rename = "contentType")]
    pub content_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mtime: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct DeleteFileOp {
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct BatchResult {
    pub success: bool,
    pub processed: u32,
    pub succeeded: u32,
    pub failed: Vec<BatchFailure>,
}

#[derive(Debug, Deserialize)]
pub struct BatchFailure {
    pub path: String,
    pub error: String,
}
```

### `ResolvedFile`

Constructed from redirect response headers, not a JSON body. The `Link` header contains two entries identified by `rel` name:

```
Link: <url>; rel="xet-auth", <url>; rel="xet-reconstruction-info"
```

```rust
#[derive(Debug)]
pub struct ResolvedFile {
    pub url: String,                            // Location header
    pub size: Option<u64>,                      // X-Linked-Size
    pub xet_hash: Option<String>,               // X-XET-Hash
    pub etag: Option<String>,                   // X-Linked-ETag
    pub last_modified: Option<String>,          // Last-Modified
    pub xet_auth_url: Option<String>,           // Link rel="xet-auth"
    pub xet_reconstruction_url: Option<String>, // Link rel="xet-reconstruction-info"
}
```

### `XetFileInfo` (feature = `"xet"`)

```rust
#[derive(Debug, Deserialize)]
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
```

---

## Pagination

**`list_buckets`** returns a JSON array with pagination via `Link` response headers (`rel="next"`), identical to the model/dataset list endpoints. The existing `paginate()` helper can be reused directly.

**`list_tree`** returns a JSON object `{ entries, nextCursor }` with cursor-in-body pagination. It uses `futures::stream::try_unfold` over cursor-from-body pagination, the same pattern as `pagination.rs` but reading `next_cursor` from the deserialized body rather than a `Link` header.

`list_tree` query params: `limit` (if set), `recursive` (if true), `cursor` (if continuing). Path suffix appended only when non-empty:

```
/api/buckets/:ns/:repo/tree          (empty path)
/api/buckets/:ns/:repo/tree/:path    (non-empty path)
```

Both streams are lazy — no HTTP request is made until the caller polls the first item.

---

## `resolve_file` and `xet_resolve_file`

**`resolve_file`** uses `inner.no_redirect_client` (already on `HFClientInner`) to prevent automatic redirect following. Expects a 3xx response. Reads `Location` and the metadata headers to populate `ResolvedFile`. Any non-redirect response (including 200) is passed to the standard error handler.

**`xet_resolve_file`** (feature = `"xet"`) uses the regular client. Sends `Accept: application/vnd.xet-fileinfo+json`. Expects a 200 JSON body deserializing to `XetFileInfo`.

---

## `batch_files` NDJSON

```
POST /api/buckets/:ns/:repo/batch
Content-Type: application/x-ndjson
```

Implementation:
1. Partition `ops` into adds and deletes (preserving within-group order).
2. Serialize each op with `serde_json::to_string`, append `\n`.
3. Concatenate into a single string body — adds first, then deletes.
4. Serialization errors surface as `HFError::Json`.

---

## Error Handling

Four new variants added to `HFError`. They will only be emitted by bucket API methods in this PR; updating existing non-bucket methods to use them is out of scope.

```rust
pub enum HFError {
    // ... existing variants ...
    #[error("forbidden")]
    Forbidden,
    #[error("conflict: {0}")]
    Conflict(String),   // carries response body
    #[error("rate limited")]
    RateLimited,
    #[error("quota exceeded")]
    QuotaExceeded,
}
```

Full status mapping for bucket methods:

| Status | `HFError` variant |
|--------|-------------------|
| 401 | `AuthRequired` (existing) |
| 403 | `Forbidden` (new) |
| 404 on bucket | `RepoNotFound { repo_id: "ns/repo" }` (existing) |
| 404 on file | `EntryNotFound { path, repo_id: "ns/repo" }` (existing) |
| 409 | `Conflict(body)` (new) |
| 429 | `RateLimited` (new) |
| 507 | `QuotaExceeded` (new) |
| other | `Http { status, url, body }` (existing) |

The 404 distinction is made at the call site: bucket-level methods (`get`, `delete`, `update_settings`) use `RepoNotFound`; file-level methods (`get_paths_info`, `resolve_file`) use `EntryNotFound`.

---

## Blocking API

All non-streaming `HFBucket` methods and `HFClient::create_bucket` get sync wrappers via the existing `sync_api!` macro. Streaming methods (`list_buckets`, `list_tree`) use `sync_api_stream!`, which wraps the async stream in a blocking iterator.

`HFClientSync::bucket()` returns `HFBucketSync`.

---

## Testing

**Unit tests** in `#[cfg(test)]` within `api/buckets.rs`:
- URL construction for each endpoint (including the path-suffix logic in `list_tree`)
- `resolve_file` header parsing (Location, X-Linked-Size, X-XET-Hash, Link)
- `batch_files` NDJSON ordering (adds before deletes)

**Integration tests** in `tests/integration_test.rs`, following existing patterns:
- Skip if `HF_TOKEN` absent
- Write operations (`create_bucket`, `batch_files`, `delete`) behind `HF_TEST_WRITE=1`
- Tests create and tear down their own bucket — no dependency on pre-existing test fixtures
- One integration test per public method; `list_buckets` and `list_tree` collect the stream into a `Vec` and assert on shape/contents

---

## Open Items

None.
