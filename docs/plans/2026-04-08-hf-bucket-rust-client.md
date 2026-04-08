# HFBucket Rust Client Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an `HFBucket` type and supporting infrastructure to `huggingface_hub_rust` that exposes the full HuggingFace Storage Buckets API as a typed async Rust client.

**Architecture:** `HFBucket` is a standalone handle type (following the `HFSpace` precedent) holding an `HFClient` reference plus namespace and repo strings. Bucket methods are implemented in `api/buckets.rs`; types live in `types/buckets.rs`. A private `check_bucket_response` helper maps HTTP status codes — including four new `HFError` variants — for all bucket endpoints.

**Tech Stack:** Rust, `reqwest` 0.13, `serde`/`serde_json`, `typed-builder`, `futures` (`try_unfold`), `tokio`

**Spec:** `docs/specs/2026-04-08-hf-bucket-rust-client-design.md` in `huggingface/xet-catalogue`  
**Target repo:** `/Users/jgodlew/git/huggingface/huggingface_hub_rust/`

---

## File Map

| Action | Path |
|--------|------|
| Create | `huggingface_hub/src/types/buckets.rs` |
| Create | `huggingface_hub/src/api/buckets.rs` |
| Modify | `huggingface_hub/src/error.rs` — add 4 new `HFError` variants |
| Modify | `huggingface_hub/src/types/mod.rs` — add `pub mod buckets; pub use buckets::*;` |
| Modify | `huggingface_hub/src/api/mod.rs` — add `pub mod buckets;` |
| Modify | `huggingface_hub/src/repository.rs` — add `HFBucket` struct |
| Modify | `huggingface_hub/src/client.rs` — add `bucket()`, `create_bucket()`, `list_buckets()` |
| Modify | `huggingface_hub/src/lib.rs` — export `HFBucket` and `HFBucketSync` |
| Modify | `huggingface_hub/src/blocking.rs` — add `HFBucketSync` and blocking wrappers |
| Modify | `huggingface_hub/tests/integration_test.rs` — add integration tests |

---

## Task 1: Add new `HFError` variants

**Files:**
- Modify: `huggingface_hub/src/error.rs`

- [ ] **Step 1: Write a failing test that matches on the new variants**

Add to the bottom of `huggingface_hub/src/error.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_error_variants_display() {
        assert_eq!(HFError::Forbidden.to_string(), "forbidden");
        assert_eq!(
            HFError::Conflict("name taken".to_string()).to_string(),
            "conflict: name taken"
        );
        assert_eq!(HFError::RateLimited.to_string(), "rate limited");
        assert_eq!(HFError::QuotaExceeded.to_string(), "quota exceeded");
    }
}
```

- [ ] **Step 2: Run the test to confirm it fails**

```bash
cd /Users/jgodlew/git/huggingface/huggingface_hub_rust
cargo test -p huggingface_hub new_error_variants_display 2>&1
```

Expected: compile error — `HFError::Forbidden` does not exist.

- [ ] **Step 3: Add the four variants to `HFError`**

In `huggingface_hub/src/error.rs`, locate the `HFError` enum and add after the last existing variant (before the closing `}`):

```rust
    #[error("forbidden")]
    Forbidden,
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("rate limited")]
    RateLimited,
    #[error("quota exceeded")]
    QuotaExceeded,
```

- [ ] **Step 4: Run the test to confirm it passes**

```bash
cargo test -p huggingface_hub new_error_variants_display 2>&1
```

Expected: `test error::tests::new_error_variants_display ... ok`

- [ ] **Step 5: Commit**

```bash
cd /Users/jgodlew/git/huggingface/huggingface_hub_rust
git add huggingface_hub/src/error.rs
git commit -m "feat(error): add Forbidden, Conflict, RateLimited, QuotaExceeded variants"
```

---

## Task 2: Create `types/buckets.rs` and wire it in

**Files:**
- Create: `huggingface_hub/src/types/buckets.rs`
- Modify: `huggingface_hub/src/types/mod.rs`

- [ ] **Step 1: Write a failing test for type deserialization**

Create `huggingface_hub/src/types/buckets.rs` with only the test module:

```rust
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
```

- [ ] **Step 2: Run the test to confirm it fails**

```bash
cargo test -p huggingface_hub bucket_info_deserializes 2>&1
```

Expected: compile error — `BucketInfo` not found.

- [ ] **Step 3: Add all types above the test module in `types/buckets.rs`**

Replace the contents of `huggingface_hub/src/types/buckets.rs` with:

```rust
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
```

- [ ] **Step 4: Wire into `types/mod.rs`**

In `huggingface_hub/src/types/mod.rs`, add alongside the existing module declarations:

```rust
pub mod buckets;
```

And add to the re-exports at the bottom:

```rust
pub use buckets::*;
```

- [ ] **Step 5: Run the tests to confirm they pass**

```bash
cargo test -p huggingface_hub bucket_info_deserializes batch_op_serializes tree_entry_deserializes bucket_overview_deserializes delete_op_serializes 2>&1
```

Expected: all 5 tests pass.

- [ ] **Step 6: Commit**

```bash
git add huggingface_hub/src/types/buckets.rs huggingface_hub/src/types/mod.rs
git commit -m "feat(types): add bucket types (BucketInfo, BucketOverview, BatchOp, TreeEntry, etc.)"
```

---

## Task 3: Add `HFBucket` struct and wire up modules

**Files:**
- Modify: `huggingface_hub/src/repository.rs`
- Create (skeleton): `huggingface_hub/src/api/buckets.rs`
- Modify: `huggingface_hub/src/api/mod.rs`
- Modify: `huggingface_hub/src/client.rs`
- Modify: `huggingface_hub/src/lib.rs`

- [ ] **Step 1: Write a failing test for `HFClient::bucket()` constructor**

In `huggingface_hub/src/api/buckets.rs` (new file, skeleton only for now):

```rust
#[cfg(test)]
mod tests {
    use crate::HFClientBuilder;

    #[test]
    fn bucket_constructor_sets_namespace_and_repo() {
        let client = HFClientBuilder::new().build().unwrap();
        let bucket = client.bucket("myuser", "my-bucket");
        assert_eq!(bucket.namespace, "myuser");
        assert_eq!(bucket.repo, "my-bucket");
    }
}
```

- [ ] **Step 2: Run the test to confirm it fails**

```bash
cargo test -p huggingface_hub bucket_constructor_sets_namespace_and_repo 2>&1
```

Expected: compile error — `client.bucket` does not exist.

- [ ] **Step 3: Add `HFBucket` struct to `repository.rs`**

In `huggingface_hub/src/repository.rs`, add after the `HFSpace` struct definition (or after `HFRepository`, following the same pattern):

```rust
/// Handle for operations on a single HuggingFace Storage Bucket.
///
/// Obtain via [`HFClient::bucket`]. Every method adds `Authorization: Bearer <token>`
/// using the token configured on the client.
#[derive(Clone)]
pub struct HFBucket {
    pub(crate) client: crate::HFClient,
    pub namespace: String,
    pub repo: String,
}
```

- [ ] **Step 4: Add `HFClient::bucket()` to `client.rs`**

In `huggingface_hub/src/client.rs`, add alongside the existing `model()`, `dataset()`, `space()` methods:

```rust
/// Creates a handle for operations on a single Storage Bucket.
/// No I/O is performed.
pub fn bucket(&self, namespace: impl Into<String>, repo: impl Into<String>) -> crate::repository::HFBucket {
    crate::repository::HFBucket {
        client: self.clone(),
        namespace: namespace.into(),
        repo: repo.into(),
    }
}
```

- [ ] **Step 5: Wire `api/buckets.rs` into `api/mod.rs`**

In `huggingface_hub/src/api/mod.rs`, add:

```rust
pub mod buckets;
```

- [ ] **Step 6: Export `HFBucket` from `lib.rs`**

In `huggingface_hub/src/lib.rs`, ensure `HFBucket` is included in the `repository` re-export. It will be exported automatically if `lib.rs` already has `pub use repository::*;`. Verify this line exists; if not, add it.

- [ ] **Step 7: Run the test to confirm it passes**

```bash
cargo test -p huggingface_hub bucket_constructor_sets_namespace_and_repo 2>&1
```

Expected: `test api::buckets::tests::bucket_constructor_sets_namespace_and_repo ... ok`

- [ ] **Step 8: Commit**

```bash
git add huggingface_hub/src/repository.rs huggingface_hub/src/api/buckets.rs \
        huggingface_hub/src/api/mod.rs huggingface_hub/src/client.rs \
        huggingface_hub/src/lib.rs
git commit -m "feat(bucket): add HFBucket struct and client.bucket() constructor"
```

---

## Task 4: Bucket CRUD — `get`, `delete`, `update_settings`

**Files:**
- Modify: `huggingface_hub/src/api/buckets.rs`

- [ ] **Step 1: Write failing tests**

Add to the `tests` module in `huggingface_hub/src/api/buckets.rs`:

```rust
#[cfg(test)]
mod tests {
    use crate::HFClientBuilder;

    #[test]
    fn bucket_constructor_sets_namespace_and_repo() {
        let client = HFClientBuilder::new().build().unwrap();
        let bucket = client.bucket("myuser", "my-bucket");
        assert_eq!(bucket.namespace, "myuser");
        assert_eq!(bucket.repo, "my-bucket");
    }

    #[test]
    fn get_bucket_url() {
        let client = HFClientBuilder::new().build().unwrap();
        let bucket = client.bucket("myuser", "my-bucket");
        let url = format!(
            "{}/api/buckets/{}/{}",
            bucket.client.inner.endpoint, bucket.namespace, bucket.repo
        );
        assert!(url.ends_with("/api/buckets/myuser/my-bucket"));
    }

    #[test]
    fn update_settings_url() {
        let client = HFClientBuilder::new().build().unwrap();
        let bucket = client.bucket("myuser", "my-bucket");
        let url = format!(
            "{}/api/buckets/{}/{}/settings",
            bucket.client.inner.endpoint, bucket.namespace, bucket.repo
        );
        assert!(url.ends_with("/api/buckets/myuser/my-bucket/settings"));
    }
}
```

- [ ] **Step 2: Run the tests to confirm they fail**

```bash
cargo test -p huggingface_hub get_bucket_url update_settings_url 2>&1
```

Expected: compile error — `bucket.client.inner` not accessible.

- [ ] **Step 3: Add `check_bucket_response` helper and implement CRUD methods**

Replace `huggingface_hub/src/api/buckets.rs` with:

```rust
use std::collections::VecDeque;

use futures::{Stream, StreamExt};

use crate::error::{HFError, NotFoundContext};
use crate::repository::HFBucket;
use crate::types::{
    BatchOp, BatchResult, BucketCreated, BucketInfo, BucketOverview, CreateBucketParams,
    ListTreeParams, PathInfo, ResolvedFile, TreeEntry, TreePage, UpdateBucketParams, XetToken,
};
use crate::{HFClient, Result};

/// Maps HTTP status codes to `HFError` variants for bucket API responses.
/// Bucket-level 404s map to `RepoNotFound`; file-level 404s map to `EntryNotFound`.
async fn check_bucket_response(
    response: reqwest::Response,
    repo_id: &str,
    not_found_ctx: NotFoundContext,
) -> Result<reqwest::Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status().as_u16();
    let url = response.url().to_string();
    let body = response.text().await.unwrap_or_default();
    Err(match status {
        401 => HFError::AuthRequired,
        403 => HFError::Forbidden,
        404 => match not_found_ctx {
            NotFoundContext::Repo => HFError::RepoNotFound {
                repo_id: repo_id.to_string(),
            },
            NotFoundContext::Entry { path } => HFError::EntryNotFound {
                path,
                repo_id: repo_id.to_string(),
            },
            _ => HFError::Http { status, url, body },
        },
        409 => HFError::Conflict(body),
        429 => HFError::RateLimited,
        507 => HFError::QuotaExceeded,
        _ => HFError::Http { status, url, body },
    })
}

impl HFBucket {
    fn repo_id(&self) -> String {
        format!("{}/{}", self.namespace, self.repo)
    }

    fn bucket_url(&self) -> String {
        format!(
            "{}/api/buckets/{}/{}",
            self.client.inner.endpoint, self.namespace, self.repo
        )
    }

    /// Returns metadata about this bucket.
    pub async fn get(&self) -> Result<BucketInfo> {
        let resp = self
            .client
            .inner
            .client
            .get(self.bucket_url())
            .headers(self.client.auth_headers())
            .send()
            .await
            .map_err(HFError::Request)?;
        let resp = check_bucket_response(resp, &self.repo_id(), NotFoundContext::Repo).await?;
        resp.json().await.map_err(HFError::Json)
    }

    /// Permanently deletes this bucket and all its files.
    pub async fn delete(&self) -> Result<()> {
        let resp = self
            .client
            .inner
            .client
            .delete(self.bucket_url())
            .headers(self.client.auth_headers())
            .send()
            .await
            .map_err(HFError::Request)?;
        check_bucket_response(resp, &self.repo_id(), NotFoundContext::Repo).await?;
        Ok(())
    }

    /// Updates visibility or CDN configuration for this bucket.
    pub async fn update_settings(&self, params: UpdateBucketParams) -> Result<()> {
        let resp = self
            .client
            .inner
            .client
            .put(format!("{}/settings", self.bucket_url()))
            .headers(self.client.auth_headers())
            .json(&params)
            .send()
            .await
            .map_err(HFError::Request)?;
        check_bucket_response(resp, &self.repo_id(), NotFoundContext::Repo).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::HFClientBuilder;

    #[test]
    fn bucket_constructor_sets_namespace_and_repo() {
        let client = HFClientBuilder::new().build().unwrap();
        let bucket = client.bucket("myuser", "my-bucket");
        assert_eq!(bucket.namespace, "myuser");
        assert_eq!(bucket.repo, "my-bucket");
    }

    #[test]
    fn get_bucket_url() {
        let client = HFClientBuilder::new().build().unwrap();
        let bucket = client.bucket("myuser", "my-bucket");
        let url = format!(
            "{}/api/buckets/{}/{}",
            bucket.client.inner.endpoint, bucket.namespace, bucket.repo
        );
        assert!(url.ends_with("/api/buckets/myuser/my-bucket"));
    }

    #[test]
    fn update_settings_url() {
        let client = HFClientBuilder::new().build().unwrap();
        let bucket = client.bucket("myuser", "my-bucket");
        let url = format!(
            "{}/api/buckets/{}/{}/settings",
            bucket.client.inner.endpoint, bucket.namespace, bucket.repo
        );
        assert!(url.ends_with("/api/buckets/myuser/my-bucket/settings"));
    }
}
```

- [ ] **Step 4: Run the tests to confirm they pass**

```bash
cargo test -p huggingface_hub get_bucket_url update_settings_url bucket_constructor 2>&1
```

Expected: all 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add huggingface_hub/src/api/buckets.rs
git commit -m "feat(bucket): add get, delete, update_settings with check_bucket_response helper"
```

---

## Task 5: `HFClient::create_bucket` and `HFClient::list_buckets`

**Files:**
- Modify: `huggingface_hub/src/api/buckets.rs`
- Modify: `huggingface_hub/src/client.rs`

- [ ] **Step 1: Write failing tests**

Add to the `tests` module in `api/buckets.rs`:

```rust
    #[test]
    fn create_bucket_url() {
        let client = HFClientBuilder::new().build().unwrap();
        let url = format!(
            "{}/api/buckets/{}/{}",
            client.inner.endpoint, "myuser", "new-bucket"
        );
        assert!(url.ends_with("/api/buckets/myuser/new-bucket"));
    }

    #[test]
    fn list_buckets_url() {
        let client = HFClientBuilder::new().build().unwrap();
        let url = format!("{}/api/buckets/{}", client.inner.endpoint, "myuser");
        assert!(url.ends_with("/api/buckets/myuser"));
    }
```

- [ ] **Step 2: Run the tests to confirm they fail**

```bash
cargo test -p huggingface_hub create_bucket_url list_buckets_url 2>&1
```

Expected: compile error — `client.inner` not accessible from test or methods not found.

- [ ] **Step 3: Add `create_bucket` and `list_buckets` to `client.rs`**

In `huggingface_hub/src/client.rs`, add the following imports at the top if not already present:

```rust
use url::Url;
```

Then add the new methods on `HFClient` (alongside `bucket()`):

```rust
/// Creates a new bucket owned by `namespace`.
pub async fn create_bucket(
    &self,
    namespace: &str,
    repo: &str,
    params: crate::types::CreateBucketParams,
) -> crate::Result<crate::types::BucketCreated> {
    let url = format!("{}/api/buckets/{}/{}", self.inner.endpoint, namespace, repo);
    let resp = self
        .inner
        .client
        .post(&url)
        .headers(self.auth_headers())
        .json(&params)
        .send()
        .await
        .map_err(crate::HFError::Request)?;
    let repo_id = format!("{}/{}", namespace, repo);
    let resp = crate::api::buckets::check_bucket_response(
        resp,
        &repo_id,
        crate::error::NotFoundContext::Repo,
    )
    .await?;
    resp.json().await.map_err(crate::HFError::Json)
}

/// Returns a paginated stream of all buckets owned by `namespace`.
/// Pagination is driven by `Link` response headers.
pub fn list_buckets(
    &self,
    namespace: &str,
) -> impl futures::Stream<Item = crate::Result<crate::types::BucketOverview>> + '_ {
    let url = Url::parse(&format!("{}/api/buckets/{}", self.inner.endpoint, namespace))
        .expect("endpoint is a valid base URL");
    self.paginate(url, vec![], None)
}
```

Note: `check_bucket_response` needs to be `pub(crate)` in `api/buckets.rs`. Change its visibility there:

```rust
pub(crate) async fn check_bucket_response( ... )
```

- [ ] **Step 4: Run the tests to confirm they pass**

```bash
cargo test -p huggingface_hub create_bucket_url list_buckets_url 2>&1
```

Expected: both tests pass.

- [ ] **Step 5: Commit**

```bash
git add huggingface_hub/src/api/buckets.rs huggingface_hub/src/client.rs
git commit -m "feat(bucket): add HFClient::create_bucket and list_buckets"
```

---

## Task 6: `batch_files` — NDJSON serialization

**Files:**
- Modify: `huggingface_hub/src/api/buckets.rs`

- [ ] **Step 1: Write failing tests**

Add to the `tests` module in `api/buckets.rs`:

```rust
    #[test]
    fn batch_files_ndjson_adds_before_deletes() {
        use crate::types::{AddFileOp, BatchOp, DeleteFileOp};

        let ops = vec![
            BatchOp::DeleteFile(DeleteFileOp { path: "old.parquet".to_string() }),
            BatchOp::AddFile(AddFileOp {
                path: "new.parquet".to_string(),
                xet_hash: "abc".to_string(),
                content_type: "application/octet-stream".to_string(),
                mtime: None,
            }),
        ];
        // Partition and serialize: adds must come first regardless of input order
        let (adds, deletes): (Vec<_>, Vec<_>) =
            ops.into_iter().partition(|op| matches!(op, BatchOp::AddFile(_)));
        let ndjson: String = adds
            .iter()
            .chain(deletes.iter())
            .map(|op| serde_json::to_string(op).map(|s| s + "\n"))
            .collect::<Result<_, _>>()
            .unwrap();
        let lines: Vec<&str> = ndjson.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("addFile"), "first line must be addFile, got: {}", lines[0]);
        assert!(lines[1].contains("deleteFile"), "second line must be deleteFile");
    }

    #[test]
    fn batch_files_each_line_ends_with_newline() {
        use crate::types::{AddFileOp, BatchOp};
        let ops = vec![BatchOp::AddFile(AddFileOp {
            path: "f.parquet".to_string(),
            xet_hash: "h".to_string(),
            content_type: "application/octet-stream".to_string(),
            mtime: None,
        })];
        let (adds, deletes): (Vec<_>, Vec<_>) =
            ops.into_iter().partition(|op| matches!(op, BatchOp::AddFile(_)));
        let ndjson: String = adds
            .iter()
            .chain(deletes.iter())
            .map(|op| serde_json::to_string(op).map(|s| s + "\n"))
            .collect::<Result<_, _>>()
            .unwrap();
        assert!(ndjson.ends_with('\n'));
    }
```

- [ ] **Step 2: Run the tests to confirm they pass (logic is already testable)**

```bash
cargo test -p huggingface_hub batch_files_ndjson batch_files_each_line 2>&1
```

These tests only exercise the serialization logic which uses already-present types. They should compile and pass. If they don't compile, check that `BatchOp`, `AddFileOp`, `DeleteFileOp` are in scope.

- [ ] **Step 3: Implement `batch_files` on `HFBucket`**

Add the following method to the `impl HFBucket` block in `api/buckets.rs`:

```rust
    /// Adds and/or removes files in a single atomic operation.
    ///
    /// All `AddFile` operations are sent before `DeleteFile` operations, as required
    /// by the batch protocol. The input order within each group is preserved.
    pub async fn batch_files(&self, ops: Vec<BatchOp>) -> Result<BatchResult> {
        let (adds, deletes): (Vec<_>, Vec<_>) =
            ops.into_iter().partition(|op| matches!(op, BatchOp::AddFile(_)));

        let ndjson = adds
            .iter()
            .chain(deletes.iter())
            .map(|op| serde_json::to_string(op).map(|s| s + "\n"))
            .collect::<std::result::Result<String, _>>()
            .map_err(HFError::Json)?;

        let resp = self
            .client
            .inner
            .client
            .post(format!("{}/batch", self.bucket_url()))
            .headers(self.client.auth_headers())
            .header("content-type", "application/x-ndjson")
            .body(ndjson)
            .send()
            .await
            .map_err(HFError::Request)?;

        let resp =
            check_bucket_response(resp, &self.repo_id(), NotFoundContext::Repo).await?;
        resp.json().await.map_err(HFError::Json)
    }
```

Also add `use serde_json;` at the top of `api/buckets.rs` if not already present.

- [ ] **Step 4: Run all bucket tests**

```bash
cargo test -p huggingface_hub batch_files 2>&1
```

Expected: both tests pass, no compile errors.

- [ ] **Step 5: Commit**

```bash
git add huggingface_hub/src/api/buckets.rs
git commit -m "feat(bucket): implement batch_files with NDJSON add-before-delete ordering"
```

---

## Task 7: `list_tree` — cursor-in-body streaming pagination

**Files:**
- Modify: `huggingface_hub/src/api/buckets.rs`

- [ ] **Step 1: Write a failing test for URL construction**

Add to the `tests` module in `api/buckets.rs`:

```rust
    #[test]
    fn list_tree_url_empty_path() {
        let client = HFClientBuilder::new().build().unwrap();
        let bucket = client.bucket("myuser", "my-bucket");
        let url = if "".is_empty() {
            format!(
                "{}/api/buckets/{}/{}/tree",
                bucket.client.inner.endpoint, bucket.namespace, bucket.repo
            )
        } else {
            format!(
                "{}/api/buckets/{}/{}/tree/{}",
                bucket.client.inner.endpoint, bucket.namespace, bucket.repo, "some/path"
            )
        };
        assert!(url.ends_with("/api/buckets/myuser/my-bucket/tree"));
    }

    #[test]
    fn list_tree_url_with_path() {
        let client = HFClientBuilder::new().build().unwrap();
        let bucket = client.bucket("myuser", "my-bucket");
        let path = "data/sub";
        let url = format!(
            "{}/api/buckets/{}/{}/tree/{}",
            bucket.client.inner.endpoint, bucket.namespace, bucket.repo, path
        );
        assert!(url.ends_with("/api/buckets/myuser/my-bucket/tree/data/sub"));
    }
```

- [ ] **Step 2: Run the tests to confirm they pass (URL logic is trivially testable)**

```bash
cargo test -p huggingface_hub list_tree_url 2>&1
```

Expected: both URL tests pass.

- [ ] **Step 3: Implement `list_tree` on `HFBucket`**

Add the following to the `impl HFBucket` block in `api/buckets.rs`. This uses `try_unfold` with a `VecDeque` buffer to yield one `TreeEntry` at a time while fetching pages lazily:

```rust
    /// Lists files and directories, yielding one entry at a time.
    ///
    /// Uses cursor-in-body pagination: the stream fetches the next page automatically
    /// when the current page's entries are exhausted. No request is made until the
    /// first item is polled.
    pub fn list_tree(
        &self,
        path: &str,
        params: ListTreeParams,
    ) -> impl Stream<Item = Result<TreeEntry>> + '_ {
        let base_url = if path.is_empty() {
            format!(
                "{}/api/buckets/{}/{}/tree",
                self.client.inner.endpoint, self.namespace, self.repo
            )
        } else {
            format!(
                "{}/api/buckets/{}/{}/tree/{}",
                self.client.inner.endpoint, self.namespace, self.repo, path
            )
        };
        let repo_id = self.repo_id();

        // State: (buffered entries from current page, cursor for next page, whether we've fetched at all)
        // cursor=None + fetched=false  → fetch first page (no cursor param)
        // cursor=Some(c) + fetched=_  → fetch next page with ?cursor=c
        // cursor=None + fetched=true  → no more pages, drain buffer then end
        futures::stream::try_unfold(
            (VecDeque::<TreeEntry>::new(), None::<String>, false),
            move |(mut pending, cursor, fetched)| {
                let client = self.client.clone();
                let repo_id = repo_id.clone();
                let base_url = base_url.clone();
                async move {
                    // Yield buffered items before fetching a new page
                    if let Some(entry) = pending.pop_front() {
                        return Ok(Some((entry, (pending, cursor, fetched))));
                    }
                    // No buffered items. Are there more pages to fetch?
                    if fetched && cursor.is_none() {
                        return Ok(None);
                    }
                    // Fetch next (or first) page
                    let mut req = client
                        .inner
                        .client
                        .get(&base_url)
                        .headers(client.auth_headers());
                    if let Some(ref c) = cursor {
                        req = req.query(&[("cursor", c.as_str())]);
                    }
                    if let Some(l) = params.limit {
                        req = req.query(&[("limit", l.to_string().as_str())]);
                    }
                    if params.recursive {
                        req = req.query(&[("recursive", "true")]);
                    }
                    let resp = req.send().await.map_err(HFError::Request)?;
                    let resp =
                        check_bucket_response(resp, &repo_id, NotFoundContext::Repo).await?;
                    let page: TreePage = resp.json().await.map_err(HFError::Json)?;
                    let next_cursor = page.next_cursor;
                    pending.extend(page.entries);
                    if let Some(entry) = pending.pop_front() {
                        Ok(Some((entry, (pending, next_cursor, true))))
                    } else {
                        Ok(None)
                    }
                }
            },
        )
    }
```

Ensure `use std::collections::VecDeque;` is at the top of the file (it was included in Task 4).

- [ ] **Step 4: Run all bucket tests**

```bash
cargo test -p huggingface_hub list_tree 2>&1
```

Expected: all `list_tree_url_*` tests pass, no compile errors.

- [ ] **Step 5: Commit**

```bash
git add huggingface_hub/src/api/buckets.rs
git commit -m "feat(bucket): implement list_tree with cursor-in-body streaming pagination"
```

---

## Task 8: `get_paths_info`, `get_xet_write_token`, `get_xet_read_token`

**Files:**
- Modify: `huggingface_hub/src/api/buckets.rs`

- [ ] **Step 1: Write failing tests**

Add to the `tests` module in `api/buckets.rs`:

```rust
    #[test]
    fn xet_token_urls() {
        let client = HFClientBuilder::new().build().unwrap();
        let bucket = client.bucket("myuser", "my-bucket");
        let write_url = format!(
            "{}/api/buckets/{}/{}/xet-write-token",
            bucket.client.inner.endpoint, bucket.namespace, bucket.repo
        );
        let read_url = format!(
            "{}/api/buckets/{}/{}/xet-read-token",
            bucket.client.inner.endpoint, bucket.namespace, bucket.repo
        );
        assert!(write_url.ends_with("/xet-write-token"));
        assert!(read_url.ends_with("/xet-read-token"));
    }

    #[test]
    fn paths_info_url() {
        let client = HFClientBuilder::new().build().unwrap();
        let bucket = client.bucket("myuser", "my-bucket");
        let url = format!(
            "{}/api/buckets/{}/{}/paths-info",
            bucket.client.inner.endpoint, bucket.namespace, bucket.repo
        );
        assert!(url.ends_with("/paths-info"));
    }
```

- [ ] **Step 2: Run the tests to confirm they pass**

```bash
cargo test -p huggingface_hub xet_token_urls paths_info_url 2>&1
```

Expected: both tests pass (URL construction tests don't need the methods yet).

- [ ] **Step 3: Implement the three methods on `HFBucket`**

Add to the `impl HFBucket` block in `api/buckets.rs`:

```rust
    /// Returns metadata for a batch of file paths.
    pub async fn get_paths_info(&self, paths: Vec<String>) -> Result<Vec<PathInfo>> {
        #[derive(serde::Serialize)]
        struct Body {
            paths: Vec<String>,
        }

        let resp = self
            .client
            .inner
            .client
            .post(format!("{}/paths-info", self.bucket_url()))
            .headers(self.client.auth_headers())
            .json(&Body { paths })
            .send()
            .await
            .map_err(HFError::Request)?;

        let resp = check_bucket_response(
            resp,
            &self.repo_id(),
            NotFoundContext::Entry { path: String::new() },
        )
        .await?;
        resp.json().await.map_err(HFError::Json)
    }

    /// Returns a short-lived JWT for uploading files to the Xet CAS.
    /// Use the returned `cas_url` and `token` to push file bytes before calling `batch_files`.
    pub async fn get_xet_write_token(&self) -> Result<XetToken> {
        let resp = self
            .client
            .inner
            .client
            .get(format!("{}/xet-write-token", self.bucket_url()))
            .headers(self.client.auth_headers())
            .send()
            .await
            .map_err(HFError::Request)?;
        let resp =
            check_bucket_response(resp, &self.repo_id(), NotFoundContext::Repo).await?;
        resp.json().await.map_err(HFError::Json)
    }

    /// Returns a short-lived JWT for downloading files from the Xet CAS directly.
    pub async fn get_xet_read_token(&self) -> Result<XetToken> {
        let resp = self
            .client
            .inner
            .client
            .get(format!("{}/xet-read-token", self.bucket_url()))
            .headers(self.client.auth_headers())
            .send()
            .await
            .map_err(HFError::Request)?;
        let resp =
            check_bucket_response(resp, &self.repo_id(), NotFoundContext::Repo).await?;
        resp.json().await.map_err(HFError::Json)
    }
```

- [ ] **Step 4: Run all bucket tests**

```bash
cargo test -p huggingface_hub -p huggingface_hub 2>&1 | grep "bucket\|FAILED\|ok"
```

Expected: all bucket tests pass, no new failures.

- [ ] **Step 5: Commit**

```bash
git add huggingface_hub/src/api/buckets.rs
git commit -m "feat(bucket): implement get_paths_info, get_xet_write_token, get_xet_read_token"
```

---

## Task 9: `resolve_file` — redirect capture with header extraction

**Files:**
- Modify: `huggingface_hub/src/api/buckets.rs`

- [ ] **Step 1: Write a failing test for `resolve_file` header parsing**

Add to the `tests` module in `api/buckets.rs`:

```rust
    #[test]
    fn resolve_file_parses_link_header() {
        // Verify the Link header parsing logic for xet-auth and xet-reconstruction-info
        let link = r#"<https://auth.example.com/token>; rel="xet-auth", <https://xet.example.com/reconstruct/abc>; rel="xet-reconstruction-info""#;
        let mut xet_auth = None;
        let mut xet_reconstruction = None;
        for part in link.split(',') {
            let part = part.trim();
            if let Some((url_part, rel_part)) = part.split_once(';') {
                let url = url_part.trim().trim_start_matches('<').trim_end_matches('>').to_string();
                let rel = rel_part.trim();
                if rel.contains("xet-auth") {
                    xet_auth = Some(url);
                } else if rel.contains("xet-reconstruction-info") {
                    xet_reconstruction = Some(url);
                }
            }
        }
        assert_eq!(xet_auth.unwrap(), "https://auth.example.com/token");
        assert_eq!(
            xet_reconstruction.unwrap(),
            "https://xet.example.com/reconstruct/abc"
        );
    }

    #[test]
    fn resolve_file_url() {
        let client = HFClientBuilder::new().build().unwrap();
        let bucket = client.bucket("myuser", "my-bucket");
        // Note: no /api/ prefix for resolve
        let url = format!(
            "{}/buckets/{}/{}/resolve/{}",
            bucket.client.inner.endpoint, bucket.namespace, bucket.repo, "data/train.parquet"
        );
        assert!(url.contains("/buckets/myuser/my-bucket/resolve/data/train.parquet"));
        assert!(!url.contains("/api/"));
    }
```

- [ ] **Step 2: Run the tests to confirm the link parsing test passes**

```bash
cargo test -p huggingface_hub resolve_file_parses_link resolve_file_url 2>&1
```

Expected: both tests pass (they test pure logic, no network).

- [ ] **Step 3: Implement `resolve_file` on `HFBucket`**

Add to the `impl HFBucket` block in `api/buckets.rs`:

```rust
    /// Resolves a file path to a direct download URL.
    ///
    /// Uses the no-redirect client to capture the 302 `Location` header rather than
    /// following it. Metadata is extracted from response headers:
    /// `X-Linked-Size`, `X-XET-Hash`, `X-Linked-ETag`, `Last-Modified`, and `Link`.
    pub async fn resolve_file(&self, path: &str) -> Result<ResolvedFile> {
        // Note: no /api/ prefix — this is the file-serving route, not the metadata API.
        let url = format!(
            "{}/buckets/{}/{}/resolve/{}",
            self.client.inner.endpoint, self.namespace, self.repo, path
        );
        let resp = self
            .client
            .inner
            .no_redirect_client
            .get(&url)
            .headers(self.client.auth_headers())
            .send()
            .await
            .map_err(HFError::Request)?;

        if !resp.status().is_redirection() {
            return Err(
                check_bucket_response(
                    resp,
                    &self.repo_id(),
                    NotFoundContext::Entry { path: path.to_string() },
                )
                .await
                .unwrap_err(),
            );
        }

        let headers = resp.headers();

        let location = headers
            .get("location")
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
            .ok_or_else(|| HFError::Http {
                status: resp.status().as_u16(),
                url: url.clone(),
                body: "missing Location header".to_string(),
            })?;

        let size = headers
            .get("x-linked-size")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok());

        let xet_hash = headers
            .get("x-xet-hash")
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);

        let etag = headers
            .get("x-linked-etag")
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);

        let last_modified = headers
            .get("last-modified")
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);

        // Parse Link header: <url>; rel="xet-auth", <url>; rel="xet-reconstruction-info"
        let mut xet_auth_url = None;
        let mut xet_reconstruction_url = None;
        if let Some(link) = headers.get("link").and_then(|v| v.to_str().ok()) {
            for part in link.split(',') {
                let part = part.trim();
                if let Some((url_part, rel_part)) = part.split_once(';') {
                    let u = url_part.trim().trim_start_matches('<').trim_end_matches('>').to_string();
                    if rel_part.contains("xet-auth") {
                        xet_auth_url = Some(u);
                    } else if rel_part.contains("xet-reconstruction-info") {
                        xet_reconstruction_url = Some(u);
                    }
                }
            }
        }

        Ok(ResolvedFile {
            url: location,
            size,
            xet_hash,
            etag,
            last_modified,
            xet_auth_url,
            xet_reconstruction_url,
        })
    }
```

- [ ] **Step 4: Run all bucket tests**

```bash
cargo test -p huggingface_hub 2>&1 | grep -E "bucket|resolve|FAILED|error"
```

Expected: all existing tests still pass, no compile errors.

- [ ] **Step 5: Commit**

```bash
git add huggingface_hub/src/api/buckets.rs
git commit -m "feat(bucket): implement resolve_file with redirect capture and header extraction"
```

---

## Task 10: `xet_resolve_file` (feature = `"xet"`)

**Files:**
- Modify: `huggingface_hub/src/api/buckets.rs`

- [ ] **Step 1: Write a failing test**

Add to the `tests` module in `api/buckets.rs`:

```rust
    #[cfg(feature = "xet")]
    #[test]
    fn xet_resolve_file_url() {
        let client = HFClientBuilder::new().build().unwrap();
        let bucket = client.bucket("myuser", "my-bucket");
        // Same URL as resolve_file — Accept header determines the response format
        let url = format!(
            "{}/buckets/{}/{}/resolve/{}",
            bucket.client.inner.endpoint, bucket.namespace, bucket.repo, "data/train.parquet"
        );
        assert!(url.contains("/buckets/myuser/my-bucket/resolve/data/train.parquet"));
    }
```

- [ ] **Step 2: Run the test under the xet feature**

```bash
cargo test -p huggingface_hub --features xet xet_resolve_file_url 2>&1
```

Expected: compile error — `xet_resolve_file` method not found (the test will compile but the URL test itself may pass; confirm `XetFileInfo` is missing).

- [ ] **Step 3: Implement `xet_resolve_file` on `HFBucket`**

Add to the `impl HFBucket` block in `api/buckets.rs`:

```rust
    /// Resolves a file path and returns Xet reconstruction metadata.
    ///
    /// Sends `Accept: application/vnd.xet-fileinfo+json` to request the JSON response
    /// instead of a redirect. Use the returned `reconstruction_url` to fetch chunk data
    /// from the Xet CAS directly.
    #[cfg(feature = "xet")]
    pub async fn xet_resolve_file(&self, path: &str) -> Result<crate::types::XetFileInfo> {
        let url = format!(
            "{}/buckets/{}/{}/resolve/{}",
            self.client.inner.endpoint, self.namespace, self.repo, path
        );
        let resp = self
            .client
            .inner
            .client
            .get(&url)
            .headers(self.client.auth_headers())
            .header("accept", "application/vnd.xet-fileinfo+json")
            .send()
            .await
            .map_err(HFError::Request)?;
        let resp = check_bucket_response(
            resp,
            &self.repo_id(),
            NotFoundContext::Entry { path: path.to_string() },
        )
        .await?;
        resp.json().await.map_err(HFError::Json)
    }
```

- [ ] **Step 4: Run tests with the xet feature**

```bash
cargo test -p huggingface_hub --features xet xet_resolve_file_url 2>&1
```

Expected: test passes.

- [ ] **Step 5: Confirm the build still works without the xet feature**

```bash
cargo build -p huggingface_hub 2>&1
```

Expected: compiles cleanly (no xet feature).

- [ ] **Step 6: Commit**

```bash
git add huggingface_hub/src/api/buckets.rs
git commit -m "feat(bucket): implement xet_resolve_file (feature = xet)"
```

---

## Task 11: Blocking wrappers (`HFBucketSync`)

**Files:**
- Modify: `huggingface_hub/src/blocking.rs`
- Modify: `huggingface_hub/src/lib.rs`

- [ ] **Step 1: Write a failing test**

Add to `huggingface_hub/src/blocking.rs` (inside `#[cfg(test)]` if one exists, or add a new one):

```rust
#[cfg(test)]
mod bucket_tests {
    #[cfg(feature = "blocking")]
    #[test]
    fn bucket_sync_constructor() {
        use crate::HFClientBuilder;
        let client = crate::blocking::HFClientSync::from(HFClientBuilder::new().build().unwrap());
        let bucket = client.bucket("myuser", "my-bucket");
        assert_eq!(bucket.inner.namespace, "myuser");
        assert_eq!(bucket.inner.repo, "my-bucket");
    }
}
```

- [ ] **Step 2: Run the test to confirm it fails**

```bash
cargo test -p huggingface_hub --features blocking bucket_sync_constructor 2>&1
```

Expected: compile error — `HFClientSync::bucket` does not exist.

- [ ] **Step 3: Add `HFBucketSync` struct to `blocking.rs`**

In `huggingface_hub/src/blocking.rs`, add alongside `HFRepositorySync` and `HFSpaceSync`:

```rust
/// Synchronous handle for Storage Bucket operations.
///
/// Obtain via [`HFClientSync::bucket`]. All methods block the current thread.
#[cfg(feature = "blocking")]
#[derive(Clone)]
pub struct HFBucketSync {
    pub(crate) inner: crate::repository::HFBucket,
    pub(crate) runtime: std::sync::Arc<tokio::runtime::Runtime>,
}
```

- [ ] **Step 4: Add `HFClientSync::bucket()` in `blocking.rs`**

In `blocking.rs`, find the `impl HFClientSync` block and add:

```rust
    /// Creates a synchronous bucket handle.
    pub fn bucket(
        &self,
        namespace: impl Into<String>,
        repo: impl Into<String>,
    ) -> HFBucketSync {
        HFBucketSync {
            inner: self.inner.bucket(namespace, repo),
            runtime: self.runtime.clone(),
        }
    }
```

- [ ] **Step 5: Add blocking methods to `HFBucketSync` in `blocking.rs`**

Add an `impl HFBucketSync` block:

```rust
#[cfg(feature = "blocking")]
impl HFBucketSync {
    pub fn get(&self) -> crate::Result<crate::types::BucketInfo> {
        self.runtime.block_on(self.inner.get())
    }

    pub fn delete(&self) -> crate::Result<()> {
        self.runtime.block_on(self.inner.delete())
    }

    pub fn update_settings(
        &self,
        params: crate::types::UpdateBucketParams,
    ) -> crate::Result<()> {
        self.runtime.block_on(self.inner.update_settings(params))
    }

    pub fn batch_files(
        &self,
        ops: Vec<crate::types::BatchOp>,
    ) -> crate::Result<crate::types::BatchResult> {
        self.runtime.block_on(self.inner.batch_files(ops))
    }

    pub fn list_tree(
        &self,
        path: &str,
        params: crate::types::ListTreeParams,
    ) -> crate::Result<Vec<crate::types::TreeEntry>> {
        use futures::StreamExt;
        self.runtime.block_on(async {
            let stream = self.inner.list_tree(path, params);
            futures::pin_mut!(stream);
            let mut items = Vec::new();
            while let Some(item) = stream.next().await {
                items.push(item?);
            }
            Ok(items)
        })
    }

    pub fn get_paths_info(
        &self,
        paths: Vec<String>,
    ) -> crate::Result<Vec<crate::types::PathInfo>> {
        self.runtime.block_on(self.inner.get_paths_info(paths))
    }

    pub fn get_xet_write_token(&self) -> crate::Result<crate::types::XetToken> {
        self.runtime.block_on(self.inner.get_xet_write_token())
    }

    pub fn get_xet_read_token(&self) -> crate::Result<crate::types::XetToken> {
        self.runtime.block_on(self.inner.get_xet_read_token())
    }

    pub fn resolve_file(&self, path: &str) -> crate::Result<crate::types::ResolvedFile> {
        self.runtime.block_on(self.inner.resolve_file(path))
    }

    #[cfg(feature = "xet")]
    pub fn xet_resolve_file(&self, path: &str) -> crate::Result<crate::types::XetFileInfo> {
        self.runtime.block_on(self.inner.xet_resolve_file(path))
    }
}
```

Also add a `list_buckets` blocking method on `HFClientSync`. Find the `impl HFClientSync` block and add:

```rust
    pub fn list_buckets(
        &self,
        namespace: &str,
    ) -> crate::Result<Vec<crate::types::BucketOverview>> {
        use futures::StreamExt;
        self.runtime.block_on(async {
            let stream = self.inner.list_buckets(namespace);
            futures::pin_mut!(stream);
            let mut items = Vec::new();
            while let Some(item) = stream.next().await {
                items.push(item?);
            }
            Ok(items)
        })
    }

    pub fn create_bucket(
        &self,
        namespace: &str,
        repo: &str,
        params: crate::types::CreateBucketParams,
    ) -> crate::Result<crate::types::BucketCreated> {
        self.runtime.block_on(self.inner.create_bucket(namespace, repo, params))
    }
```

- [ ] **Step 6: Export `HFBucketSync` from `lib.rs`**

In `huggingface_hub/src/lib.rs`, find the blocking re-export line:

```rust
#[cfg(feature = "blocking")]
pub use blocking::{HFClientSync, HFRepoSync, HFRepositorySync, HFSpaceSync};
```

Add `HFBucketSync` to this list:

```rust
#[cfg(feature = "blocking")]
pub use blocking::{HFBucketSync, HFClientSync, HFRepoSync, HFRepositorySync, HFSpaceSync};
```

- [ ] **Step 7: Run the test to confirm it passes**

```bash
cargo test -p huggingface_hub --features blocking bucket_sync_constructor 2>&1
```

Expected: test passes.

- [ ] **Step 8: Run the full test suite to check for regressions**

```bash
cargo test -p huggingface_hub --features blocking 2>&1 | grep -E "FAILED|error\[" | head -20
```

Expected: no failures.

- [ ] **Step 9: Commit**

```bash
git add huggingface_hub/src/blocking.rs huggingface_hub/src/lib.rs
git commit -m "feat(bucket): add HFBucketSync blocking wrappers"
```

---

## Task 12: Integration tests

**Files:**
- Modify: `huggingface_hub/tests/integration_test.rs`

- [ ] **Step 1: Write the integration tests (they will be skipped without credentials)**

Add the following to `huggingface_hub/tests/integration_test.rs`. The `api()` and `write_enabled()` helpers are already defined in the file; add only the new test functions:

```rust
// ---- HFBucket integration tests ----

/// Helper: creates a unique test bucket name to avoid collisions between runs.
fn test_bucket_name() -> String {
    format!(
        "test-bucket-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    )
}

#[tokio::test]
async fn test_list_buckets() {
    let Some(api) = api() else { return };
    let username = cached_username().await;
    // list_buckets is a read operation — no HF_TEST_WRITE required
    let buckets: Vec<_> = api
        .list_buckets(username)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<huggingface_hub::Result<Vec<_>>>()
        .expect("list_buckets failed");
    // Simply assert the call succeeds; the user may have zero buckets
    let _ = buckets;
}

#[tokio::test]
async fn test_create_and_delete_bucket() {
    let Some(api) = api() else { return };
    if !write_enabled() {
        return;
    }
    let username = cached_username().await;
    let name = test_bucket_name();

    // Create
    let created = api
        .create_bucket(
            username,
            &name,
            huggingface_hub::CreateBucketParams::builder().private(true).build(),
        )
        .await
        .expect("create_bucket failed");
    assert!(created.id.contains(&name));

    // Get
    let bucket = api.bucket(username, &name);
    let info = bucket.get().await.expect("get failed");
    assert_eq!(info.name, name);
    assert!(info.private);

    // Update settings
    bucket
        .update_settings(
            huggingface_hub::UpdateBucketParams::builder().private(false).build(),
        )
        .await
        .expect("update_settings failed");

    let info = bucket.get().await.unwrap();
    assert!(!info.private);

    // Delete
    bucket.delete().await.expect("delete failed");

    // Confirm gone
    assert!(matches!(
        bucket.get().await,
        Err(huggingface_hub::HFError::RepoNotFound { .. })
    ));
}

#[tokio::test]
async fn test_bucket_list_tree_empty() {
    let Some(api) = api() else { return };
    if !write_enabled() {
        return;
    }
    let username = cached_username().await;
    let name = test_bucket_name();

    api.create_bucket(
        username,
        &name,
        huggingface_hub::CreateBucketParams::builder().build(),
    )
    .await
    .expect("create_bucket failed");

    let bucket = api.bucket(username, &name);

    let entries: Vec<_> = bucket
        .list_tree("", huggingface_hub::ListTreeParams::builder().build())
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<huggingface_hub::Result<Vec<_>>>()
        .expect("list_tree failed");

    assert!(entries.is_empty(), "new bucket should have no files");

    bucket.delete().await.unwrap();
}

#[tokio::test]
async fn test_get_xet_write_and_read_token() {
    let Some(api) = api() else { return };
    if !write_enabled() {
        return;
    }
    let username = cached_username().await;
    let name = test_bucket_name();

    api.create_bucket(
        username,
        &name,
        huggingface_hub::CreateBucketParams::builder().build(),
    )
    .await
    .unwrap();

    let bucket = api.bucket(username, &name);

    let write_tok = bucket.get_xet_write_token().await.expect("xet write token failed");
    assert!(!write_tok.token.is_empty());
    assert!(!write_tok.cas_url.is_empty());

    let read_tok = bucket.get_xet_read_token().await.expect("xet read token failed");
    assert!(!read_tok.token.is_empty());

    bucket.delete().await.unwrap();
}
```

- [ ] **Step 2: Run the integration tests without credentials (they should be skipped)**

```bash
cargo test -p huggingface_hub --test integration_test test_list_buckets test_create_and_delete_bucket test_bucket_list_tree test_get_xet 2>&1
```

Expected: all 4 tests report "ok" (they exit early due to missing `HF_TOKEN`).

- [ ] **Step 3: Run the full library test suite to check for regressions**

```bash
cargo test -p huggingface_hub 2>&1 | grep -E "FAILED|error\[" | head -20
```

Expected: no failures.

- [ ] **Step 4: Commit**

```bash
git add huggingface_hub/tests/integration_test.rs
git commit -m "test(bucket): add integration tests for create, get, update, delete, list_tree, xet tokens"
```

---

## Self-Review

After all tasks are complete, run the full suite one final time:

```bash
cargo test -p huggingface_hub 2>&1
cargo test -p huggingface_hub --features blocking 2>&1
cargo test -p huggingface_hub --features xet 2>&1
cargo clippy -p huggingface_hub -- -D warnings 2>&1
```

All expected clean.
