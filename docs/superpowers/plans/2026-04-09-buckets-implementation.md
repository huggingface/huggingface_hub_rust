# HuggingFace Buckets Support — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add HuggingFace Buckets API support to the Rust SDK with a new `HFBucket` handle, bucket API methods, blocking wrapper, and CLI commands matching the Python `hf buckets` interface.

**Architecture:** Parallel `HFBucket` type (like `HFRepository`), bucket-specific API methods split between `HFClient` (lifecycle) and `HFBucket` (scoped ops), xet integration refactored to share token-fetching infrastructure, CLI commands in `src/bin/hfrs/commands/buckets/`.

**Tech Stack:** Rust (async/await, tokio), reqwest, serde, typed-builder, clap, futures streams, xet-client (behind feature flag)

**Design Spec:** `docs/superpowers/specs/2026-04-09-buckets-design.md`

---

### Task 1: Error handling improvements

**Files:**
- Modify: `huggingface_hub/src/error.rs`
- Modify: `huggingface_hub/src/client.rs:273-304` (check_response)
- Modify: `huggingface_hub/src/bin/hfrs/main.rs:137-243` (format_hf_error)

- [ ] **Step 1: Add new HFError variants**

In `huggingface_hub/src/error.rs`, add three new variants to `HFError` and one new variant to `NotFoundContext`:

```rust
// Add after EntryNotFound variant:
#[error("Bucket not found: {bucket_id}")]
BucketNotFound { bucket_id: String },

// Add after XetNotEnabled variant (or any logical spot before the transparent variants):
#[error("Forbidden")]
Forbidden,

#[error("Conflict: {0}")]
Conflict(String),

#[error("Rate limited")]
RateLimited,
```

Add `Bucket` to `NotFoundContext`:

```rust
pub(crate) enum NotFoundContext {
    Repo,
    Bucket,
    Entry { path: String },
    Revision { revision: String },
    Generic,
}
```

- [ ] **Step 2: Update check_response in client.rs**

Replace the `match status.as_u16()` block in `HFClient::check_response` (lines 288-303):

```rust
match status.as_u16() {
    401 => Err(HFError::AuthRequired),
    403 => Err(HFError::Forbidden),
    404 => match not_found_ctx {
        crate::error::NotFoundContext::Repo => Err(HFError::RepoNotFound { repo_id: repo_id_str }),
        crate::error::NotFoundContext::Bucket => {
            Err(HFError::BucketNotFound { bucket_id: repo_id_str })
        },
        crate::error::NotFoundContext::Entry { path } => Err(HFError::EntryNotFound {
            path,
            repo_id: repo_id_str,
        }),
        crate::error::NotFoundContext::Revision { revision } => Err(HFError::RevisionNotFound {
            revision,
            repo_id: repo_id_str,
        }),
        crate::error::NotFoundContext::Generic => Err(HFError::Http { status, url, body }),
    },
    409 => Err(HFError::Conflict(body)),
    429 => Err(HFError::RateLimited),
    _ => Err(HFError::Http { status, url, body }),
}
```

- [ ] **Step 3: Update format_hf_error in main.rs**

Add match arms for the new variants in the `format_hf_error` function in `huggingface_hub/src/bin/hfrs/main.rs`. Insert after the `HFError::AuthRequired` arm:

```rust
HFError::BucketNotFound { bucket_id } => {
    format!("Bucket '{bucket_id}' not found. If the bucket is private, make sure you are authenticated.")
},
HFError::Forbidden => {
    "Permission denied. Check that your token has the required scopes for this operation.".to_string()
},
HFError::Conflict(body) => {
    if body.contains("already exists") {
        "Resource already exists. Use --exist-ok to skip this error.".to_string()
    } else {
        format!("Conflict: {body}")
    }
},
HFError::RateLimited => {
    "Rate limited. Please wait a moment and try again.".to_string()
},
```

Also remove the now-redundant `403`, `409`, and `429` cases from the `HFError::Http` match arm since they're now caught by dedicated variants. The `HFError::Http` match arm's inner `match status_code` should remove lines handling `403`, `409`, and `429` (those are now handled by the new `Forbidden`, `Conflict`, `RateLimited` variants that `check_response` produces).

- [ ] **Step 4: Verify compilation**

Run: `cargo clippy -p huggingface-hub --all-features -- -D warnings`
Expected: Clean compilation (or only pre-existing warnings unrelated to our changes).

- [ ] **Step 5: Run existing tests**

Run: `cargo test -p huggingface-hub`
Expected: All existing tests pass. The new error variants don't break existing behavior since the old `HFError::Http` for 403/409/429 now becomes `Forbidden`/`Conflict`/`RateLimited`, but only through `check_response` which is the only code path.

- [ ] **Step 6: Commit**

```bash
git add huggingface_hub/src/error.rs huggingface_hub/src/client.rs huggingface_hub/src/bin/hfrs/main.rs
git commit -m "feat: add BucketNotFound, Forbidden, Conflict, RateLimited error variants"
```

---

### Task 2: Bucket types and param structs

**Files:**
- Create: `huggingface_hub/src/types/buckets.rs`
- Create: `huggingface_hub/src/types/bucket_params.rs`
- Modify: `huggingface_hub/src/types/mod.rs`

- [ ] **Step 1: Create bucket data types**

Create `huggingface_hub/src/types/buckets.rs`:

```rust
use serde::{Deserialize, Serialize};

/// Metadata about a bucket on the Hugging Face Hub.
///
/// Returned by [`HFBucket::info`](crate::bucket::HFBucket::info) and
/// [`HFClient::list_buckets`](crate::client::HFClient::list_buckets).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketInfo {
    /// Full bucket identifier, e.g. `"namespace/bucket_name"`.
    pub id: String,
    /// Whether the bucket is private.
    pub private: bool,
    /// ISO 8601 creation timestamp.
    #[serde(rename = "createdAt")]
    pub created_at: String,
    /// Total size of all files in bytes.
    pub size: u64,
    /// Number of files in the bucket.
    #[serde(rename = "totalFiles")]
    pub total_files: u64,
}

/// URL returned after creating a bucket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketUrl {
    /// Full URL to the bucket on the Hub.
    pub url: String,
}

/// A file or directory entry in a bucket tree listing.
///
/// Returned by [`HFBucket::list_tree`](crate::bucket::HFBucket::list_tree) and
/// [`HFBucket::get_paths_info`](crate::bucket::HFBucket::get_paths_info).
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
        /// Last modification time (ISO 8601), if available.
        mtime: Option<String>,
        /// Upload timestamp (ISO 8601), if available.
        uploaded_at: Option<String>,
    },
    /// A directory entry.
    Directory {
        /// Directory path within the bucket.
        path: String,
        /// Upload timestamp (ISO 8601), if available.
        uploaded_at: Option<String>,
    },
}

/// Metadata for a single file in a bucket, retrieved via HEAD request.
///
/// Returned by [`HFBucket::get_file_metadata`](crate::bucket::HFBucket::get_file_metadata).
#[derive(Debug, Clone)]
pub struct BucketFileMetadata {
    /// File size in bytes.
    pub size: u64,
    /// Xet content-addressable hash.
    pub xet_hash: String,
}
```

- [ ] **Step 2: Create bucket param structs**

Create `huggingface_hub/src/types/bucket_params.rs`:

```rust
use std::path::PathBuf;

use typed_builder::TypedBuilder;

/// Parameters for creating a new bucket on the Hub.
///
/// Used with [`HFClient::create_bucket`](crate::client::HFClient::create_bucket).
#[derive(Debug, Clone, TypedBuilder)]
pub struct CreateBucketParams {
    /// Namespace (user or organization) that owns the bucket.
    #[builder(setter(into))]
    pub namespace: String,
    /// Bucket name within the namespace.
    #[builder(setter(into))]
    pub name: String,
    /// Whether the bucket should be private. Defaults to `false`.
    #[builder(default = false)]
    pub private: bool,
    /// Enterprise resource group ID (optional).
    #[builder(default, setter(into, strip_option))]
    pub resource_group_id: Option<String>,
    /// If `true`, do not error when the bucket already exists. Defaults to `false`.
    #[builder(default = false)]
    pub exist_ok: bool,
}

/// Parameters for listing files in a bucket tree.
///
/// Used with [`HFBucket::list_tree`](crate::bucket::HFBucket::list_tree).
#[derive(Debug, Clone, Default, TypedBuilder)]
pub struct ListBucketTreeParams {
    /// Filter results to entries under this prefix.
    #[builder(default, setter(into, strip_option))]
    pub prefix: Option<String>,
    /// If `true`, list entries recursively under the prefix.
    #[builder(default, setter(strip_option))]
    pub recursive: Option<bool>,
}

/// Parameters for batch operations on bucket files.
///
/// Used with [`HFBucket::batch`](crate::bucket::HFBucket::batch).
/// Operations are chunked at 1000 entries per request.
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
///
/// Represents an `addFile` entry in the NDJSON batch payload.
/// The file content must have already been uploaded to xet to obtain the `xet_hash`.
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
    /// MIME content type (e.g. `"text/plain"`, `"application/octet-stream"`).
    pub content_type: Option<String>,
}

/// A server-side copy operation for the batch endpoint.
///
/// Represents a `copyFile` entry in the NDJSON batch payload.
/// Copies are performed by xet hash — no data transfer occurs.
#[derive(Debug, Clone)]
pub struct BucketCopyFile {
    /// Destination path in the bucket.
    pub path: String,
    /// Xet content hash to copy.
    pub xet_hash: String,
    /// Source repo type (e.g. `"bucket"`, `"model"`).
    pub source_repo_type: String,
    /// Source repo or bucket ID (e.g. `"user/my-bucket"`).
    pub source_repo_id: String,
}

/// Parameters for downloading files from a bucket.
///
/// Used with [`HFBucket::download_files`](crate::bucket::HFBucket::download_files).
#[derive(Debug, Clone, TypedBuilder)]
pub struct BucketDownloadFilesParams {
    /// List of `(remote_path, local_path)` pairs to download.
    pub files: Vec<(String, PathBuf)>,
}
```

- [ ] **Step 3: Register types in mod.rs**

Add to `huggingface_hub/src/types/mod.rs`:

```rust
pub mod bucket_params;
pub mod buckets;
```

And add the re-exports:

```rust
pub use bucket_params::*;
pub use buckets::*;
```

- [ ] **Step 4: Verify compilation**

Run: `cargo clippy -p huggingface-hub --all-features -- -D warnings`
Expected: Clean compilation.

- [ ] **Step 5: Commit**

```bash
git add huggingface_hub/src/types/buckets.rs huggingface_hub/src/types/bucket_params.rs huggingface_hub/src/types/mod.rs
git commit -m "feat: add bucket data types and param structs"
```

---

### Task 3: HFBucket handle and URL helpers

**Files:**
- Create: `huggingface_hub/src/bucket.rs`
- Modify: `huggingface_hub/src/client.rs`
- Modify: `huggingface_hub/src/lib.rs`

- [ ] **Step 1: Create the HFBucket handle**

Create `huggingface_hub/src/bucket.rs`:

```rust
use std::fmt;

use crate::client::HFClient;

/// A handle for a single bucket on the Hugging Face Hub.
///
/// `HFBucket` is created via [`HFClient::bucket`] and binds together the client,
/// owner (namespace), and bucket name. All bucket-scoped API operations are methods
/// on this type.
///
/// Cheap to clone — the inner [`HFClient`] is `Arc`-backed.
///
/// # Example
///
/// ```rust,no_run
/// # use huggingface_hub::HFClient;
/// # #[tokio::main] async fn main() -> huggingface_hub::error::Result<()> {
/// let client = HFClient::builder().build()?;
/// let bucket = client.bucket("my-org", "my-bucket");
/// let info = bucket.info().await?;
/// println!("Bucket: {} ({} files)", info.id, info.total_files);
/// # Ok(()) }
/// ```
#[derive(Clone)]
pub struct HFBucket {
    pub(crate) hf_client: HFClient,
    owner: String,
    name: String,
}

impl fmt::Debug for HFBucket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HFBucket")
            .field("owner", &self.owner)
            .field("name", &self.name)
            .finish()
    }
}

impl HFBucket {
    /// Construct a new bucket handle. Prefer [`HFClient::bucket`] in most cases.
    pub fn new(client: HFClient, owner: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            hf_client: client,
            owner: owner.into(),
            name: name.into(),
        }
    }

    /// Return a reference to the underlying [`HFClient`].
    pub fn client(&self) -> &HFClient {
        &self.hf_client
    }

    /// The bucket owner (user or organization namespace).
    pub fn owner(&self) -> &str {
        &self.owner
    }

    /// The bucket name (without owner prefix).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The full `"owner/name"` bucket identifier used in Hub API calls.
    pub fn bucket_id(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::HFBucket;

    #[test]
    fn test_bucket_accessors() {
        let client = crate::HFClient::builder().build().unwrap();
        let bucket = HFBucket::new(client, "my-org", "my-bucket");

        assert_eq!(bucket.owner(), "my-org");
        assert_eq!(bucket.name(), "my-bucket");
        assert_eq!(bucket.bucket_id(), "my-org/my-bucket");
    }
}
```

- [ ] **Step 2: Add factory method and URL helpers to HFClient**

In `huggingface_hub/src/client.rs`, add the `bucket()` factory method in the existing `impl HFClient` block (after the `api_url` and `download_url` methods, before `check_response`):

```rust
/// Build a bucket API URL: `{endpoint}/api/buckets/{bucket_id}`
pub(crate) fn bucket_api_url(&self, bucket_id: &str) -> String {
    format!("{}/api/buckets/{}", self.endpoint(), bucket_id)
}

/// Build a bucket file resolve URL: `{endpoint}/buckets/{bucket_id}/resolve/{path}`
pub(crate) fn bucket_resolve_url(&self, bucket_id: &str, path: &str) -> String {
    format!("{}/buckets/{}/resolve/{}", self.endpoint(), bucket_id, path)
}
```

Also add the import and factory method. Add at the top of `client.rs` (in the imports section or create a new `impl HFClient` block):

```rust
use crate::bucket::HFBucket;
```

Then add to the `impl HFClient` block:

```rust
/// Create an [`HFBucket`] handle for a bucket.
pub fn bucket(&self, owner: impl Into<String>, name: impl Into<String>) -> HFBucket {
    HFBucket::new(self.clone(), owner, name)
}
```

- [ ] **Step 3: Register bucket module and exports in lib.rs**

In `huggingface_hub/src/lib.rs`, add the module declaration (after `pub mod cache;`):

```rust
pub mod bucket;
```

And add the re-export (after `pub use repository::*;`):

```rust
pub use bucket::*;
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p huggingface-hub`
Expected: All tests pass, including the new `test_bucket_accessors` test.

- [ ] **Step 5: Commit**

```bash
git add huggingface_hub/src/bucket.rs huggingface_hub/src/client.rs huggingface_hub/src/lib.rs
git commit -m "feat: add HFBucket handle with factory method and URL helpers"
```

---

### Task 4: Refactor xet token URL handling

**Files:**
- Modify: `huggingface_hub/src/xet.rs`

This task refactors the xet token infrastructure so both `HFRepository` and `HFBucket` can use it. The key change: `fetch_xet_connection_info` and `xet_token_url` now accept a pre-built token URL instead of constructing it from repo_type/repo_id/revision.

- [ ] **Step 1: Refactor fetch_xet_connection_info signature**

In `huggingface_hub/src/xet.rs`, change the `fetch_xet_connection_info` function (lines 40-62) to accept a pre-built token URL:

```rust
/// Fetch xet connection info from a token URL.
async fn fetch_xet_connection_info(api: &HFClient, token_url: &str) -> Result<XetConnectionInfo> {
    let response = api.http_client().get(token_url).headers(api.auth_headers()).send().await?;

    let response = api
        .check_response(response, None, crate::error::NotFoundContext::Generic)
        .await?;

    let token_resp: XetTokenResponse = response.json().await?;
    Ok(XetConnectionInfo {
        endpoint: token_resp.cas_url,
        access_token: token_resp.access_token,
        expiration_unix_epoch: token_resp.exp,
    })
}
```

- [ ] **Step 2: Add repo-specific token URL builder**

Replace the existing `xet_token_url` function (lines 64-73) with a repo-specific version:

```rust
/// Build xet token URL for a repository:
/// `{endpoint}/api/{segment}/{repo_id}/xet-{token_type}-token/{revision}`
fn repo_xet_token_url(
    api: &HFClient,
    token_type: &str,
    repo_id: &str,
    repo_type: Option<RepoType>,
    revision: &str,
) -> String {
    let segment = constants::repo_type_api_segment(repo_type);
    format!(
        "{}/api/{}/{}/xet-{}-token/{}",
        api.endpoint(),
        segment,
        repo_id,
        token_type,
        revision
    )
}

/// Build xet token URL for a bucket:
/// `{endpoint}/api/buckets/{bucket_id}/xet-{token_type}-token`
pub(crate) fn bucket_xet_token_url(api: &HFClient, token_type: &str, bucket_id: &str) -> String {
    format!(
        "{}/api/buckets/{}/xet-{}-token",
        api.endpoint(),
        bucket_id,
        token_type
    )
}
```

- [ ] **Step 3: Update all HFRepository xet methods to use new signatures**

Update every call to the old `fetch_xet_connection_info` and `xet_token_url` in the `impl HFRepository` block. There are multiple call sites — each needs two changes:

1. Replace `fetch_xet_connection_info(&self.hf_client, "read"/"write", &repo_path, repo_type, revision)` with:
   ```rust
   let token_url = repo_xet_token_url(&self.hf_client, "read"/"write", &repo_path, repo_type, revision);
   fetch_xet_connection_info(&self.hf_client, &token_url)
   ```

2. Replace `xet_token_url(&self.hf_client, ...)` with `repo_xet_token_url(&self.hf_client, ...)` in the `.with_token_refresh_url()` calls.

The affected methods are:
- `xet_download_to_local_dir` (line ~110 and ~133)
- `xet_download_to_blob` (line ~164 and ~189)
- `xet_download_batch` (line ~217 and ~233)
- `xet_download_stream` (line ~283 and ~299)
- `xet_upload` (line ~332 and ~350)

- [ ] **Step 4: Update HFClient::get_xet_token**

Update the `get_xet_token` method (lines 399-402) to use the new function:

```rust
pub async fn get_xet_token(&self, params: &GetXetTokenParams) -> Result<XetConnectionInfo> {
    let revision = params.revision.as_deref().unwrap_or(constants::DEFAULT_REVISION);
    let token_url = repo_xet_token_url(
        self,
        params.token_type.as_str(),
        &params.repo_id,
        params.repo_type,
        revision,
    );
    fetch_xet_connection_info(self, &token_url).await
}
```

- [ ] **Step 5: Verify compilation and tests**

Run: `cargo clippy -p huggingface-hub --all-features -- -D warnings && cargo test -p huggingface-hub`
Expected: All pass — this is a pure refactor, no behavior change.

- [ ] **Step 6: Commit**

```bash
git add huggingface_hub/src/xet.rs
git commit -m "refactor: extract xet token URL builders for repo and bucket"
```

---

### Task 5: Bucket lifecycle API on HFClient

**Files:**
- Create: `huggingface_hub/src/api/buckets.rs`
- Modify: `huggingface_hub/src/api/mod.rs`

- [ ] **Step 1: Create api/buckets.rs with lifecycle methods**

Create `huggingface_hub/src/api/buckets.rs`:

```rust
use futures::Stream;
use url::Url;

use crate::bucket::HFBucket;
use crate::client::HFClient;
use crate::error::{HFError, NotFoundContext, Result};
use crate::types::{BucketInfo, BucketUrl, CreateBucketParams};

impl HFClient {
    /// Create a new bucket on the Hub.
    ///
    /// Endpoint: `POST /api/buckets/{namespace}/{name}`
    pub async fn create_bucket(&self, params: &CreateBucketParams) -> Result<BucketUrl> {
        let url = format!(
            "{}/api/buckets/{}/{}",
            self.endpoint(),
            params.namespace,
            params.name
        );

        let mut body = serde_json::json!({});
        if params.private {
            body["private"] = serde_json::Value::Bool(true);
        }
        if let Some(ref rg) = params.resource_group_id {
            body["resourceGroupId"] = serde_json::Value::String(rg.clone());
        }

        let response = self
            .http_client()
            .post(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;

        let bucket_id = format!("{}/{}", params.namespace, params.name);

        if response.status().as_u16() == 409 && params.exist_ok {
            return Ok(BucketUrl {
                url: format!("{}/buckets/{}", self.endpoint(), bucket_id),
            });
        }

        let response = self
            .check_response(response, Some(&bucket_id), NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    /// Delete a bucket from the Hub.
    ///
    /// Endpoint: `DELETE /api/buckets/{bucket_id}`
    pub async fn delete_bucket(&self, bucket_id: &str, missing_ok: bool) -> Result<()> {
        let url = self.bucket_api_url(bucket_id);

        let response = self
            .http_client()
            .delete(&url)
            .headers(self.auth_headers())
            .send()
            .await?;

        if response.status().as_u16() == 404 && missing_ok {
            return Ok(());
        }

        self.check_response(response, Some(bucket_id), NotFoundContext::Bucket)
            .await?;
        Ok(())
    }

    /// List buckets in a namespace.
    ///
    /// Endpoint: `GET /api/buckets/{namespace}` (paginated)
    pub fn list_buckets(&self, namespace: &str) -> Result<impl Stream<Item = Result<BucketInfo>> + '_> {
        let url = Url::parse(&format!("{}/api/buckets/{}", self.endpoint(), namespace))?;
        Ok(self.paginate(url, vec![], None))
    }

    /// Move (rename) a bucket.
    ///
    /// Endpoint: `POST /api/repos/move` with `type: "bucket"`
    pub async fn move_bucket(&self, from_id: &str, to_id: &str) -> Result<()> {
        let url = format!("{}/api/repos/move", self.endpoint());
        let body = serde_json::json!({
            "fromRepo": from_id,
            "toRepo": to_id,
            "type": "bucket",
        });

        let response = self
            .http_client()
            .post(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;

        self.check_response(response, None, NotFoundContext::Generic)
            .await?;
        Ok(())
    }
}
```

- [ ] **Step 2: Register buckets module in api/mod.rs**

Add to `huggingface_hub/src/api/mod.rs`:

```rust
pub mod buckets;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo clippy -p huggingface-hub --all-features -- -D warnings`
Expected: Clean compilation.

- [ ] **Step 4: Commit**

```bash
git add huggingface_hub/src/api/buckets.rs huggingface_hub/src/api/mod.rs
git commit -m "feat: add bucket lifecycle API methods on HFClient"
```

---

### Task 6: Bucket scoped API methods on HFBucket

**Files:**
- Modify: `huggingface_hub/src/api/buckets.rs`

- [ ] **Step 1: Add info, list_tree, get_paths_info, get_file_metadata, batch, and delete_files**

Append to `huggingface_hub/src/api/buckets.rs`:

```rust
use crate::types::{
    BatchBucketFilesParams, BucketFileMetadata, BucketTreeEntry, ListBucketTreeParams,
};

const BUCKET_BATCH_CHUNK_SIZE: usize = 1000;
const BUCKET_PATHS_INFO_BATCH_SIZE: usize = 1000;

impl HFBucket {
    /// Fetch metadata for this bucket.
    ///
    /// Endpoint: `GET /api/buckets/{bucket_id}`
    pub async fn info(&self) -> Result<BucketInfo> {
        let bucket_id = self.bucket_id();
        let url = self.hf_client.bucket_api_url(&bucket_id);

        let response = self
            .hf_client
            .http_client()
            .get(&url)
            .headers(self.hf_client.auth_headers())
            .send()
            .await?;

        let response = self
            .hf_client
            .check_response(response, Some(&bucket_id), NotFoundContext::Bucket)
            .await?;
        Ok(response.json().await?)
    }

    /// List files and directories in this bucket.
    ///
    /// Endpoint: `GET /api/buckets/{bucket_id}/tree[/{prefix}]` (paginated)
    pub fn list_tree(
        &self,
        params: &ListBucketTreeParams,
    ) -> Result<impl Stream<Item = Result<BucketTreeEntry>> + '_> {
        let bucket_id = self.bucket_id();
        let mut url_str = format!("{}/api/buckets/{}/tree", self.hf_client.endpoint(), bucket_id);
        if let Some(ref prefix) = params.prefix {
            url_str = format!("{}/{}", url_str, prefix);
        }
        let url = Url::parse(&url_str)?;

        let mut query = Vec::new();
        if let Some(true) = params.recursive {
            query.push(("recursive".to_string(), "true".to_string()));
        }

        Ok(self.hf_client.paginate(url, query, None))
    }

    /// Get file information for specific paths in the bucket.
    ///
    /// Endpoint: `POST /api/buckets/{bucket_id}/paths-info` (batched at 1000 per request)
    pub async fn get_paths_info(&self, paths: &[String]) -> Result<Vec<BucketTreeEntry>> {
        let bucket_id = self.bucket_id();
        let url = format!(
            "{}/api/buckets/{}/paths-info",
            self.hf_client.endpoint(),
            bucket_id
        );

        let mut all_entries = Vec::new();

        for chunk in paths.chunks(BUCKET_PATHS_INFO_BATCH_SIZE) {
            let body = serde_json::json!({ "paths": chunk });

            let response = self
                .hf_client
                .http_client()
                .post(&url)
                .headers(self.hf_client.auth_headers())
                .json(&body)
                .send()
                .await?;

            let response = self
                .hf_client
                .check_response(response, Some(&bucket_id), NotFoundContext::Bucket)
                .await?;

            let entries: Vec<BucketTreeEntry> = response.json().await?;
            all_entries.extend(entries);
        }

        Ok(all_entries)
    }

    /// Get metadata for a single file via HEAD request.
    ///
    /// Endpoint: `HEAD /buckets/{bucket_id}/resolve/{path}`
    pub async fn get_file_metadata(&self, remote_path: &str) -> Result<BucketFileMetadata> {
        let bucket_id = self.bucket_id();
        let url = self.hf_client.bucket_resolve_url(&bucket_id, remote_path);

        let response = self
            .hf_client
            .no_redirect_client()
            .head(&url)
            .headers(self.hf_client.auth_headers())
            .send()
            .await?;

        let response = self
            .hf_client
            .check_response(
                response,
                Some(&bucket_id),
                NotFoundContext::Entry {
                    path: remote_path.to_string(),
                },
            )
            .await?;

        let size = response
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);

        let xet_hash = response
            .headers()
            .get("x-xet-hash")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        Ok(BucketFileMetadata { size, xet_hash })
    }

    /// Execute batch operations on bucket files.
    ///
    /// Sends an NDJSON payload to `POST /api/buckets/{bucket_id}/batch`.
    /// Operations are chunked at 1000 entries per request.
    pub async fn batch(&self, params: &BatchBucketFilesParams) -> Result<()> {
        let bucket_id = self.bucket_id();
        let url = format!(
            "{}/api/buckets/{}/batch",
            self.hf_client.endpoint(),
            bucket_id
        );

        // Build NDJSON lines
        let mut lines: Vec<serde_json::Value> = Vec::new();

        for file in &params.add {
            let mut entry = serde_json::json!({
                "type": "addFile",
                "path": file.path,
                "xetHash": file.xet_hash,
                "size": file.size,
            });
            if let Some(mtime) = file.mtime {
                entry["mtime"] = serde_json::Value::Number(mtime.into());
            }
            if let Some(ref ct) = file.content_type {
                entry["contentType"] = serde_json::Value::String(ct.clone());
            }
            lines.push(entry);
        }

        for path in &params.delete {
            lines.push(serde_json::json!({
                "type": "deleteFile",
                "path": path,
            }));
        }

        for copy in &params.copy {
            lines.push(serde_json::json!({
                "type": "copyFile",
                "path": copy.path,
                "xetHash": copy.xet_hash,
                "sourceRepoType": copy.source_repo_type,
                "sourceRepoId": copy.source_repo_id,
            }));
        }

        // Send in chunks
        for chunk in lines.chunks(BUCKET_BATCH_CHUNK_SIZE) {
            let ndjson_body = chunk
                .iter()
                .map(|v| serde_json::to_string(v).unwrap())
                .collect::<Vec<_>>()
                .join("\n");

            let response = self
                .hf_client
                .http_client()
                .post(&url)
                .headers(self.hf_client.auth_headers())
                .header("content-type", "application/x-ndjson")
                .body(ndjson_body)
                .send()
                .await?;

            self.hf_client
                .check_response(response, Some(&bucket_id), NotFoundContext::Bucket)
                .await?;
        }

        Ok(())
    }

    /// Delete files from the bucket.
    ///
    /// Convenience wrapper around [`batch`](Self::batch) with delete-only operations.
    pub async fn delete_files(&self, paths: &[String]) -> Result<()> {
        let params = BatchBucketFilesParams {
            delete: paths.to_vec(),
            ..Default::default()
        };
        self.batch(&params).await
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo clippy -p huggingface-hub --all-features -- -D warnings`
Expected: Clean compilation.

- [ ] **Step 3: Commit**

```bash
git add huggingface_hub/src/api/buckets.rs
git commit -m "feat: add bucket scoped API methods (info, list_tree, batch, etc.)"
```

---

### Task 7: Bucket xet upload/download methods

**Files:**
- Modify: `huggingface_hub/src/xet.rs`
- Modify: `huggingface_hub/src/api/buckets.rs`

- [ ] **Step 1: Add bucket xet methods to xet.rs**

Append to `huggingface_hub/src/xet.rs`, inside a new `impl HFBucket` block (before the `impl HFClient` block for `get_xet_token` and before the `#[cfg(test)]` module):

```rust
use crate::bucket::HFBucket;

impl HFBucket {
    /// Upload files to xet and return file info (hash + size) for each.
    pub(crate) async fn xet_upload(
        &self,
        files: &[(String, AddSource)],
    ) -> Result<Vec<XetFileInfo>> {
        let bucket_id = self.bucket_id();
        let token_url = bucket_xet_token_url(&self.hf_client, "write", &bucket_id);
        tracing::info!(bucket = bucket_id.as_str(), "fetching bucket xet write token");
        let conn = fetch_xet_connection_info(&self.hf_client, &token_url).await?;
        tracing::info!(endpoint = conn.endpoint.as_str(), "bucket xet write token obtained");

        let (session, generation) = self.hf_client.xet_session()?;
        let commit = match session.new_upload_commit() {
            Ok(b) => b,
            Err(e) => {
                self.hf_client.replace_xet_session(generation, &e);
                self.hf_client
                    .xet_session()?
                    .0
                    .new_upload_commit()
                    .map_err(|e| HFError::Other(format!("Xet upload failed: {e}")))?
            },
        }
        .with_endpoint(conn.endpoint.clone())
        .with_token_info(conn.access_token.clone(), conn.expiration_unix_epoch)
        .with_token_refresh_url(token_url, self.hf_client.auth_headers())
        .build()
        .await
        .map_err(|e| HFError::Other(format!("Xet upload failed: {e}")))?;

        let mut task_ids_in_order = Vec::with_capacity(files.len());

        for (path_in_bucket, source) in files {
            tracing::info!(path = path_in_bucket.as_str(), "queuing bucket xet upload");
            let handle = match source {
                AddSource::File(path) => commit
                    .upload_from_path(path.clone(), Sha256Policy::Compute)
                    .await
                    .map_err(|e| HFError::Other(format!("Xet upload failed: {e}")))?,
                AddSource::Bytes(bytes) => commit
                    .upload_bytes(bytes.clone(), Sha256Policy::Compute, None)
                    .await
                    .map_err(|e| HFError::Other(format!("Xet upload failed: {e}")))?,
            };
            task_ids_in_order.push(handle.task_id());
        }

        tracing::info!(file_count = files.len(), "committing bucket xet uploads");
        let results = commit
            .commit()
            .await
            .map_err(|e| HFError::Other(format!("Xet upload failed: {e}")))?;

        let mut xet_file_infos = Vec::with_capacity(files.len());
        for task_id in &task_ids_in_order {
            let metadata: &XetFileMetadata = results
                .uploads
                .get(task_id)
                .ok_or_else(|| HFError::Other("Missing xet upload result for task".to_string()))?;
            xet_file_infos.push(metadata.xet_info.clone());
        }

        Ok(xet_file_infos)
    }

    /// Download files from a bucket via xet.
    pub(crate) async fn xet_download_batch(&self, files: &[XetBatchFile]) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }

        let bucket_id = self.bucket_id();
        let token_url = bucket_xet_token_url(&self.hf_client, "read", &bucket_id);
        let conn = fetch_xet_connection_info(&self.hf_client, &token_url).await?;

        let (session, generation) = self.hf_client.xet_session()?;
        let group = match session.new_file_download_group() {
            Ok(b) => b,
            Err(e) => {
                self.hf_client.replace_xet_session(generation, &e);
                self.hf_client
                    .xet_session()?
                    .0
                    .new_file_download_group()
                    .map_err(|e| HFError::Other(format!("Xet batch download failed: {e}")))?
            },
        }
        .with_endpoint(conn.endpoint.clone())
        .with_token_info(conn.access_token.clone(), conn.expiration_unix_epoch)
        .with_token_refresh_url(token_url, self.hf_client.auth_headers())
        .build()
        .await
        .map_err(|e| HFError::Other(format!("Xet batch download failed: {e}")))?;

        let mut incomplete_paths = Vec::with_capacity(files.len());
        for file in files {
            if let Some(parent) = file.path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }

            let incomplete = PathBuf::from(format!("{}.incomplete", file.path.display()));
            let file_info = XetFileInfo::new(file.hash.clone(), file.file_size);

            group
                .download_file_to_path(file_info, incomplete.clone())
                .await
                .map_err(|e| HFError::Other(format!("Xet batch download failed: {e}")))?;

            incomplete_paths.push((incomplete, file.path.clone()));
        }

        group
            .finish()
            .await
            .map_err(|e| HFError::Other(format!("Xet batch download failed: {e}")))?;

        for (incomplete, final_path) in &incomplete_paths {
            tokio::fs::rename(incomplete, final_path).await?;
        }

        Ok(())
    }
}
```

- [ ] **Step 2: Add upload_files and download_files to api/buckets.rs**

Append to `huggingface_hub/src/api/buckets.rs` (in the `impl HFBucket` block or a new one):

```rust
use std::path::PathBuf;
use crate::types::BucketDownloadFilesParams;

#[cfg(feature = "xet")]
use crate::types::BucketAddFile;

impl HFBucket {
    /// Upload local files to the bucket.
    ///
    /// Uploads file contents to xet, then registers them via the batch endpoint.
    #[cfg(feature = "xet")]
    pub async fn upload_files(&self, files: &[(PathBuf, String)]) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }

        let xet_files: Vec<(String, crate::types::AddSource)> = files
            .iter()
            .map(|(local_path, remote_path)| {
                (remote_path.clone(), crate::types::AddSource::File(local_path.clone()))
            })
            .collect();

        let xet_infos = self.xet_upload(&xet_files).await?;

        let add_files: Vec<BucketAddFile> = files
            .iter()
            .zip(xet_infos.iter())
            .map(|((local_path, remote_path), xet_info)| {
                let metadata = std::fs::metadata(local_path).ok();
                let size = metadata.as_ref().map(|m| m.len()).unwrap_or(xet_info.file_size);
                let mtime = metadata
                    .as_ref()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs());
                let content_type = mime_guess::from_path(local_path)
                    .first()
                    .map(|m| m.to_string())
                    .or_else(|| mime_guess::from_path(remote_path).first().map(|m| m.to_string()));

                BucketAddFile {
                    path: remote_path.clone(),
                    xet_hash: xet_info.file_hash.clone(),
                    size,
                    mtime,
                    content_type,
                }
            })
            .collect();

        let batch_params = crate::types::BatchBucketFilesParams {
            add: add_files,
            ..Default::default()
        };
        self.batch(&batch_params).await
    }

    /// Upload local files to the bucket (stub when xet feature is disabled).
    #[cfg(not(feature = "xet"))]
    pub async fn upload_files(&self, _files: &[(PathBuf, String)]) -> Result<()> {
        Err(HFError::XetNotEnabled)
    }

    /// Download files from the bucket to local paths.
    ///
    /// Resolves xet hashes via `get_paths_info`, then downloads via xet.
    #[cfg(feature = "xet")]
    pub async fn download_files(&self, params: &BucketDownloadFilesParams) -> Result<()> {
        if params.files.is_empty() {
            return Ok(());
        }

        let remote_paths: Vec<String> = params.files.iter().map(|(r, _)| r.clone()).collect();
        let entries = self.get_paths_info(&remote_paths).await?;

        let mut xet_batch_files = Vec::new();

        for ((_remote_path, local_path), entry) in params.files.iter().zip(entries.iter()) {
            match entry {
                BucketTreeEntry::File {
                    xet_hash, size, ..
                } => {
                    xet_batch_files.push(crate::xet::XetBatchFile {
                        hash: xet_hash.clone(),
                        file_size: *size,
                        path: local_path.clone(),
                    });
                },
                BucketTreeEntry::Directory { path, .. } => {
                    return Err(HFError::InvalidParameter(format!(
                        "Cannot download directory entry: {path}"
                    )));
                },
            }
        }

        self.xet_download_batch(&xet_batch_files).await
    }

    /// Download files from the bucket (stub when xet feature is disabled).
    #[cfg(not(feature = "xet"))]
    pub async fn download_files(&self, _params: &BucketDownloadFilesParams) -> Result<()> {
        Err(HFError::XetNotEnabled)
    }
}
```

- [ ] **Step 3: Note on content_type**

Skip MIME type guessing for now — set `content_type: None` in every `BucketAddFile`. No new dependency needed. This can be revisited later if content-type detection becomes important. The `upload_files` code in Step 2 should use:

```rust
BucketAddFile {
    path: remote_path.clone(),
    xet_hash: xet_info.file_hash.clone(),
    size,
    mtime,
    content_type: None,
}
```

Remove the `mime_guess` import and content_type detection logic from the Step 2 code above.

- [ ] **Step 4: Verify compilation**

Run: `cargo clippy -p huggingface-hub --all-features -- -D warnings`
Expected: Clean compilation.

- [ ] **Step 5: Commit**

```bash
git add huggingface_hub/src/xet.rs huggingface_hub/src/api/buckets.rs
git commit -m "feat: add bucket xet upload/download and upload_files/download_files"
```

---

### Task 8: Blocking API (HFBucketSync)

**Files:**
- Modify: `huggingface_hub/src/blocking.rs`
- Modify: `huggingface_hub/src/api/buckets.rs` (add sync_api macros)
- Modify: `huggingface_hub/src/lib.rs`

- [ ] **Step 1: Add HFBucketSync to blocking.rs**

In `huggingface_hub/src/blocking.rs`, add the import and struct. Add `use crate::bucket;` to the imports at top.

Add after `HFSpaceSync`:

```rust
/// Synchronous/blocking counterpart to [`bucket::HFBucket`].
///
/// Holds a reference to the underlying async handle and the shared tokio runtime.
/// Blocking API methods are defined via the `sync_api!` macro in `api/buckets.rs`.
#[derive(Clone)]
pub struct HFBucketSync {
    pub(crate) inner: Arc<bucket::HFBucket>,
    pub(crate) runtime: Arc<tokio::runtime::Runtime>,
}

impl fmt::Debug for HFBucketSync {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HFBucketSync").field("inner", &self.inner).finish()
    }
}

impl HFBucketSync {
    /// Creates a blocking bucket handle from a client, owner, and name.
    pub fn new(client: HFClientSync, owner: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            inner: Arc::new(bucket::HFBucket::new(client.inner.clone(), owner, name)),
            runtime: client.runtime.clone(),
        }
    }
}

impl std::ops::Deref for HFBucketSync {
    type Target = bucket::HFBucket;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
```

Add factory method to `HFClientSync` (in the existing `impl HFClientSync` block):

```rust
/// Creates a blocking handle for a bucket.
pub fn bucket(&self, owner: impl Into<String>, name: impl Into<String>) -> HFBucketSync {
    HFBucketSync::new(self.clone(), owner, name)
}
```

- [ ] **Step 2: Add sync_api macros to api/buckets.rs**

Append to the bottom of `huggingface_hub/src/api/buckets.rs`:

```rust
sync_api! {
    impl HFClient -> HFClientSync {
        fn create_bucket(&self, params: &CreateBucketParams) -> Result<BucketUrl>;
        fn delete_bucket(&self, bucket_id: &str, missing_ok: bool) -> Result<()>;
        fn move_bucket(&self, from_id: &str, to_id: &str) -> Result<()>;
    }
}

sync_api_stream! {
    impl HFClient -> HFClientSync {
        fn list_buckets(&self, namespace: &str) -> BucketInfo;
    }
}

sync_api! {
    impl HFBucket -> HFBucketSync {
        fn info(&self) -> Result<BucketInfo>;
        fn get_file_metadata(&self, remote_path: &str) -> Result<BucketFileMetadata>;
        fn get_paths_info(&self, paths: &[String]) -> Result<Vec<BucketTreeEntry>>;
        fn batch(&self, params: &BatchBucketFilesParams) -> Result<()>;
        fn upload_files(&self, files: &[(std::path::PathBuf, String)]) -> Result<()>;
        fn download_files(&self, params: &BucketDownloadFilesParams) -> Result<()>;
        fn delete_files(&self, paths: &[String]) -> Result<()>;
    }
}

sync_api_stream! {
    impl HFBucket -> HFBucketSync {
        fn list_tree(&self, params: &ListBucketTreeParams) -> BucketTreeEntry;
    }
}
```

- [ ] **Step 3: Export HFBucketSync from lib.rs**

In `huggingface_hub/src/lib.rs`, update the blocking export line:

```rust
#[cfg(feature = "blocking")]
pub use blocking::{HFBucketSync, HFClientSync, HFRepoSync, HFRepositorySync, HFSpaceSync};
```

- [ ] **Step 4: Verify compilation and run tests**

Run: `cargo clippy -p huggingface-hub --all-features -- -D warnings && cargo test -p huggingface-hub`
Expected: All pass.

- [ ] **Step 5: Commit**

```bash
git add huggingface_hub/src/blocking.rs huggingface_hub/src/api/buckets.rs huggingface_hub/src/lib.rs
git commit -m "feat: add HFBucketSync blocking wrapper"
```

---

### Task 9: CLI scaffolding and simple commands (create, info, delete, move)

**Files:**
- Create: `huggingface_hub/src/bin/hfrs/commands/buckets/mod.rs`
- Create: `huggingface_hub/src/bin/hfrs/commands/buckets/create.rs`
- Create: `huggingface_hub/src/bin/hfrs/commands/buckets/info.rs`
- Create: `huggingface_hub/src/bin/hfrs/commands/buckets/delete.rs`
- Create: `huggingface_hub/src/bin/hfrs/commands/buckets/move_bucket.rs`
- Modify: `huggingface_hub/src/bin/hfrs/commands/mod.rs`
- Modify: `huggingface_hub/src/bin/hfrs/cli.rs`
- Modify: `huggingface_hub/src/bin/hfrs/main.rs`

- [ ] **Step 1: Create buckets/mod.rs**

Create `huggingface_hub/src/bin/hfrs/commands/buckets/mod.rs`:

```rust
pub mod create;
pub mod delete;
pub mod info;
pub mod move_bucket;

use anyhow::Result;
use clap::{Args as ClapArgs, Subcommand};
use huggingface_hub::HFClient;

use crate::output::CommandResult;

/// Interact with buckets on the Hub
#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    pub command: BucketsCommand,
}

#[derive(Subcommand)]
pub enum BucketsCommand {
    /// Create a new bucket
    Create(create::Args),
    /// Show detailed information about a bucket
    Info(info::Args),
    /// Delete a bucket
    Delete(delete::Args),
    /// Move (rename) a bucket
    Move(move_bucket::Args),
}

pub async fn execute(api: &HFClient, args: Args) -> Result<CommandResult> {
    match args.command {
        BucketsCommand::Create(a) => create::execute(api, a).await,
        BucketsCommand::Info(a) => info::execute(api, a).await,
        BucketsCommand::Delete(a) => delete::execute(api, a).await,
        BucketsCommand::Move(a) => move_bucket::execute(api, a).await,
    }
}
```

- [ ] **Step 2: Create a helper for parsing bucket IDs**

The CLI needs to parse `hf://buckets/namespace/name` or `namespace/name` into `(namespace, name)`. Add this as a function in `buckets/mod.rs`:

```rust
/// Parse a bucket ID from CLI input.
/// Accepts `namespace/name` or `hf://buckets/namespace/name`.
/// Returns `(namespace, name)` or an error.
pub(crate) fn parse_bucket_id(input: &str) -> Result<(String, String)> {
    let id = input
        .strip_prefix("hf://buckets/")
        .unwrap_or(input);

    match id.split_once('/') {
        Some((ns, name)) if !ns.is_empty() && !name.is_empty() && !name.contains('/') => {
            Ok((ns.to_string(), name.to_string()))
        },
        _ => anyhow::bail!(
            "Invalid bucket ID '{input}'. Expected format: 'namespace/bucket_name' or 'hf://buckets/namespace/bucket_name'"
        ),
    }
}
```

- [ ] **Step 3: Create create.rs**

Create `huggingface_hub/src/bin/hfrs/commands/buckets/create.rs`:

```rust
use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{CreateBucketParams, HFClient};

use super::parse_bucket_id;
use crate::output::CommandResult;

/// Create a new bucket
#[derive(ClapArgs)]
pub struct Args {
    /// Bucket ID (namespace/name or hf://buckets/namespace/name)
    pub bucket_id: String,

    /// Make the bucket private
    #[arg(long)]
    pub private: bool,

    /// Do not fail if the bucket already exists
    #[arg(long)]
    pub exist_ok: bool,

    /// Print only the bucket handle
    #[arg(short, long)]
    pub quiet: bool,
}

pub async fn execute(api: &HFClient, args: Args) -> Result<CommandResult> {
    let (namespace, name) = parse_bucket_id(&args.bucket_id)?;

    let params = CreateBucketParams::builder()
        .namespace(&namespace)
        .name(&name)
        .private(args.private)
        .exist_ok(args.exist_ok)
        .build();

    let result = api.create_bucket(&params).await?;
    let handle = format!("hf://buckets/{}/{}", namespace, name);

    if args.quiet {
        Ok(CommandResult::Raw(handle))
    } else {
        Ok(CommandResult::Raw(format!(
            "Bucket created: {} (handle: {})",
            result.url, handle
        )))
    }
}
```

- [ ] **Step 4: Create info.rs**

Create `huggingface_hub/src/bin/hfrs/commands/buckets/info.rs`:

```rust
use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::HFClient;

use super::parse_bucket_id;
use crate::output::CommandResult;

/// Show detailed information about a bucket
#[derive(ClapArgs)]
pub struct Args {
    /// Bucket ID (namespace/name or hf://buckets/namespace/name)
    pub bucket_id: String,

    /// Print only the bucket ID
    #[arg(short, long)]
    pub quiet: bool,
}

pub async fn execute(api: &HFClient, args: Args) -> Result<CommandResult> {
    let (namespace, name) = parse_bucket_id(&args.bucket_id)?;
    let bucket = api.bucket(&namespace, &name);
    let info = bucket.info().await?;

    if args.quiet {
        Ok(CommandResult::Raw(info.id.clone()))
    } else {
        let json = serde_json::to_string_pretty(&info)?;
        Ok(CommandResult::Raw(json))
    }
}
```

- [ ] **Step 5: Create delete.rs**

Create `huggingface_hub/src/bin/hfrs/commands/buckets/delete.rs`:

```rust
use std::io::{self, Write};

use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::HFClient;

use super::parse_bucket_id;
use crate::output::CommandResult;

/// Delete a bucket
#[derive(ClapArgs)]
pub struct Args {
    /// Bucket ID (namespace/name or hf://buckets/namespace/name)
    pub bucket_id: String,

    /// Skip confirmation prompt
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// Do not fail if the bucket does not exist
    #[arg(long)]
    pub missing_ok: bool,

    /// Print only the bucket ID
    #[arg(short, long)]
    pub quiet: bool,
}

pub async fn execute(api: &HFClient, args: Args) -> Result<CommandResult> {
    let (namespace, name) = parse_bucket_id(&args.bucket_id)?;
    let bucket_id = format!("{}/{}", namespace, name);

    if !args.yes {
        eprint!("Are you sure you want to delete bucket '{bucket_id}'? [y/N] ");
        io::stderr().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            return Ok(CommandResult::Raw("Aborted.".to_string()));
        }
    }

    api.delete_bucket(&bucket_id, args.missing_ok).await?;

    if args.quiet {
        Ok(CommandResult::Raw(bucket_id))
    } else {
        Ok(CommandResult::Raw(format!("Bucket deleted: {bucket_id}")))
    }
}
```

- [ ] **Step 6: Create move_bucket.rs**

Create `huggingface_hub/src/bin/hfrs/commands/buckets/move_bucket.rs`:

```rust
use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::HFClient;

use super::parse_bucket_id;
use crate::output::CommandResult;

/// Move (rename) a bucket
#[derive(ClapArgs)]
pub struct Args {
    /// Source bucket ID
    pub from_id: String,

    /// Destination bucket ID
    pub to_id: String,
}

pub async fn execute(api: &HFClient, args: Args) -> Result<CommandResult> {
    let (from_ns, from_name) = parse_bucket_id(&args.from_id)?;
    let (to_ns, to_name) = parse_bucket_id(&args.to_id)?;
    let from = format!("{}/{}", from_ns, from_name);
    let to = format!("{}/{}", to_ns, to_name);

    api.move_bucket(&from, &to).await?;

    Ok(CommandResult::Raw(format!("Bucket moved: {from} -> {to}")))
}
```

- [ ] **Step 7: Register buckets in CLI**

In `huggingface_hub/src/bin/hfrs/commands/mod.rs`, add:

```rust
pub mod buckets;
```

In `huggingface_hub/src/bin/hfrs/cli.rs`, add to the `Command` enum:

```rust
/// Interact with buckets on the Hub
Buckets(crate::commands::buckets::Args),
```

In `huggingface_hub/src/bin/hfrs/main.rs`, add to the match in the main function:

```rust
Command::Buckets(args) => commands::buckets::execute(&api, args).await,
```

Also add format arms for the new error variants if not already done in Task 1 — check that `BucketNotFound`, `Forbidden`, `Conflict`, `RateLimited` are handled in `format_hf_error`.

- [ ] **Step 8: Verify compilation**

Run: `cargo clippy -p huggingface-hub --all-features -- -D warnings`
Expected: Clean compilation.

- [ ] **Step 9: Commit**

```bash
git add huggingface_hub/src/bin/hfrs/commands/buckets/ huggingface_hub/src/bin/hfrs/commands/mod.rs huggingface_hub/src/bin/hfrs/cli.rs huggingface_hub/src/bin/hfrs/main.rs
git commit -m "feat: add hfrs buckets CLI scaffolding with create, info, delete, move"
```

---

### Task 10: CLI list command

**Files:**
- Create: `huggingface_hub/src/bin/hfrs/commands/buckets/list.rs`
- Modify: `huggingface_hub/src/bin/hfrs/commands/buckets/mod.rs`

This is the most complex CLI command — it's overloaded to list buckets or list files.

- [ ] **Step 1: Create list.rs**

Create `huggingface_hub/src/bin/hfrs/commands/buckets/list.rs`:

```rust
use anyhow::Result;
use clap::Args as ClapArgs;
use futures::StreamExt;
use huggingface_hub::{BucketTreeEntry, HFClient, ListBucketTreeParams};

use crate::cli::OutputFormat;
use crate::output::{CommandOutput, CommandResult};

/// List buckets or files in a bucket
#[derive(ClapArgs)]
pub struct Args {
    /// Namespace (to list buckets) or bucket_id[/prefix] (to list files)
    pub argument: String,

    /// Show sizes in human-readable format
    #[arg(short = 'h', long)]
    pub human_readable: bool,

    /// Display files in tree format
    #[arg(long)]
    pub tree: bool,

    /// List files recursively
    #[arg(short = 'R', long)]
    pub recursive: bool,

    /// Output format
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,

    /// Print only paths, one per line
    #[arg(short, long)]
    pub quiet: bool,
}

/// Determine whether the argument refers to a namespace (list buckets)
/// or a bucket_id (list files). Heuristic: if it contains exactly one `/`
/// and no prefix after bucket name, or has `hf://buckets/` prefix → list files.
fn parse_list_argument(input: &str) -> ListTarget {
    let id = input.strip_prefix("hf://buckets/").unwrap_or(input);

    // If no `/` → namespace
    let Some(first_slash) = id.find('/') else {
        return ListTarget::Namespace(id.to_string());
    };

    let namespace = &id[..first_slash];
    let rest = &id[first_slash + 1..];

    // rest might be "bucket_name" or "bucket_name/prefix"
    if let Some(slash_pos) = rest.find('/') {
        let bucket_name = &rest[..slash_pos];
        let prefix = &rest[slash_pos + 1..];
        ListTarget::Files {
            namespace: namespace.to_string(),
            bucket_name: bucket_name.to_string(),
            prefix: if prefix.is_empty() {
                None
            } else {
                Some(prefix.to_string())
            },
        }
    } else {
        ListTarget::Files {
            namespace: namespace.to_string(),
            bucket_name: rest.to_string(),
            prefix: None,
        }
    }
}

enum ListTarget {
    Namespace(String),
    Files {
        namespace: String,
        bucket_name: String,
        prefix: Option<String>,
    },
}

pub async fn execute(api: &HFClient, args: Args) -> Result<CommandResult> {
    match parse_list_argument(&args.argument) {
        ListTarget::Namespace(namespace) => {
            if args.tree || args.recursive {
                anyhow::bail!("--tree and --recursive are only valid when listing files in a bucket");
            }
            list_buckets(api, &namespace, args.format, args.quiet, args.human_readable).await
        },
        ListTarget::Files {
            namespace,
            bucket_name,
            prefix,
        } => {
            if args.tree && matches!(args.format, OutputFormat::Json) {
                anyhow::bail!("--tree cannot be used with --format json");
            }
            list_files(api, &namespace, &bucket_name, prefix, &args).await
        },
    }
}

async fn list_buckets(
    api: &HFClient,
    namespace: &str,
    format: OutputFormat,
    quiet: bool,
    human_readable: bool,
) -> Result<CommandResult> {
    let stream = api.list_buckets(namespace)?;
    futures::pin_mut!(stream);

    let mut buckets = Vec::new();
    while let Some(bucket) = stream.next().await {
        buckets.push(bucket?);
    }

    let json_value = serde_json::to_value(&buckets)?;
    let headers = vec![
        "id".to_string(),
        "private".to_string(),
        "size".to_string(),
        "total_files".to_string(),
        "created_at".to_string(),
    ];
    let rows: Vec<Vec<String>> = buckets
        .iter()
        .map(|b| {
            vec![
                b.id.clone(),
                b.private.to_string(),
                if human_readable {
                    format_size_human(b.size)
                } else {
                    b.size.to_string()
                },
                b.total_files.to_string(),
                b.created_at.clone(),
            ]
        })
        .collect();
    let quiet_values = buckets.iter().map(|b| b.id.clone()).collect();

    Ok(CommandResult::Formatted {
        output: CommandOutput {
            headers,
            rows,
            json_value,
            quiet_values,
        },
        format,
        quiet,
    })
}

async fn list_files(
    api: &HFClient,
    namespace: &str,
    bucket_name: &str,
    prefix: Option<String>,
    args: &Args,
) -> Result<CommandResult> {
    let bucket = api.bucket(namespace, bucket_name);
    let params = ListBucketTreeParams::builder()
        .prefix(prefix)
        .recursive(if args.recursive { Some(true) } else { None })
        .build();

    let stream = bucket.list_tree(&params)?;
    futures::pin_mut!(stream);

    let mut entries = Vec::new();
    while let Some(entry) = stream.next().await {
        entries.push(entry?);
    }

    if args.tree {
        let tree_output = format_tree(&entries, args.human_readable);
        return Ok(CommandResult::Raw(tree_output));
    }

    if args.quiet {
        let lines: Vec<String> = entries
            .iter()
            .map(|e| match e {
                BucketTreeEntry::File { path, .. } => path.clone(),
                BucketTreeEntry::Directory { path, .. } => format!("{path}/"),
            })
            .collect();
        return Ok(CommandResult::Raw(lines.join("\n")));
    }

    let json_value = serde_json::to_value(&entries)?;

    if matches!(args.format, OutputFormat::Json) {
        return Ok(CommandResult::Raw(serde_json::to_string_pretty(&json_value)?));
    }

    // Table format: SIZE  DATE  PATH
    let lines: Vec<String> = entries
        .iter()
        .map(|e| match e {
            BucketTreeEntry::File {
                path,
                size,
                mtime,
                ..
            } => {
                let size_str = if args.human_readable {
                    format_size_human(*size)
                } else {
                    size.to_string()
                };
                let date_str = mtime.as_deref().unwrap_or("-");
                format!("{size_str:>10}  {date_str}  {path}")
            },
            BucketTreeEntry::Directory { path, .. } => {
                format!("{:>10}  {}  {path}/", "-", "-")
            },
        })
        .collect();

    Ok(CommandResult::Raw(lines.join("\n")))
}

fn format_tree(entries: &[BucketTreeEntry], _human_readable: bool) -> String {
    let mut lines = Vec::new();
    let total = entries.len();
    for (i, entry) in entries.iter().enumerate() {
        let is_last = i == total - 1;
        let connector = if is_last { "\u{2514}\u{2500}\u{2500}" } else { "\u{251c}\u{2500}\u{2500}" };
        let name = match entry {
            BucketTreeEntry::File { path, .. } => path.clone(),
            BucketTreeEntry::Directory { path, .. } => format!("{path}/"),
        };
        lines.push(format!("{connector} {name}"));
    }
    lines.join("\n")
}

fn format_size_human(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    for unit in UNITS {
        if size < 1024.0 {
            return if size.fract() == 0.0 {
                format!("{:.0}{unit}", size)
            } else {
                format!("{:.1}{unit}", size)
            };
        }
        size /= 1024.0;
    }
    format!("{:.1}PB", size)
}
```

- [ ] **Step 2: Register list in mod.rs**

In `huggingface_hub/src/bin/hfrs/commands/buckets/mod.rs`, add:

```rust
pub mod list;
```

And add to `BucketsCommand`:

```rust
/// List buckets or files in a bucket
#[command(alias = "ls")]
List(list::Args),
```

And add to the match in `execute`:

```rust
BucketsCommand::List(a) => list::execute(api, a).await,
```

- [ ] **Step 3: Verify compilation**

Run: `cargo clippy -p huggingface-hub --all-features -- -D warnings`
Expected: Clean compilation.

- [ ] **Step 4: Commit**

```bash
git add huggingface_hub/src/bin/hfrs/commands/buckets/list.rs huggingface_hub/src/bin/hfrs/commands/buckets/mod.rs
git commit -m "feat: add hfrs buckets list/ls command"
```

---

### Task 11: CLI remove command

**Files:**
- Create: `huggingface_hub/src/bin/hfrs/commands/buckets/remove.rs`
- Modify: `huggingface_hub/src/bin/hfrs/commands/buckets/mod.rs`

- [ ] **Step 1: Create remove.rs**

Create `huggingface_hub/src/bin/hfrs/commands/buckets/remove.rs`:

```rust
use std::io::{self, Write};

use anyhow::Result;
use clap::Args as ClapArgs;
use futures::StreamExt;
use huggingface_hub::{BucketTreeEntry, HFClient, ListBucketTreeParams};

use crate::output::CommandResult;

/// Remove files from a bucket
#[derive(ClapArgs)]
pub struct Args {
    /// Bucket path (namespace/bucket_name/path or hf://buckets/namespace/bucket_name/path)
    pub argument: String,

    /// Remove files recursively under the given prefix
    #[arg(short = 'R', long)]
    pub recursive: bool,

    /// Skip confirmation prompt
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// Preview deletions without actually deleting
    #[arg(long)]
    pub dry_run: bool,

    /// Include only files matching pattern(s) (requires --recursive)
    #[arg(long)]
    pub include: Vec<String>,

    /// Exclude files matching pattern(s) (requires --recursive)
    #[arg(long)]
    pub exclude: Vec<String>,

    /// Print only file paths
    #[arg(short, long)]
    pub quiet: bool,
}

/// Parse bucket path argument into (namespace, bucket_name, path_prefix).
fn parse_bucket_path(input: &str) -> Result<(String, String, Option<String>)> {
    let id = input.strip_prefix("hf://buckets/").unwrap_or(input);

    let parts: Vec<&str> = id.splitn(3, '/').collect();
    match parts.len() {
        2 => Ok((parts[0].to_string(), parts[1].to_string(), None)),
        3 => Ok((
            parts[0].to_string(),
            parts[1].to_string(),
            if parts[2].is_empty() {
                None
            } else {
                Some(parts[2].to_string())
            },
        )),
        _ => anyhow::bail!(
            "Invalid bucket path '{input}'. Expected: namespace/bucket_name/path"
        ),
    }
}

pub async fn execute(api: &HFClient, args: Args) -> Result<CommandResult> {
    if (!args.include.is_empty() || !args.exclude.is_empty()) && !args.recursive {
        anyhow::bail!("--include and --exclude require --recursive");
    }

    let (namespace, bucket_name, path_prefix) = parse_bucket_path(&args.argument)?;

    if path_prefix.is_none() && !args.recursive {
        anyhow::bail!("Must specify a file path, or use --recursive to delete all files under a prefix");
    }

    let bucket = api.bucket(&namespace, &bucket_name);
    let bucket_id = format!("{namespace}/{bucket_name}");

    let paths_to_delete = if args.recursive {
        let params = ListBucketTreeParams::builder()
            .prefix(path_prefix.clone())
            .recursive(Some(true))
            .build();
        let stream = bucket.list_tree(&params)?;
        futures::pin_mut!(stream);

        let mut paths = Vec::new();
        while let Some(entry) = stream.next().await {
            let entry = entry?;
            if let BucketTreeEntry::File { ref path, .. } = entry {
                let include_match = args.include.is_empty()
                    || args.include.iter().any(|pat| {
                        globset::Glob::new(pat)
                            .ok()
                            .and_then(|g| g.compile_matcher().is_match(path).then_some(()))
                            .is_some()
                    });
                let exclude_match = args.exclude.iter().any(|pat| {
                    globset::Glob::new(pat)
                        .ok()
                        .and_then(|g| g.compile_matcher().is_match(path).then_some(()))
                        .is_some()
                });
                if include_match && !exclude_match {
                    paths.push(path.clone());
                }
            }
        }
        paths
    } else {
        vec![path_prefix.unwrap()]
    };

    if paths_to_delete.is_empty() {
        return Ok(CommandResult::Raw("No files to delete.".to_string()));
    }

    // Preview
    for path in &paths_to_delete {
        if args.quiet {
            println!("{path}");
        } else {
            println!("delete: hf://buckets/{bucket_id}/{path}");
        }
    }

    if args.dry_run {
        let count = paths_to_delete.len();
        return Ok(CommandResult::Raw(format!(
            "(dry run) {count} file(s) would be removed."
        )));
    }

    // Confirm
    if !args.yes {
        let count = paths_to_delete.len();
        eprint!("Delete {count} file(s)? [y/N] ");
        io::stderr().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            return Ok(CommandResult::Raw("Aborted.".to_string()));
        }
    }

    bucket.delete_files(&paths_to_delete).await?;

    let count = paths_to_delete.len();
    Ok(CommandResult::Raw(format!("{count} file(s) removed.")))
}
```

- [ ] **Step 2: Register remove in mod.rs**

In `huggingface_hub/src/bin/hfrs/commands/buckets/mod.rs`, add:

```rust
pub mod remove;
```

And add to `BucketsCommand`:

```rust
/// Remove files from a bucket
#[command(alias = "rm")]
Remove(remove::Args),
```

And add to the match in `execute`:

```rust
BucketsCommand::Remove(a) => remove::execute(api, a).await,
```

- [ ] **Step 3: Verify compilation**

Run: `cargo clippy -p huggingface-hub --all-features -- -D warnings`
Expected: Clean compilation.

- [ ] **Step 4: Commit**

```bash
git add huggingface_hub/src/bin/hfrs/commands/buckets/remove.rs huggingface_hub/src/bin/hfrs/commands/buckets/mod.rs
git commit -m "feat: add hfrs buckets remove/rm command"
```

---

### Task 12: CLI cp command

**Files:**
- Create: `huggingface_hub/src/bin/hfrs/commands/buckets/cp.rs`
- Modify: `huggingface_hub/src/bin/hfrs/commands/buckets/mod.rs`

- [ ] **Step 1: Create cp.rs**

Create `huggingface_hub/src/bin/hfrs/commands/buckets/cp.rs`:

```rust
use std::io::{self, Read, Write};
use std::path::PathBuf;

use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{BucketDownloadFilesParams, HFClient};

use crate::output::CommandResult;

/// Copy files to/from a bucket
#[derive(ClapArgs)]
pub struct Args {
    /// Source: local path, hf://buckets/ns/name/path, or - for stdin
    pub src: String,

    /// Destination: local path, hf://buckets/ns/name/path, or - for stdout.
    /// Defaults to current directory with source filename.
    pub dst: Option<String>,

    /// Suppress output
    #[arg(short, long)]
    pub quiet: bool,
}

struct BucketPath {
    namespace: String,
    bucket_name: String,
    path: String,
}

fn parse_bucket_path(input: &str) -> Option<BucketPath> {
    let rest = input.strip_prefix("hf://buckets/")?;
    let parts: Vec<&str> = rest.splitn(3, '/').collect();
    if parts.len() < 3 || parts[2].is_empty() {
        return None;
    }
    Some(BucketPath {
        namespace: parts[0].to_string(),
        bucket_name: parts[1].to_string(),
        path: parts[2].to_string(),
    })
}

fn filename_from_path(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

pub async fn execute(api: &HFClient, args: Args) -> Result<CommandResult> {
    let src_is_stdin = args.src == "-";
    let src_is_bucket = args.src.starts_with("hf://buckets/");
    let dst_str = args.dst.clone().unwrap_or_else(|| ".".to_string());
    let dst_is_stdout = dst_str == "-";
    let dst_is_bucket = dst_str.starts_with("hf://buckets/");

    if !src_is_bucket && !dst_is_bucket && !src_is_stdin && !dst_is_stdout {
        anyhow::bail!("At least one of source or destination must be a bucket path (hf://buckets/...)");
    }

    // Local to bucket upload
    if !src_is_bucket && !src_is_stdin && dst_is_bucket {
        let dst = parse_bucket_path(&dst_str)
            .ok_or_else(|| anyhow::anyhow!("Invalid bucket destination: {}", dst_str))?;

        let local_path = PathBuf::from(&args.src);
        if !local_path.exists() {
            anyhow::bail!("Source file not found: {}", args.src);
        }

        let bucket = api.bucket(&dst.namespace, &dst.bucket_name);
        bucket
            .upload_files(&[(local_path, dst.path.clone())])
            .await?;

        if !args.quiet {
            return Ok(CommandResult::Raw(format!(
                "Uploaded: {} -> hf://buckets/{}/{}/{}",
                args.src, dst.namespace, dst.bucket_name, dst.path
            )));
        }
        return Ok(CommandResult::Silent);
    }

    // Stdin to bucket upload
    if src_is_stdin && dst_is_bucket {
        let dst = parse_bucket_path(&dst_str)
            .ok_or_else(|| anyhow::anyhow!("Invalid bucket destination: {}", dst_str))?;

        let mut data = Vec::new();
        io::stdin().read_to_end(&mut data)?;

        let tmp = tempfile::NamedTempFile::new()?;
        std::fs::write(tmp.path(), &data)?;

        let bucket = api.bucket(&dst.namespace, &dst.bucket_name);
        bucket
            .upload_files(&[(tmp.path().to_path_buf(), dst.path.clone())])
            .await?;

        if !args.quiet {
            return Ok(CommandResult::Raw(format!(
                "Uploaded: (stdin) -> hf://buckets/{}/{}/{}",
                dst.namespace, dst.bucket_name, dst.path
            )));
        }
        return Ok(CommandResult::Silent);
    }

    // Bucket to local download
    if src_is_bucket && !dst_is_bucket && !dst_is_stdout {
        let src = parse_bucket_path(&args.src)
            .ok_or_else(|| anyhow::anyhow!("Invalid bucket source: {}", args.src))?;

        let mut local_path = PathBuf::from(&dst_str);
        if local_path.is_dir() {
            local_path = local_path.join(filename_from_path(&src.path));
        }

        if let Some(parent) = local_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let bucket = api.bucket(&src.namespace, &src.bucket_name);
        let params = BucketDownloadFilesParams::builder()
            .files(vec![(src.path.clone(), local_path.clone())])
            .build();
        bucket.download_files(&params).await?;

        if !args.quiet {
            return Ok(CommandResult::Raw(format!(
                "Downloaded: hf://buckets/{}/{}/{} -> {}",
                src.namespace,
                src.bucket_name,
                src.path,
                local_path.display()
            )));
        }
        return Ok(CommandResult::Silent);
    }

    // Bucket to stdout
    if src_is_bucket && dst_is_stdout {
        let src = parse_bucket_path(&args.src)
            .ok_or_else(|| anyhow::anyhow!("Invalid bucket source: {}", args.src))?;

        let tmp = tempfile::NamedTempFile::new()?;
        let bucket = api.bucket(&src.namespace, &src.bucket_name);
        let params = BucketDownloadFilesParams::builder()
            .files(vec![(src.path.clone(), tmp.path().to_path_buf())])
            .build();
        bucket.download_files(&params).await?;

        let data = std::fs::read(tmp.path())?;
        io::stdout().write_all(&data)?;
        return Ok(CommandResult::Silent);
    }

    // Bucket to bucket (server-side copy)
    if src_is_bucket && dst_is_bucket {
        let src = parse_bucket_path(&args.src)
            .ok_or_else(|| anyhow::anyhow!("Invalid bucket source: {}", args.src))?;
        let dst = parse_bucket_path(&dst_str)
            .ok_or_else(|| anyhow::anyhow!("Invalid bucket destination: {}", dst_str))?;

        let src_bucket = api.bucket(&src.namespace, &src.bucket_name);
        let metadata = src_bucket.get_file_metadata(&src.path).await?;

        let dst_bucket = api.bucket(&dst.namespace, &dst.bucket_name);
        let copy_params = huggingface_hub::BatchBucketFilesParams {
            copy: vec![huggingface_hub::BucketCopyFile {
                path: dst.path.clone(),
                xet_hash: metadata.xet_hash,
                source_repo_type: "bucket".to_string(),
                source_repo_id: format!("{}/{}", src.namespace, src.bucket_name),
            }],
            ..Default::default()
        };
        dst_bucket.batch(&copy_params).await?;

        if !args.quiet {
            return Ok(CommandResult::Raw(format!(
                "Copied: hf://buckets/{}/{}/{} -> hf://buckets/{}/{}/{}",
                src.namespace, src.bucket_name, src.path, dst.namespace, dst.bucket_name, dst.path
            )));
        }
        return Ok(CommandResult::Silent);
    }

    anyhow::bail!("Unsupported copy operation: {} -> {}", args.src, dst_str)
}
```

- [ ] **Step 2: Add tempfile dependency**

Add `tempfile` as an optional dependency in `huggingface_hub/Cargo.toml` dependencies section:

```toml
tempfile = { version = "3", optional = true }
```

And add to the `cli` feature:

```toml
cli = [
    "blocking",
    "xet",
    "spaces",
    "tokio/macros",
    "tokio/rt-multi-thread",
    "dep:clap",
    "dep:owo-colors",
    "dep:comfy-table",
    "dep:anyhow",
    "dep:tracing-subscriber",
    "dep:tempfile",
]
```

`tempfile` is already in `[dev-dependencies]` — add it as an optional `[dependencies]` entry for the CLI binary. This is a lightweight, well-maintained crate.

- [ ] **Step 3: Register cp in mod.rs**

In `huggingface_hub/src/bin/hfrs/commands/buckets/mod.rs`, add:

```rust
pub mod cp;
```

And add to `BucketsCommand`:

```rust
/// Copy files to/from a bucket
Cp(cp::Args),
```

And add to the match in `execute`:

```rust
BucketsCommand::Cp(a) => cp::execute(api, a).await,
```

- [ ] **Step 4: Verify compilation**

Run: `cargo clippy -p huggingface-hub --all-features -- -D warnings`
Expected: Clean compilation.

- [ ] **Step 5: Commit**

```bash
git add huggingface_hub/src/bin/hfrs/commands/buckets/cp.rs huggingface_hub/src/bin/hfrs/commands/buckets/mod.rs huggingface_hub/Cargo.toml
git commit -m "feat: add hfrs buckets cp command"
```

---

### Task 13: Format, lint, and final verification

**Files:**
- All files modified in Tasks 1-12

- [ ] **Step 1: Format**

Run: `cargo +nightly fmt`

- [ ] **Step 2: Lint**

Run: `cargo clippy -p huggingface-hub --all-features -- -D warnings`
Fix any warnings.

- [ ] **Step 3: Run all tests**

Run: `cargo test -p huggingface-hub`
Expected: All tests pass.

- [ ] **Step 4: Build the CLI**

Run: `cargo build -p huggingface-hub --release --features cli`
Expected: Clean build, `hfrs` binary produced.

- [ ] **Step 5: Verify CLI help**

Run: `./target/release/hfrs buckets --help`
Expected: Shows all bucket subcommands (create, list/ls, info, delete, remove/rm, move, cp).

Run: `./target/release/hfrs buckets create --help`
Expected: Shows create arguments and flags.

- [ ] **Step 6: Commit any formatting/lint fixes**

```bash
git add -A
git commit -m "chore: format and lint bucket implementation"
```
