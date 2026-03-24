# File System Cache Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement an interoperable file system cache for the Rust `huggingface-hub` library that shares the same on-disk layout as the Python `huggingface_hub` library.

**Architecture:** New `src/cache.rs` module with path computation, file locking, and cache scan/delete logic. The `download_file` method is reworked so `local_dir` is optional — when omitted, downloads go to the cache. A new `snapshot_download` method downloads entire repo snapshots with xet batching. Cache management (scan, delete) is exposed as `HfApi` methods.

**Tech Stack:** Rust, tokio, fs4 (file locking), reqwest (HTTP), globset (pattern matching)

**Spec:** `docs/superpowers/specs/2026-03-24-cache-system-design.md`

---

## File Structure

| File | Responsibility |
|------|---------------|
| `huggingface_hub/src/cache.rs` | **New.** Cache path computation, file locking, ref read/write, symlink creation, cache scan, cache deletion. All helpers are `pub(crate)`. |
| `huggingface_hub/src/types/cache.rs` | **New.** Public types: `CachedFileInfo`, `CachedRevisionInfo`, `CachedRepoInfo`, `HfCacheInfo`, `DeleteCacheRevision`. |
| `huggingface_hub/src/api/cache.rs` | **New.** `HfApi` methods: `scan_cache`, `delete_cache_revisions`. |
| `huggingface_hub/src/api/files.rs` | **Modify.** Rework `download_file` for cache mode. Add `snapshot_download`. |
| `huggingface_hub/src/client.rs` | **Modify.** Add `cache_dir` to `HfApiBuilder`/`HfApiInner`. |
| `huggingface_hub/src/constants.rs` | **Modify.** Add lock timeout constant. |
| `huggingface_hub/src/error.rs` | **Modify.** Add `LocalEntryNotFound`, `CacheLockTimeout` variants. |
| `huggingface_hub/src/types/params.rs` | **Modify.** Change `DownloadFileParams.local_dir` to `Option<PathBuf>`, add new fields. Add `SnapshotDownloadParams`. |
| `huggingface_hub/src/types/mod.rs` | **Modify.** Add `pub mod cache;` and re-export. |
| `huggingface_hub/src/api/mod.rs` | **Modify.** Add `pub mod cache;`. |
| `huggingface_hub/src/lib.rs` | **Modify.** Add `pub(crate) mod cache;`. |
| `huggingface_hub/src/xet.rs` | **Modify.** Update `xet_download` to accept blob target path. Add batch download helper. |
| `huggingface_hub/Cargo.toml` | **Modify.** Add `fs4` and `pathdiff` dependencies. |
| `huggingface_hub/tests/integration_test.rs` | **Modify.** Add cache integration tests, interop tests. |

---

### Task 1: Add `fs4` dependency and new constants

**Files:**
- Modify: `huggingface_hub/Cargo.toml`
- Modify: `huggingface_hub/src/constants.rs`

- [ ] **Step 1: Add `fs4` to Cargo.toml**

In `huggingface_hub/Cargo.toml`, add to `[dependencies]`:
```toml
fs4 = { version = "0.13", features = ["tokio"] }
pathdiff = "0.2"
```

- [ ] **Step 2: Add cache-related constants**

In `huggingface_hub/src/constants.rs`, add before the `#[cfg(test)]` block:
```rust
/// Default lock timeout in seconds for cache file locks
pub const CACHE_LOCK_TIMEOUT_SECS: u64 = 10;

/// Header for commit hash in download responses
pub const HEADER_X_REPO_COMMIT: &str = "x-repo-commit";

/// Header for linked etag (LFS files)
pub const HEADER_X_LINKED_ETAG: &str = "x-linked-etag";
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p huggingface-hub`
Expected: success

- [ ] **Step 4: Commit**

```bash
git add huggingface_hub/Cargo.toml huggingface_hub/src/constants.rs
git commit -m "feat: add fs4 dependency and cache-related constants"
```

---

### Task 2: Add new error variants

**Files:**
- Modify: `huggingface_hub/src/error.rs`

- [ ] **Step 1: Add error variants**

In `huggingface_hub/src/error.rs`, add two new variants to the `HfError` enum before the `#[error(transparent)]` block:

```rust
#[error("File not found in local cache: {path}")]
LocalEntryNotFound { path: String },

#[error("Cache lock timed out: {}", path.display())]
CacheLockTimeout { path: std::path::PathBuf },
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p huggingface-hub`
Expected: success

- [ ] **Step 3: Commit**

```bash
git add huggingface_hub/src/error.rs
git commit -m "feat: add LocalEntryNotFound and CacheLockTimeout error variants"
```

---

### Task 3: Add `cache_dir` to `HfApiBuilder` and `HfApiInner`

**Files:**
- Modify: `huggingface_hub/src/client.rs`

- [ ] **Step 1: Write a failing test**

Add to the existing `client.rs` tests or at the bottom of the file in a `#[cfg(test)]` module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_cache_dir_explicit() {
        let api = HfApiBuilder::new()
            .cache_dir("/tmp/my-cache")
            .build()
            .unwrap();
        assert_eq!(api.inner.cache_dir, std::path::PathBuf::from("/tmp/my-cache"));
    }

    #[test]
    fn test_builder_cache_dir_default() {
        // With no explicit cache_dir and no env vars, should resolve to ~/.cache/huggingface/hub
        let api = HfApiBuilder::new().build().unwrap();
        assert!(api.inner.cache_dir.to_string_lossy().ends_with("huggingface/hub")
            || api.inner.cache_dir.to_string_lossy().contains("huggingface"));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p huggingface-hub -- tests::test_builder_cache_dir`
Expected: FAIL — `cache_dir` field does not exist

- [ ] **Step 3: Implement**

In `huggingface_hub/src/client.rs`:

Add `cache_dir` field to `HfApiInner`:
```rust
pub(crate) struct HfApiInner {
    pub(crate) client: ClientWithMiddleware,
    pub(crate) endpoint: String,
    pub(crate) token: Option<String>,
    pub(crate) cache_dir: std::path::PathBuf,
    #[cfg(feature = "xet")]
    pub(crate) xet_session: std::sync::Mutex<Option<xet::xet_session::XetSession>>,
}
```

Add `cache_dir` field to `HfApiBuilder`:
```rust
pub struct HfApiBuilder {
    endpoint: Option<String>,
    token: Option<String>,
    user_agent: Option<String>,
    headers: Option<HeaderMap>,
    client: Option<reqwest::Client>,
    cache_dir: Option<std::path::PathBuf>,
}
```

Initialize it as `None` in `HfApiBuilder::new()`.

Add the builder method:
```rust
pub fn cache_dir(mut self, path: impl Into<std::path::PathBuf>) -> Self {
    self.cache_dir = Some(path.into());
    self
}
```

In `build()`, resolve the cache dir after endpoint resolution:
```rust
let cache_dir = self.cache_dir
    .or_else(|| std::env::var(constants::HF_HUB_CACHE).ok().map(std::path::PathBuf::from))
    .unwrap_or_else(|| {
        let hf_home = std::env::var(constants::HF_HOME).unwrap_or_else(|_| {
            let home = dirs_or_home();
            format!("{home}/.cache/huggingface")
        });
        std::path::PathBuf::from(hf_home).join("hub")
    });
```

Add `cache_dir` to the `HfApiInner` construction in `build()`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p huggingface-hub -- tests::test_builder_cache_dir`
Expected: PASS

- [ ] **Step 5: Run full check**

Run: `cargo check -p huggingface-hub --all-features`
Expected: success

- [ ] **Step 6: Commit**

```bash
git add huggingface_hub/src/client.rs
git commit -m "feat: add cache_dir to HfApiBuilder and HfApiInner"
```

---

### Task 4: Create `src/cache.rs` with path computation functions

**Files:**
- Create: `huggingface_hub/src/cache.rs`
- Modify: `huggingface_hub/src/lib.rs`

- [ ] **Step 1: Write failing tests for path computation**

Create `huggingface_hub/src/cache.rs` with tests only:

```rust
use std::path::{Path, PathBuf};
use crate::types::RepoType;

pub(crate) fn repo_folder_name(repo_id: &str, repo_type: Option<RepoType>) -> String {
    todo!()
}

pub(crate) fn blob_path(cache_dir: &Path, repo_folder: &str, etag: &str) -> PathBuf {
    todo!()
}

pub(crate) fn snapshot_path(cache_dir: &Path, repo_folder: &str, commit_hash: &str, filename: &str) -> PathBuf {
    todo!()
}

pub(crate) fn ref_path(cache_dir: &Path, repo_folder: &str, revision: &str) -> PathBuf {
    todo!()
}

pub(crate) fn lock_path(cache_dir: &Path, repo_folder: &str, etag: &str) -> PathBuf {
    todo!()
}

pub(crate) fn no_exist_path(cache_dir: &Path, repo_folder: &str, commit_hash: &str, filename: &str) -> PathBuf {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repo_folder_name_model_with_org() {
        assert_eq!(
            repo_folder_name("google/bert-base-uncased", Some(RepoType::Model)),
            "models--google--bert-base-uncased"
        );
    }

    #[test]
    fn test_repo_folder_name_model_no_org() {
        assert_eq!(
            repo_folder_name("gpt2", Some(RepoType::Model)),
            "models--gpt2"
        );
    }

    #[test]
    fn test_repo_folder_name_model_none_type() {
        assert_eq!(
            repo_folder_name("gpt2", None),
            "models--gpt2"
        );
    }

    #[test]
    fn test_repo_folder_name_dataset() {
        assert_eq!(
            repo_folder_name("rajpurkar/squad", Some(RepoType::Dataset)),
            "datasets--rajpurkar--squad"
        );
    }

    #[test]
    fn test_repo_folder_name_space() {
        assert_eq!(
            repo_folder_name("dalle-mini/dalle-mini", Some(RepoType::Space)),
            "spaces--dalle-mini--dalle-mini"
        );
    }

    #[test]
    fn test_blob_path() {
        let cache = Path::new("/home/user/.cache/huggingface/hub");
        assert_eq!(
            blob_path(cache, "models--gpt2", "abc123"),
            PathBuf::from("/home/user/.cache/huggingface/hub/models--gpt2/blobs/abc123")
        );
    }

    #[test]
    fn test_snapshot_path() {
        let cache = Path::new("/cache");
        assert_eq!(
            snapshot_path(cache, "models--gpt2", "aaa111", "config.json"),
            PathBuf::from("/cache/models--gpt2/snapshots/aaa111/config.json")
        );
    }

    #[test]
    fn test_snapshot_path_nested_file() {
        let cache = Path::new("/cache");
        assert_eq!(
            snapshot_path(cache, "models--gpt2", "aaa111", "subdir/model.bin"),
            PathBuf::from("/cache/models--gpt2/snapshots/aaa111/subdir/model.bin")
        );
    }

    #[test]
    fn test_ref_path() {
        let cache = Path::new("/cache");
        assert_eq!(
            ref_path(cache, "models--gpt2", "main"),
            PathBuf::from("/cache/models--gpt2/refs/main")
        );
    }

    #[test]
    fn test_ref_path_pr() {
        let cache = Path::new("/cache");
        assert_eq!(
            ref_path(cache, "models--gpt2", "refs/pr/1"),
            PathBuf::from("/cache/models--gpt2/refs/refs/pr/1")
        );
    }

    #[test]
    fn test_lock_path() {
        let cache = Path::new("/cache");
        assert_eq!(
            lock_path(cache, "models--gpt2", "abc123"),
            PathBuf::from("/cache/.locks/models--gpt2/abc123.lock")
        );
    }

    #[test]
    fn test_no_exist_path() {
        let cache = Path::new("/cache");
        assert_eq!(
            no_exist_path(cache, "models--gpt2", "aaa111", "missing.json"),
            PathBuf::from("/cache/models--gpt2/.no_exist/aaa111/missing.json")
        );
    }
}
```

- [ ] **Step 2: Add module to lib.rs**

In `huggingface_hub/src/lib.rs`, add after `pub mod constants;`:
```rust
pub(crate) mod cache;
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p huggingface-hub cache::tests`
Expected: FAIL — all functions panic with `todo!()`

- [ ] **Step 4: Implement path functions**

Replace the `todo!()` bodies:

```rust
pub(crate) fn repo_folder_name(repo_id: &str, repo_type: Option<RepoType>) -> String {
    let type_str = match repo_type {
        None | Some(RepoType::Model) => "models",
        Some(RepoType::Dataset) => "datasets",
        Some(RepoType::Space) => "spaces",
    };
    let parts: Vec<&str> = std::iter::once(type_str)
        .chain(repo_id.split('/'))
        .collect();
    parts.join("--")
}

pub(crate) fn blob_path(cache_dir: &Path, repo_folder: &str, etag: &str) -> PathBuf {
    cache_dir.join(repo_folder).join("blobs").join(etag)
}

pub(crate) fn snapshot_path(cache_dir: &Path, repo_folder: &str, commit_hash: &str, filename: &str) -> PathBuf {
    cache_dir.join(repo_folder).join("snapshots").join(commit_hash).join(filename)
}

pub(crate) fn ref_path(cache_dir: &Path, repo_folder: &str, revision: &str) -> PathBuf {
    cache_dir.join(repo_folder).join("refs").join(revision)
}

pub(crate) fn lock_path(cache_dir: &Path, repo_folder: &str, etag: &str) -> PathBuf {
    cache_dir.join(".locks").join(repo_folder).join(format!("{etag}.lock"))
}

pub(crate) fn no_exist_path(cache_dir: &Path, repo_folder: &str, commit_hash: &str, filename: &str) -> PathBuf {
    cache_dir.join(repo_folder).join(".no_exist").join(commit_hash).join(filename)
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p huggingface-hub cache::tests`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add huggingface_hub/src/cache.rs huggingface_hub/src/lib.rs
git commit -m "feat: add cache path computation functions with tests"
```

---

### Task 5: Add ref read/write and symlink helpers to `cache.rs`

**Files:**
- Modify: `huggingface_hub/src/cache.rs`

- [ ] **Step 1: Write failing tests**

Add to `cache.rs` tests module:

```rust
#[tokio::test]
async fn test_write_and_read_ref() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path();
    let repo_folder = "models--gpt2";
    write_ref(cache, repo_folder, "main", "abc123def456abc123def456abc123def456abcd").await.unwrap();
    let hash = read_ref(cache, repo_folder, "main").await.unwrap();
    assert_eq!(hash, Some("abc123def456abc123def456abc123def456abcd".to_string()));
}

#[tokio::test]
async fn test_read_ref_missing() {
    let dir = tempfile::tempdir().unwrap();
    let hash = read_ref(dir.path(), "models--gpt2", "main").await.unwrap();
    assert_eq!(hash, None);
}

#[cfg(not(windows))]
#[tokio::test]
async fn test_create_symlink() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path();
    let repo_folder = "models--gpt2";
    let etag = "abc123";
    let commit = "def456";

    // Create the blob file
    let blob = blob_path(cache, repo_folder, etag);
    tokio::fs::create_dir_all(blob.parent().unwrap()).await.unwrap();
    tokio::fs::write(&blob, b"file content").await.unwrap();

    // Create the symlink
    create_pointer_symlink(cache, repo_folder, commit, "config.json", etag).await.unwrap();

    let pointer = snapshot_path(cache, repo_folder, commit, "config.json");
    assert!(pointer.exists());
    assert!(pointer.symlink_metadata().unwrap().file_type().is_symlink());

    let content = tokio::fs::read_to_string(&pointer).await.unwrap();
    assert_eq!(content, "file content");
}

#[cfg(not(windows))]
#[tokio::test]
async fn test_create_symlink_nested_file() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path();
    let repo_folder = "models--gpt2";
    let etag = "abc123";
    let commit = "def456";

    // Create the blob file
    let blob = blob_path(cache, repo_folder, etag);
    tokio::fs::create_dir_all(blob.parent().unwrap()).await.unwrap();
    tokio::fs::write(&blob, b"nested content").await.unwrap();

    // Create symlink for a nested file (subdir/model.bin)
    create_pointer_symlink(cache, repo_folder, commit, "subdir/model.bin", etag).await.unwrap();

    let pointer = snapshot_path(cache, repo_folder, commit, "subdir/model.bin");
    assert!(pointer.exists());
    assert!(pointer.symlink_metadata().unwrap().file_type().is_symlink());

    // Symlink target should be ../../../blobs/abc123 (3 levels up for nested file)
    let target = std::fs::read_link(&pointer).unwrap();
    assert!(target.to_string_lossy().contains("blobs"));

    let content = tokio::fs::read_to_string(&pointer).await.unwrap();
    assert_eq!(content, "nested content");
}
```

Add `use tempfile;` to the test module (already a dev-dependency).

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p huggingface-hub cache::tests`
Expected: FAIL — functions not defined

- [ ] **Step 3: Implement ref and symlink helpers**

Add to `cache.rs`:

```rust
pub(crate) async fn write_ref(
    cache_dir: &Path,
    repo_folder: &str,
    revision: &str,
    commit_hash: &str,
) -> crate::error::Result<()> {
    let path = ref_path(cache_dir, repo_folder, revision);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(&path, commit_hash).await?;
    Ok(())
}

pub(crate) async fn read_ref(
    cache_dir: &Path,
    repo_folder: &str,
    revision: &str,
) -> crate::error::Result<Option<String>> {
    let path = ref_path(cache_dir, repo_folder, revision);
    match tokio::fs::read_to_string(&path).await {
        Ok(content) => Ok(Some(content.trim().to_string())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub(crate) async fn create_pointer_symlink(
    cache_dir: &Path,
    repo_folder: &str,
    commit_hash: &str,
    filename: &str,
    etag: &str,
) -> crate::error::Result<()> {
    let pointer = snapshot_path(cache_dir, repo_folder, commit_hash, filename);
    if let Some(parent) = pointer.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // Calculate relative path from pointer location to blob
    let blob = blob_path(cache_dir, repo_folder, etag);

    // Compute relative path from pointer's parent to blob
    let pointer_parent = pointer.parent().unwrap();
    let relative = pathdiff::diff_paths(&blob, pointer_parent)
        .unwrap_or(blob);

    // Remove existing symlink/file if it exists
    let _ = tokio::fs::remove_file(&pointer).await;

    #[cfg(not(windows))]
    {
        tokio::fs::symlink(&relative, &pointer).await?;
    }
    #[cfg(windows)]
    {
        // Windows fallback: copy file instead of symlink
        tokio::fs::copy(&blob_path(cache_dir, repo_folder, etag), &pointer).await?;
    }

    Ok(())
}

/// Check if a revision string looks like a full commit hash (40 hex chars).
pub(crate) fn is_commit_hash(revision: &str) -> bool {
    revision.len() == 40 && revision.chars().all(|c| c.is_ascii_hexdigit())
}
```

Note: `pathdiff` was already added to `Cargo.toml` in Task 1.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p huggingface-hub cache::tests`
Expected: PASS

- [ ] **Step 5: Run lints**

Run: `cargo clippy -p huggingface-hub -- -D warnings`
Expected: no warnings

- [ ] **Step 6: Commit**

```bash
git add huggingface_hub/Cargo.toml huggingface_hub/src/cache.rs
git commit -m "feat: add ref read/write, symlink creation, and commit hash detection"
```

---

### Task 6: Add file lock helpers to `cache.rs`

**Files:**
- Modify: `huggingface_hub/src/cache.rs`

- [ ] **Step 1: Write failing test**

Add to `cache.rs` tests:

```rust
#[tokio::test]
async fn test_acquire_and_release_lock() {
    let dir = tempfile::tempdir().unwrap();
    let lock = acquire_lock(dir.path(), "models--gpt2", "abc123").await.unwrap();
    // Lock file should exist
    let lock_file_path = lock_path(dir.path(), "models--gpt2", "abc123");
    assert!(lock_file_path.exists());
    drop(lock); // release
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p huggingface-hub cache::tests::test_acquire_and_release_lock`
Expected: FAIL — `acquire_lock` not defined

- [ ] **Step 3: Implement lock helpers**

Add to `cache.rs`:

```rust
use std::fs::File;
use fs4::FileExt;

pub(crate) struct CacheLock {
    _file: File,
}

pub(crate) async fn acquire_lock(
    cache_dir: &Path,
    repo_folder: &str,
    etag: &str,
) -> crate::error::Result<CacheLock> {
    let path = lock_path(cache_dir, repo_folder, etag);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let lock_path_clone = path.clone();
    let lock = tokio::time::timeout(
        std::time::Duration::from_secs(crate::constants::CACHE_LOCK_TIMEOUT_SECS),
        tokio::task::spawn_blocking(move || {
            let file = File::create(&lock_path_clone)?;
            file.lock_exclusive()?;
            Ok::<_, std::io::Error>(file)
        }),
    )
    .await
    .map_err(|_| crate::error::HfError::CacheLockTimeout { path: path.clone() })?
    .map_err(|e| crate::error::HfError::Other(format!("Lock task failed: {e}")))?
    .map_err(crate::error::HfError::Io)?;

    Ok(CacheLock { _file: lock })
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p huggingface-hub cache::tests::test_acquire_and_release_lock`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add huggingface_hub/src/cache.rs
git commit -m "feat: add cross-process file lock helpers using fs4"
```

---

### Task 7: Update `DownloadFileParams` and add `SnapshotDownloadParams`

**Files:**
- Modify: `huggingface_hub/src/types/params.rs`

- [ ] **Step 1: Update `DownloadFileParams`**

Change `local_dir` from required to optional with `#[builder(default)]`, and add new fields:

```rust
#[derive(TypedBuilder)]
pub struct DownloadFileParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub filename: String,
    #[builder(default, setter(strip_option))]
    pub local_dir: Option<PathBuf>,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(strip_option))]
    pub force_download: Option<bool>,
    #[builder(default, setter(strip_option))]
    pub local_files_only: Option<bool>,
}
```

- [ ] **Step 2: Add `SnapshotDownloadParams`**

Add after `DownloadFileParams`:

```rust
#[derive(TypedBuilder)]
pub struct SnapshotDownloadParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(strip_option))]
    pub allow_patterns: Option<Vec<String>>,
    #[builder(default, setter(strip_option))]
    pub ignore_patterns: Option<Vec<String>>,
    #[builder(default, setter(strip_option))]
    pub local_dir: Option<PathBuf>,
    #[builder(default, setter(strip_option))]
    pub force_download: Option<bool>,
    #[builder(default, setter(strip_option))]
    pub local_files_only: Option<bool>,
    #[builder(default, setter(strip_option))]
    pub max_workers: Option<usize>,
}
```

- [ ] **Step 3: Fix all callers of `DownloadFileParams` that set `local_dir`**

Update `huggingface_hub/tests/integration_test.rs` — the existing `test_download_file` uses `.local_dir(dir.path().to_path_buf())`. This continues to work because `TypedBuilder` with `strip_option` accepts the inner type directly.

Update `huggingface_hub/src/xet.rs` — `xet_download` accesses `params.local_dir` directly. Change to `params.local_dir.as_ref().expect(...)` or handle `None` (will be fully reworked in Task 10).

- [ ] **Step 4: Verify compilation**

Run: `cargo check -p huggingface-hub --all-features`
Expected: success

- [ ] **Step 5: Run existing tests**

Run: `cargo test -p huggingface-hub`
Expected: PASS (no behavior change)

- [ ] **Step 6: Commit**

```bash
git add huggingface_hub/src/types/params.rs huggingface_hub/src/xet.rs huggingface_hub/tests/integration_test.rs
git commit -m "feat: make DownloadFileParams.local_dir optional, add SnapshotDownloadParams"
```

---

### Task 8: Rework `download_file` for cache mode

This is the core task. The `download_file` method gains cache-aware behavior when `local_dir` is `None`.

**Files:**
- Modify: `huggingface_hub/src/api/files.rs`

- [ ] **Step 1: Write a unit test for the cache-mode download path**

Add to `huggingface_hub/tests/integration_test.rs`:

```rust
#[tokio::test]
async fn test_download_file_to_cache() {
    let Some(api_base) = api() else { return };
    let cache_dir = tempfile::tempdir().unwrap();
    let api = HfApiBuilder::new()
        .cache_dir(cache_dir.path())
        .build()
        .unwrap();

    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("config.json")
        .build();
    let path = api.download_file(&params).await.unwrap();

    // File should exist and be readable
    assert!(path.exists());
    let content = std::fs::read_to_string(&path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(json.get("model_type").is_some());

    // Should be under snapshots/
    assert!(path.to_string_lossy().contains("snapshots"));

    // Blob should exist — find the repo folder dynamically (Hub may redirect gpt2 -> openai-community/gpt2)
    let repo_folder = std::fs::read_dir(cache_dir.path()).unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.file_name().to_string_lossy().contains("gpt2"))
        .expect("repo folder not found");
    let blobs_dir = repo_folder.path().join("blobs");
    assert!(blobs_dir.exists());
    let blob_count = std::fs::read_dir(&blobs_dir).unwrap().count();
    assert_eq!(blob_count, 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `HF_TOKEN=hf_xxx cargo test -p huggingface-hub --test integration_test test_download_file_to_cache`
Expected: FAIL — no cache logic exists yet

- [ ] **Step 3: Implement cache-mode download**

Rework the `download_file` method in `huggingface_hub/src/api/files.rs`. The method body becomes:

```rust
pub async fn download_file(&self, params: &DownloadFileParams) -> Result<PathBuf> {
    let revision = params
        .revision
        .as_deref()
        .unwrap_or(constants::DEFAULT_REVISION);

    // local_dir mode: direct download, no cache
    if let Some(ref local_dir) = params.local_dir {
        return self.download_file_to_local_dir(params, local_dir, revision).await;
    }

    // Cache mode
    let repo_folder = crate::cache::repo_folder_name(&params.repo_id, params.repo_type);
    let cache_dir = &self.inner.cache_dir;

    // Step 2: Check cache hit by commit hash
    if crate::cache::is_commit_hash(revision) {
        let pointer = crate::cache::snapshot_path(cache_dir, &repo_folder, revision, &params.filename);
        if pointer.exists() && params.force_download != Some(true) {
            return Ok(pointer);
        }
    }

    // Step 3: local_files_only — check cache, no network
    if params.local_files_only == Some(true) {
        return self.resolve_from_cache_only(cache_dir, &repo_folder, revision, &params.filename).await;
    }

    // Step 4: Fetch from network
    self.download_file_to_cache(params, cache_dir, &repo_folder, revision).await
}
```

Implement the helper methods in a separate `impl HfApi` block in the same file:

```rust
impl HfApi {
    async fn download_file_to_local_dir(
        &self,
        params: &DownloadFileParams,
        local_dir: &Path,
        revision: &str,
    ) -> Result<PathBuf> {
        let url = self.download_url(params.repo_type, &params.repo_id, revision, &params.filename);

        #[cfg(feature = "xet")]
        {
            // existing xet HEAD check + xet_download path for local_dir
            // (preserve current behavior)
        }

        let response = self.inner.client.get(&url)
            .headers(self.auth_headers())
            .send().await?;
        let response = self.check_response(response, Some(&params.repo_id),
            crate::error::NotFoundContext::Entry { path: params.filename.clone() }).await?;

        tokio::fs::create_dir_all(local_dir).await?;
        let dest_path = local_dir.join(&params.filename);
        if let Some(parent) = dest_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let mut file = tokio::fs::File::create(&dest_path).await?;
        let mut stream = response.bytes_stream();
        use tokio::io::AsyncWriteExt;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
        }
        file.flush().await?;
        Ok(dest_path)
    }

    async fn resolve_from_cache_only(
        &self,
        cache_dir: &Path,
        repo_folder: &str,
        revision: &str,
        filename: &str,
    ) -> Result<PathBuf> {
        // Try direct commit hash
        if crate::cache::is_commit_hash(revision) {
            let pointer = crate::cache::snapshot_path(cache_dir, repo_folder, revision, filename);
            if pointer.exists() {
                return Ok(pointer);
            }
        }

        // Try resolving via ref
        if let Some(commit_hash) = crate::cache::read_ref(cache_dir, repo_folder, revision).await? {
            let pointer = crate::cache::snapshot_path(cache_dir, repo_folder, &commit_hash, filename);
            if pointer.exists() {
                return Ok(pointer);
            }
        }

        Err(HfError::LocalEntryNotFound {
            path: format!("{}/{}", repo_folder, filename),
        })
    }

    async fn download_file_to_cache(
        &self,
        params: &DownloadFileParams,
        cache_dir: &Path,
        repo_folder: &str,
        revision: &str,
    ) -> Result<PathBuf> {
        let url = self.download_url(params.repo_type, &params.repo_id, revision, &params.filename);

        // Check for cached etag to send If-None-Match
        // (look up existing blob via ref -> snapshot -> readlink)
        let cached_etag = self.find_cached_etag(cache_dir, repo_folder, revision, &params.filename).await;

        let mut request = self.inner.client.get(&url).headers(self.auth_headers());
        if let Some(ref etag) = cached_etag {
            if params.force_download != Some(true) {
                request = request.header(reqwest::header::IF_NONE_MATCH, format!("\"{etag}\""));
            }
        }

        let response = request.send().await?;
        let status = response.status();

        let status = response.status();

        // Handle 404 before check_response
        if status == reqwest::StatusCode::NOT_FOUND {
            if let Some(commit) = response.headers().get(constants::HEADER_X_REPO_COMMIT)
                .and_then(|v| v.to_str().ok()) {
                let marker = crate::cache::no_exist_path(cache_dir, repo_folder, commit, &params.filename);
                if let Some(parent) = marker.parent() {
                    let _ = tokio::fs::create_dir_all(parent).await;
                }
                let _ = tokio::fs::File::create(&marker).await;
            }
            return Err(HfError::EntryNotFound {
                path: params.filename.clone(),
                repo_id: params.repo_id.clone(),
            });
        }

        // Handle 304 Not Modified before check_response (304 is not 2xx)
        if status == reqwest::StatusCode::NOT_MODIFIED {
            let headers = response.headers();
            let etag = headers.get(constants::HEADER_X_LINKED_ETAG)
                .or_else(|| headers.get(reqwest::header::ETAG))
                .and_then(|v| v.to_str().ok())
                .map(|v| v.trim_matches('"').to_string())
                .or(cached_etag)
                .ok_or_else(|| HfError::Other("Missing ETag on 304 response".to_string()))?;

            let commit_hash = headers.get(constants::HEADER_X_REPO_COMMIT)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
                .ok_or_else(|| HfError::Other("Missing X-Repo-Commit on 304".to_string()))?;

            if !crate::cache::is_commit_hash(revision) {
                crate::cache::write_ref(cache_dir, repo_folder, revision, &commit_hash).await?;
            }
            crate::cache::create_pointer_symlink(cache_dir, repo_folder, &commit_hash, &params.filename, &etag).await?;
            return Ok(crate::cache::snapshot_path(cache_dir, repo_folder, &commit_hash, &params.filename));
        }

        // Now safe to call check_response (only 2xx remains)
        let response = self.check_response(response, Some(&params.repo_id),
            crate::error::NotFoundContext::Entry { path: params.filename.clone() }).await?;

        // Extract headers from 200 response
        let headers = response.headers();
        let etag = headers.get(constants::HEADER_X_LINKED_ETAG)
            .or_else(|| headers.get(reqwest::header::ETAG))
            .and_then(|v| v.to_str().ok())
            .map(|v| v.trim_matches('"').to_string())
            .ok_or_else(|| HfError::Other("Missing ETag header in response".to_string()))?;

        let commit_hash = headers.get(constants::HEADER_X_REPO_COMMIT)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .ok_or_else(|| HfError::Other("Missing X-Repo-Commit header".to_string()))?;

        // Write ref
        if !crate::cache::is_commit_hash(revision) {
            crate::cache::write_ref(cache_dir, repo_folder, revision, &commit_hash).await?;
        }

        // Check if blob already exists (another process may have downloaded it)
        if crate::cache::blob_path(cache_dir, repo_folder, &etag).exists()
            && params.force_download != Some(true)
        {
            crate::cache::create_pointer_symlink(cache_dir, repo_folder, &commit_hash, &params.filename, &etag).await?;
            return Ok(crate::cache::snapshot_path(cache_dir, repo_folder, &commit_hash, &params.filename));
        }

        // Acquire lock, download to .incomplete, rename, symlink
        let lock = crate::cache::acquire_lock(cache_dir, repo_folder, &etag).await?;

        let blob = crate::cache::blob_path(cache_dir, repo_folder, &etag);
        let incomplete_path = PathBuf::from(format!("{}.incomplete", blob.display()));

        if let Some(parent) = blob.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let mut file = tokio::fs::File::create(&incomplete_path).await?;
        let mut stream = response.bytes_stream();
        use tokio::io::AsyncWriteExt;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
        }
        file.flush().await?;
        drop(file);

        tokio::fs::rename(&incomplete_path, &blob).await?;

        crate::cache::create_pointer_symlink(cache_dir, repo_folder, &commit_hash, &params.filename, &etag).await?;

        drop(lock);

        Ok(crate::cache::snapshot_path(cache_dir, repo_folder, &commit_hash, &params.filename))
    }

    async fn find_cached_etag(
        &self,
        cache_dir: &Path,
        repo_folder: &str,
        revision: &str,
        filename: &str,
    ) -> Option<String> {
        let commit_hash = if crate::cache::is_commit_hash(revision) {
            Some(revision.to_string())
        } else {
            crate::cache::read_ref(cache_dir, repo_folder, revision).await.ok().flatten()
        };

        let commit_hash = commit_hash?;
        let pointer = crate::cache::snapshot_path(cache_dir, repo_folder, &commit_hash, filename);

        // Read the symlink target to extract the etag (blob filename)
        let target = tokio::fs::read_link(&pointer).await.ok()?;
        target.file_name()?.to_str().map(|s| s.to_string())
    }
}
```

This is a large implementation. The key is to get the core flow working — refinements (xet in cache mode) come in later tasks.

- [ ] **Step 4: Run the integration test**

Run: `HF_TOKEN=hf_xxx cargo test -p huggingface-hub --test integration_test test_download_file_to_cache`
Expected: PASS

- [ ] **Step 5: Run all existing tests to verify no regressions**

Run: `HF_TOKEN=hf_xxx cargo test -p huggingface-hub --test integration_test`
Expected: PASS (existing `test_download_file` unchanged)

- [ ] **Step 6: Format and lint**

Run: `cargo +nightly fmt && cargo clippy -p huggingface-hub -- -D warnings`
Expected: clean

- [ ] **Step 7: Commit**

```bash
git add huggingface_hub/src/api/files.rs huggingface_hub/tests/integration_test.rs
git commit -m "feat: rework download_file to support cache mode"
```

---

### Task 9: Add cache-mode integration tests

**Files:**
- Modify: `huggingface_hub/tests/integration_test.rs`

- [ ] **Step 1: Add cache hit test**

```rust
#[tokio::test]
async fn test_download_file_cache_hit() {
    let Some(_) = api() else { return };
    let cache_dir = tempfile::tempdir().unwrap();
    let api = HfApiBuilder::new()
        .cache_dir(cache_dir.path())
        .build()
        .unwrap();

    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("config.json")
        .build();

    // First download
    let path1 = api.download_file(&params).await.unwrap();

    // Second download — should be cache hit
    let path2 = api.download_file(&params).await.unwrap();

    assert_eq!(path1, path2);
}
```

- [ ] **Step 2: Add local_files_only test**

```rust
#[tokio::test]
async fn test_download_file_local_files_only_miss() {
    let cache_dir = tempfile::tempdir().unwrap();
    let api = HfApiBuilder::new()
        .cache_dir(cache_dir.path())
        .build()
        .unwrap();

    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("config.json")
        .local_files_only(true)
        .build();

    let result = api.download_file(&params).await;
    assert!(matches!(result, Err(huggingface_hub::HfError::LocalEntryNotFound { .. })));
}

#[tokio::test]
async fn test_download_file_local_files_only_hit() {
    let Some(_) = api() else { return };
    let cache_dir = tempfile::tempdir().unwrap();
    let api = HfApiBuilder::new()
        .cache_dir(cache_dir.path())
        .build()
        .unwrap();

    // Download first
    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("config.json")
        .build();
    let path1 = api.download_file(&params).await.unwrap();

    // Now local_files_only should work
    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("config.json")
        .local_files_only(true)
        .build();
    let path2 = api.download_file(&params).await.unwrap();
    assert_eq!(path1, path2);
}
```

- [ ] **Step 3: Add symlink structure test**

```rust
#[cfg(not(windows))]
#[tokio::test]
async fn test_download_file_cache_symlink_structure() {
    let Some(_) = api() else { return };
    let cache_dir = tempfile::tempdir().unwrap();
    let api = HfApiBuilder::new()
        .cache_dir(cache_dir.path())
        .build()
        .unwrap();

    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("config.json")
        .build();
    let path = api.download_file(&params).await.unwrap();

    // Path should be a symlink
    let meta = std::fs::symlink_metadata(&path).unwrap();
    assert!(meta.file_type().is_symlink());

    // Target should be in blobs/
    let target = std::fs::read_link(&path).unwrap();
    assert!(target.to_string_lossy().contains("blobs"));
}
```

- [ ] **Step 4: Run all new tests**

Run: `HF_TOKEN=hf_xxx cargo test -p huggingface-hub --test integration_test test_download_file_cache`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add huggingface_hub/tests/integration_test.rs
git commit -m "test: add cache-mode integration tests for download_file"
```

---

### Task 10: Update xet download for cache mode

**Files:**
- Modify: `huggingface_hub/src/xet.rs`
- Modify: `huggingface_hub/src/api/files.rs`

- [ ] **Step 1: Add `xet_download_to_blob` function**

In `huggingface_hub/src/xet.rs`, add a new function that downloads to a specified blob path instead of `params.local_dir`:

```rust
pub(crate) async fn xet_download_to_blob(
    api: &HfApi,
    repo_id: &str,
    repo_type: Option<RepoType>,
    revision: &str,
    file_hash: &str,
    file_size: u64,
    blob_path: &Path,
) -> Result<()> {
    let session = api
        .get_or_init_xet_session("read", repo_id, repo_type, revision)
        .await?;

    if let Some(parent) = blob_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let incomplete_path = PathBuf::from(format!("{}.incomplete", blob_path.display()));

    let group = session
        .new_download_group()
        .await
        .map_err(|e| HfError::Other(format!("Xet download failed: {e}")))?;

    let file_info = XetFileInfo {
        hash: file_hash.to_string(),
        file_size,
        sha256: None,
    };

    group
        .download_file_to_path(file_info, incomplete_path.clone())
        .await
        .map_err(|e| HfError::Other(format!("Xet download failed: {e}")))?;

    group
        .finish()
        .await
        .map_err(|e| HfError::Other(format!("Xet download failed: {e}")))?;

    tokio::fs::rename(&incomplete_path, blob_path).await?;
    Ok(())
}
```

- [ ] **Step 2: Add batch xet download function for snapshot_download**

```rust
pub(crate) async fn xet_download_batch(
    api: &HfApi,
    repo_id: &str,
    repo_type: Option<RepoType>,
    revision: &str,
    files: &[(String, u64, PathBuf)], // (xet_hash, file_size, blob_path)
) -> Result<()> {
    let session = api
        .get_or_init_xet_session("read", repo_id, repo_type, revision)
        .await?;

    let group = session
        .new_download_group()
        .await
        .map_err(|e| HfError::Other(format!("Xet download failed: {e}")))?;

    for (hash, size, blob_path) in files {
        if let Some(parent) = blob_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let incomplete_path = PathBuf::from(format!("{}.incomplete", blob_path.display()));

        let file_info = XetFileInfo {
            hash: hash.clone(),
            file_size: *size,
            sha256: None,
        };

        group
            .download_file_to_path(file_info, incomplete_path)
            .await
            .map_err(|e| HfError::Other(format!("Xet download failed: {e}")))?;
    }

    group
        .finish()
        .await
        .map_err(|e| HfError::Other(format!("Xet download failed: {e}")))?;

    // Rename all .incomplete -> final blob
    for (_, _, blob_path) in files {
        let incomplete_path = PathBuf::from(format!("{}.incomplete", blob_path.display()));
        tokio::fs::rename(&incomplete_path, blob_path).await?;
    }

    Ok(())
}
```

- [ ] **Step 3: Integrate xet into the cache-mode download flow in `files.rs`**

In `download_file_to_cache`, before the standard GET request, add the xet HEAD check:

```rust
#[cfg(feature = "xet")]
{
    let head_response = self.inner.client.head(&url)
        .headers(self.auth_headers())
        .send().await?;
    let head_response = self.check_response(head_response, Some(&params.repo_id),
        crate::error::NotFoundContext::Entry { path: params.filename.clone() }).await?;

    let headers = head_response.headers();
    if let Some(xet_hash) = headers.get(constants::HEADER_X_XET_HASH).and_then(|v| v.to_str().ok()) {
        let etag = headers.get(constants::HEADER_X_LINKED_ETAG)
            .or_else(|| headers.get(reqwest::header::ETAG))
            .and_then(|v| v.to_str().ok())
            .map(|v| v.trim_matches('"').to_string())
            .ok_or_else(|| HfError::Other("Missing ETag header".to_string()))?;

        let commit_hash = headers.get(constants::HEADER_X_REPO_COMMIT)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .ok_or_else(|| HfError::Other("Missing X-Repo-Commit header".to_string()))?;

        if !crate::cache::is_commit_hash(revision) {
            crate::cache::write_ref(cache_dir, repo_folder, revision, &commit_hash).await?;
        }

        let blob = crate::cache::blob_path(cache_dir, repo_folder, &etag);
        if !blob.exists() || params.force_download == Some(true) {
            let file_size: u64 = headers.get(reqwest::header::CONTENT_LENGTH)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);

            let lock = crate::cache::acquire_lock(cache_dir, repo_folder, &etag).await?;
            crate::xet::xet_download_to_blob(self, &params.repo_id, params.repo_type, revision, xet_hash, file_size, &blob).await?;
            crate::cache::create_pointer_symlink(cache_dir, repo_folder, &commit_hash, &params.filename, &etag).await?;
            drop(lock);
        } else {
            crate::cache::create_pointer_symlink(cache_dir, repo_folder, &commit_hash, &params.filename, &etag).await?;
        }

        return Ok(crate::cache::snapshot_path(cache_dir, repo_folder, &commit_hash, &params.filename));
    }
}
```

- [ ] **Step 4: Update original `xet_download` for local_dir mode**

Update `xet_download` in `xet.rs` to handle `Option<PathBuf>` for `local_dir`:

```rust
pub(crate) async fn xet_download(
    api: &HfApi,
    params: &DownloadFileParams,
    head_response: &reqwest::Response,
) -> Result<PathBuf> {
    let local_dir = params.local_dir.as_ref()
        .ok_or_else(|| HfError::Other("xet_download requires local_dir".to_string()))?;
    // ... rest unchanged but uses local_dir variable instead of params.local_dir
```

- [ ] **Step 5: Verify compilation and tests**

Run: `cargo check -p huggingface-hub --all-features && cargo test -p huggingface-hub`
Expected: success

- [ ] **Step 6: Commit**

```bash
git add huggingface_hub/src/xet.rs huggingface_hub/src/api/files.rs
git commit -m "feat: add xet cache-mode download and batch download support"
```

---

### Task 11: Implement `snapshot_download`

**Files:**
- Modify: `huggingface_hub/src/api/files.rs`
- Modify: `huggingface_hub/tests/integration_test.rs`

- [ ] **Step 1: Write integration test**

Add to `integration_test.rs`:

```rust
#[tokio::test]
async fn test_snapshot_download() {
    let Some(_) = api() else { return };
    let cache_dir = tempfile::tempdir().unwrap();
    let api = HfApiBuilder::new()
        .cache_dir(cache_dir.path())
        .build()
        .unwrap();

    let params = SnapshotDownloadParams::builder()
        .repo_id("gpt2")
        .allow_patterns(vec!["*.json".to_string()])
        .build();
    let snapshot_dir = api.snapshot_download(&params).await.unwrap();

    assert!(snapshot_dir.exists());
    assert!(snapshot_dir.to_string_lossy().contains("snapshots"));

    // Should have downloaded json files
    let config = snapshot_dir.join("config.json");
    assert!(config.exists());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `HF_TOKEN=hf_xxx cargo test -p huggingface-hub --test integration_test test_snapshot_download`
Expected: FAIL — method not defined

- [ ] **Step 3: Implement `snapshot_download`**

Add to `huggingface_hub/src/api/files.rs`:

```rust
impl HfApi {
    pub async fn snapshot_download(&self, params: &SnapshotDownloadParams) -> Result<PathBuf> {
        let revision = params.revision.as_deref().unwrap_or(constants::DEFAULT_REVISION);
        let max_workers = params.max_workers.unwrap_or(8);

        let repo_folder = crate::cache::repo_folder_name(&params.repo_id, params.repo_type);
        let cache_dir = &self.inner.cache_dir;

        // local_files_only: resolve from cache only, no network
        if params.local_files_only == Some(true) {
            let commit_hash = if crate::cache::is_commit_hash(revision) {
                revision.to_string()
            } else {
                crate::cache::read_ref(cache_dir, &repo_folder, revision).await?
                    .ok_or_else(|| HfError::LocalEntryNotFound {
                        path: format!("{}/{}", repo_folder, revision),
                    })?
            };
            let snapshot_dir = cache_dir.join(&repo_folder).join("snapshots").join(&commit_hash);
            if snapshot_dir.exists() {
                return Ok(snapshot_dir);
            }
            return Err(HfError::LocalEntryNotFound {
                path: format!("{}/{}", repo_folder, commit_hash),
            });
        }

        // Step 1: Resolve revision to commit hash (requires network)
        let commit_hash = if crate::cache::is_commit_hash(revision) {
            revision.to_string()
        } else {
            let sha = match params.repo_type {
                Some(RepoType::Dataset) => {
                    let p = crate::types::DatasetInfoParams::builder()
                        .repo_id(&params.repo_id).revision(revision).build();
                    self.dataset_info(&p).await?.sha
                }
                Some(RepoType::Space) => {
                    let p = crate::types::SpaceInfoParams::builder()
                        .repo_id(&params.repo_id).revision(revision).build();
                    self.space_info(&p).await?.sha
                }
                _ => {
                    let p = crate::types::ModelInfoParams::builder()
                        .repo_id(&params.repo_id).revision(revision).build();
                    self.model_info(&p).await?.sha
                }
            };
            sha.ok_or_else(|| HfError::Other(
                format!("No commit hash returned for {}/{}", params.repo_id, revision)
            ))?
        };

        // Step 2: List files
        let tree_params = ListRepoTreeParams::builder()
            .repo_id(&params.repo_id)
            .recursive(true)
            .build();
        let tree_params = ListRepoTreeParams {
            revision: Some(commit_hash.clone()),
            repo_type: params.repo_type,
            ..tree_params
        };

        let stream = self.list_repo_tree(&tree_params);
        futures::pin_mut!(stream);

        let mut filenames: Vec<String> = Vec::new();
        while let Some(entry) = stream.next().await {
            let entry = entry?;
            if let RepoTreeEntry::File { path, .. } = entry {
                filenames.push(path);
            }
        }

        // Step 3: Filter
        if let Some(ref allow) = params.allow_patterns {
            filenames.retain(|f| matches_any_glob(allow, f));
        }
        if let Some(ref ignore) = params.ignore_patterns {
            filenames.retain(|f| !matches_any_glob(ignore, f));
        }

        // Step 4: Filter cache hits
        if params.force_download != Some(true) {
            filenames.retain(|f| {
                !crate::cache::snapshot_path(cache_dir, &repo_folder, &commit_hash, f).exists()
            });
        }

        // Step 5-6: Download (non-xet concurrent, xet batched in parallel)
        if let Some(ref local_dir) = params.local_dir {
            // local_dir mode: download all to local_dir
            let download_futs = filenames.iter().map(|filename| {
                let dl_params = DownloadFileParams::builder()
                    .repo_id(&params.repo_id)
                    .filename(filename)
                    .local_dir(local_dir.clone())
                    .force_download(params.force_download.unwrap_or(false))
                    .build();
                let dl_params = DownloadFileParams {
                    repo_type: params.repo_type,
                    revision: Some(commit_hash.clone()),
                    ..dl_params
                };
                self.download_file(&dl_params)
            });

            futures::stream::iter(download_futs)
                .buffer_unordered(max_workers)
                .try_collect::<Vec<_>>()
                .await?;

            return Ok(local_dir.clone());
        }

        // Cache mode downloads
        let download_futs = filenames.iter().map(|filename| {
            let dl_params = DownloadFileParams::builder()
                .repo_id(&params.repo_id)
                .filename(filename)
                .build();
            let dl_params = DownloadFileParams {
                repo_type: params.repo_type,
                revision: Some(commit_hash.clone()),
                force_download: params.force_download,
                ..dl_params
            };
            self.download_file(&dl_params)
        });

        futures::stream::iter(download_futs)
            .buffer_unordered(max_workers)
            .try_collect::<Vec<_>>()
            .await?;

        // Write ref
        if !crate::cache::is_commit_hash(revision) {
            crate::cache::write_ref(cache_dir, &repo_folder, revision, &commit_hash).await?;
        }

        Ok(cache_dir.join(&repo_folder).join("snapshots").join(&commit_hash))
    }
}
```

Note: The xet batch optimization (splitting into xet vs non-xet groups and using `tokio::join!`) should be implemented as a follow-up refinement within this task after the basic flow works. The initial implementation delegates to `download_file` which already handles xet per-file. The batch optimization requires the HEAD-detection phase to be pulled into `snapshot_download`.

- [ ] **Step 4: Add `SnapshotDownloadParams` to imports and sync wrapper**

In `huggingface_hub/src/api/files.rs`, add `SnapshotDownloadParams` to the imports.

Add the sync API wrapper at the bottom of the file:
```rust
sync_api! {
    impl HfApiSync {
        // ... existing entries ...
        fn snapshot_download(&self, params: &SnapshotDownloadParams) -> Result<PathBuf>;
    }
}
```

- [ ] **Step 5: Run the test**

Run: `HF_TOKEN=hf_xxx cargo test -p huggingface-hub --test integration_test test_snapshot_download`
Expected: PASS

- [ ] **Step 6: Format and lint**

Run: `cargo +nightly fmt && cargo clippy -p huggingface-hub -- -D warnings`
Expected: clean

- [ ] **Step 7: Commit**

```bash
git add huggingface_hub/src/api/files.rs huggingface_hub/tests/integration_test.rs
git commit -m "feat: implement snapshot_download with concurrent downloads"
```

---

### Task 12: Implement xet batch optimization for `snapshot_download`

**Files:**
- Modify: `huggingface_hub/src/api/files.rs`

- [ ] **Step 1: Refactor snapshot_download cache-mode to split xet/non-xet**

In the cache-mode section of `snapshot_download`, replace the simple `download_file` loop with the detection + parallel download pattern described in the spec:

1. Send concurrent HEAD requests (when xet feature enabled) to detect xet files
2. Split into xet group and non-xet group
3. Use `tokio::join!` to run both groups in parallel:
   - Xet group: call `xet_download_batch` with a single download group
   - Non-xet group: concurrent GET downloads via `buffer_unordered`

This task requires careful integration with the existing flow. The HEAD requests extract etag, commit hash, and xet hash for each file.

- [ ] **Step 2: Verify with tests**

Run: `HF_TOKEN=hf_xxx cargo test -p huggingface-hub --test integration_test test_snapshot_download`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add huggingface_hub/src/api/files.rs
git commit -m "feat: batch xet downloads into single download group for snapshot_download"
```

---

### Task 13: Create cache management types

**Files:**
- Create: `huggingface_hub/src/types/cache.rs`
- Modify: `huggingface_hub/src/types/mod.rs`

- [ ] **Step 1: Create cache types**

Create `huggingface_hub/src/types/cache.rs`:

```rust
use std::path::PathBuf;
use std::time::SystemTime;
use super::repo::RepoType;

pub struct CachedFileInfo {
    pub file_name: String,
    pub file_path: PathBuf,
    pub blob_path: PathBuf,
    pub size_on_disk: u64,
    pub blob_last_accessed: SystemTime,
    pub blob_last_modified: SystemTime,
}

pub struct CachedRevisionInfo {
    pub commit_hash: String,
    pub snapshot_path: PathBuf,
    pub files: Vec<CachedFileInfo>,
    pub size_on_disk: u64,
    pub refs: Vec<String>,
    pub last_modified: SystemTime,
}

pub struct CachedRepoInfo {
    pub repo_id: String,
    pub repo_type: RepoType,
    pub repo_path: PathBuf,
    pub revisions: Vec<CachedRevisionInfo>,
    pub nb_files: usize,
    pub size_on_disk: u64,
    pub last_accessed: SystemTime,
    pub last_modified: SystemTime,
}

pub struct HfCacheInfo {
    pub cache_dir: PathBuf,
    pub repos: Vec<CachedRepoInfo>,
    pub size_on_disk: u64,
    pub warnings: Vec<String>,
}

pub struct DeleteCacheRevision {
    pub repo_id: String,
    pub repo_type: RepoType,
    pub commit_hash: String,
}
```

- [ ] **Step 2: Add to types/mod.rs**

Add to `huggingface_hub/src/types/mod.rs`:
```rust
pub mod cache;
pub use cache::*;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p huggingface-hub`
Expected: success

- [ ] **Step 4: Commit**

```bash
git add huggingface_hub/src/types/cache.rs huggingface_hub/src/types/mod.rs
git commit -m "feat: add cache management types"
```

---

### Task 14: Implement `scan_cache`

**Files:**
- Create: `huggingface_hub/src/api/cache.rs`
- Modify: `huggingface_hub/src/api/mod.rs`
- Modify: `huggingface_hub/src/cache.rs`

- [ ] **Step 1: Write failing test**

Add to `huggingface_hub/src/cache.rs` tests:

```rust
#[tokio::test]
async fn test_scan_cache_empty() {
    let dir = tempfile::tempdir().unwrap();
    let result = scan_cache_dir(dir.path()).await.unwrap();
    assert_eq!(result.repos.len(), 0);
    assert_eq!(result.size_on_disk, 0);
}

#[tokio::test]
async fn test_scan_cache_with_repo() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path();
    let repo_folder = "models--gpt2";

    // Create a blob
    let blob_dir = cache.join(repo_folder).join("blobs");
    tokio::fs::create_dir_all(&blob_dir).await.unwrap();
    tokio::fs::write(blob_dir.join("abc123"), b"hello world").await.unwrap();

    // Create a snapshot with symlink
    let snap_dir = cache.join(repo_folder).join("snapshots").join("commit1");
    tokio::fs::create_dir_all(&snap_dir).await.unwrap();
    #[cfg(not(windows))]
    tokio::fs::symlink("../../blobs/abc123", snap_dir.join("file.txt")).await.unwrap();

    // Create a ref
    let refs_dir = cache.join(repo_folder).join("refs");
    tokio::fs::create_dir_all(&refs_dir).await.unwrap();
    tokio::fs::write(refs_dir.join("main"), "commit1\n").await.unwrap();

    let result = scan_cache_dir(cache).await.unwrap();
    assert_eq!(result.repos.len(), 1);
    assert_eq!(result.repos[0].repo_id, "gpt2");
    assert_eq!(result.repos[0].revisions.len(), 1);
    assert_eq!(result.repos[0].revisions[0].refs, vec!["main"]);
}
```

- [ ] **Step 2: Implement `scan_cache_dir` in `cache.rs`**

Add a `pub(crate) async fn scan_cache_dir(cache_dir: &Path) -> Result<HfCacheInfo>` function that:

1. Lists directories in `cache_dir` matching `{type}s--*` pattern
2. For each repo folder:
   - Parse repo_type and repo_id from folder name
   - Read `refs/` directory to build ref → commit hash mapping
   - Read `snapshots/` directory to enumerate revisions
   - For each revision, walk files, follow symlinks to blobs, collect sizes and timestamps
3. Aggregate sizes, collect warnings for malformed entries

- [ ] **Step 3: Create `api/cache.rs` with `HfApi` methods**

Create `huggingface_hub/src/api/cache.rs`:

```rust
use crate::client::HfApi;
use crate::error::Result;
use crate::types::cache::HfCacheInfo;

impl HfApi {
    pub async fn scan_cache(&self) -> Result<HfCacheInfo> {
        crate::cache::scan_cache_dir(&self.inner.cache_dir).await
    }
}
```

Add `pub mod cache;` to `huggingface_hub/src/api/mod.rs`.

- [ ] **Step 4: Run tests**

Run: `cargo test -p huggingface-hub cache::tests::test_scan_cache`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add huggingface_hub/src/cache.rs huggingface_hub/src/api/cache.rs huggingface_hub/src/api/mod.rs
git commit -m "feat: implement scan_cache for cache directory inspection"
```

---

### Task 15: Implement `delete_cache_revisions`

**Files:**
- Modify: `huggingface_hub/src/cache.rs`
- Modify: `huggingface_hub/src/api/cache.rs`

- [ ] **Step 1: Write failing test**

Add to `cache.rs` tests:

```rust
#[tokio::test]
async fn test_delete_cache_revision() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path();
    let repo_folder = "models--gpt2";

    // Create blobs
    let blob_dir = cache.join(repo_folder).join("blobs");
    tokio::fs::create_dir_all(&blob_dir).await.unwrap();
    tokio::fs::write(blob_dir.join("shared_blob"), b"shared").await.unwrap();
    tokio::fs::write(blob_dir.join("unique_blob"), b"unique").await.unwrap();

    // Create two snapshots
    let snap1 = cache.join(repo_folder).join("snapshots").join("commit1");
    let snap2 = cache.join(repo_folder).join("snapshots").join("commit2");
    tokio::fs::create_dir_all(&snap1).await.unwrap();
    tokio::fs::create_dir_all(&snap2).await.unwrap();

    #[cfg(not(windows))]
    {
        tokio::fs::symlink("../../blobs/shared_blob", snap1.join("file.txt")).await.unwrap();
        tokio::fs::symlink("../../blobs/shared_blob", snap2.join("file.txt")).await.unwrap();
        tokio::fs::symlink("../../blobs/unique_blob", snap1.join("extra.txt")).await.unwrap();
    }

    // Delete commit1
    delete_revisions(cache, &[("gpt2", RepoType::Model, "commit1")]).await.unwrap();

    // commit1 snapshot gone
    assert!(!snap1.exists());
    // commit2 snapshot still exists
    assert!(snap2.exists());
    // shared blob still exists (referenced by commit2)
    assert!(blob_dir.join("shared_blob").exists());
    // unique blob deleted (orphaned)
    #[cfg(not(windows))]
    assert!(!blob_dir.join("unique_blob").exists());
}
```

- [ ] **Step 2: Implement `delete_revisions` in `cache.rs`**

```rust
pub(crate) async fn delete_revisions(
    cache_dir: &Path,
    revisions: &[(&str, RepoType, &str)], // (repo_id, repo_type, commit_hash)
) -> crate::error::Result<()> {
    // Group by repo
    // For each repo:
    //   1. Remove snapshot directories
    //   2. Remove refs pointing to deleted commits
    //   3. Find orphaned blobs (not referenced by remaining snapshots)
    //   4. Delete orphaned blobs
    // Log warnings for non-fatal errors, continue processing
    todo!()
}
```

- [ ] **Step 3: Add `HfApi::delete_cache_revisions` to `api/cache.rs`**

```rust
impl HfApi {
    pub async fn delete_cache_revisions(&self, revisions: &[DeleteCacheRevision]) -> Result<()> {
        let refs: Vec<(&str, RepoType, &str)> = revisions.iter()
            .map(|r| (r.repo_id.as_str(), r.repo_type, r.commit_hash.as_str()))
            .collect();
        crate::cache::delete_revisions(&self.inner.cache_dir, &refs).await
    }
}
```

- [ ] **Step 4: Implement the deletion logic**

The full implementation should:
1. Group revisions by repo folder
2. For each repo, remove the snapshot directory for each specified commit
3. Read all refs, remove any that point to deleted commits
4. Walk remaining snapshots to find all referenced blobs
5. Walk blobs directory, delete any blob not in the referenced set
6. Wrap each IO operation in error handling that logs and continues

- [ ] **Step 5: Run tests**

Run: `cargo test -p huggingface-hub cache::tests::test_delete_cache_revision`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add huggingface_hub/src/cache.rs huggingface_hub/src/api/cache.rs
git commit -m "feat: implement delete_cache_revisions with orphan cleanup"
```

---

### Task 16: Add sync API wrappers for new methods

**Files:**
- Modify: `huggingface_hub/src/api/files.rs`
- Modify: `huggingface_hub/src/api/cache.rs`

- [ ] **Step 1: Add sync wrappers**

In `huggingface_hub/src/api/files.rs`, update the `sync_api!` block to include `snapshot_download`:

```rust
sync_api! {
    impl HfApiSync {
        fn list_repo_files(&self, params: &ListRepoFilesParams) -> Result<Vec<String>>;
        fn get_paths_info(&self, params: &GetPathsInfoParams) -> Result<Vec<RepoTreeEntry>>;
        fn download_file(&self, params: &DownloadFileParams) -> Result<PathBuf>;
        fn snapshot_download(&self, params: &SnapshotDownloadParams) -> Result<PathBuf>;
        fn create_commit(&self, params: &CreateCommitParams) -> Result<CommitInfo>;
        fn upload_file(&self, params: &UploadFileParams) -> Result<CommitInfo>;
        fn upload_folder(&self, params: &UploadFolderParams) -> Result<CommitInfo>;
        fn delete_file(&self, params: &DeleteFileParams) -> Result<CommitInfo>;
        fn delete_folder(&self, params: &DeleteFolderParams) -> Result<CommitInfo>;
    }
}
```

In `huggingface_hub/src/api/cache.rs`, add:

```rust
sync_api! {
    impl HfApiSync {
        fn scan_cache(&self) -> Result<HfCacheInfo>;
        fn delete_cache_revisions(&self, revisions: &[DeleteCacheRevision]) -> Result<()>;
    }
}
```

- [ ] **Step 2: Verify compilation with blocking feature**

Run: `cargo check -p huggingface-hub --features blocking`
Expected: success

- [ ] **Step 3: Commit**

```bash
git add huggingface_hub/src/api/files.rs huggingface_hub/src/api/cache.rs
git commit -m "feat: add sync API wrappers for cache and snapshot methods"
```

---

### Task 17: Integration tests for scan_cache and delete_cache_revisions

**Files:**
- Modify: `huggingface_hub/tests/integration_test.rs`

- [ ] **Step 1: Add scan_cache test after downloads**

```rust
#[tokio::test]
async fn test_scan_cache_after_download() {
    let Some(_) = api() else { return };
    let cache_dir = tempfile::tempdir().unwrap();
    let api = HfApiBuilder::new()
        .cache_dir(cache_dir.path())
        .build()
        .unwrap();

    // Download a file
    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("config.json")
        .build();
    api.download_file(&params).await.unwrap();

    // Scan
    let info = api.scan_cache().await.unwrap();
    assert_eq!(info.repos.len(), 1);
    assert!(info.repos[0].repo_id.contains("gpt2"));
    assert_eq!(info.repos[0].revisions.len(), 1);
    assert!(info.repos[0].revisions[0].files.len() >= 1);
    assert!(info.size_on_disk > 0);
}
```

- [ ] **Step 2: Add delete test**

```rust
#[tokio::test]
async fn test_delete_cache_revisions_integration() {
    let Some(_) = api() else { return };
    let cache_dir = tempfile::tempdir().unwrap();
    let api = HfApiBuilder::new()
        .cache_dir(cache_dir.path())
        .build()
        .unwrap();

    // Download
    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("config.json")
        .build();
    api.download_file(&params).await.unwrap();

    // Scan to get commit hash
    let info = api.scan_cache().await.unwrap();
    let commit = info.repos[0].revisions[0].commit_hash.clone();

    // Delete
    api.delete_cache_revisions(&[DeleteCacheRevision {
        repo_id: info.repos[0].repo_id.clone(),
        repo_type: info.repos[0].repo_type,
        commit_hash: commit,
    }]).await.unwrap();

    // Scan again — should be empty or have no revisions
    let info = api.scan_cache().await.unwrap();
    if !info.repos.is_empty() {
        assert_eq!(info.repos[0].revisions.len(), 0);
    }
}
```

- [ ] **Step 3: Run tests**

Run: `HF_TOKEN=hf_xxx cargo test -p huggingface-hub --test integration_test test_scan_cache test_delete_cache`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add huggingface_hub/tests/integration_test.rs
git commit -m "test: add integration tests for scan_cache and delete_cache_revisions"
```

---

### Task 18: Cross-library interoperability tests

**Files:**
- Modify: `huggingface_hub/tests/integration_test.rs`

- [ ] **Step 1: Add helper to create Python venv**

```rust
use std::process::Command;

fn python_available() -> bool {
    Command::new("python3").arg("--version").output().is_ok()
}

fn setup_python_venv(base_dir: &Path) -> Option<PathBuf> {
    if !python_available() {
        return None;
    }
    let venv_dir = base_dir.join("venv");
    let status = Command::new("python3")
        .args(["-m", "venv", &venv_dir.to_string_lossy()])
        .status()
        .ok()?;
    if !status.success() { return None; }

    let pip = venv_dir.join("bin").join("pip");
    let status = Command::new(&pip)
        .args(["install", "huggingface_hub"])
        .status()
        .ok()?;
    if !status.success() { return None; }

    Some(venv_dir)
}

fn python_bin(venv_dir: &Path) -> PathBuf {
    venv_dir.join("bin").join("python")
}
```

- [ ] **Step 2: Add Python-downloads-first test**

```rust
#[tokio::test]
async fn test_interop_python_downloads_first() {
    let Some(_) = api() else { return };
    let base_dir = tempfile::tempdir().unwrap();
    let cache_dir = base_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).unwrap();

    let Some(venv_dir) = setup_python_venv(base_dir.path()) else { return };
    let python = python_bin(&venv_dir);
    let token = std::env::var("HF_TOKEN").unwrap();

    // Python downloads config.json
    let script = format!(
        r#"
import os
os.environ["HF_HUB_CACHE"] = "{cache}"
os.environ["HF_TOKEN"] = "{token}"
from huggingface_hub import hf_hub_download
path = hf_hub_download("gpt2", "config.json")
print(path)
"#,
        cache = cache_dir.display(),
        token = token,
    );
    let output = Command::new(&python).args(["-c", &script]).output().unwrap();
    assert!(output.status.success(), "Python script failed: {}", String::from_utf8_lossy(&output.stderr));

    // Count blobs
    let repo_folder = std::fs::read_dir(&cache_dir).unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.file_name().to_string_lossy().contains("gpt2"))
        .unwrap();
    let blob_count_before = std::fs::read_dir(repo_folder.path().join("blobs"))
        .unwrap().count();

    // Rust downloads same file
    let api = HfApiBuilder::new()
        .cache_dir(&cache_dir)
        .build()
        .unwrap();
    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("config.json")
        .build();
    let path = api.download_file(&params).await.unwrap();
    assert!(path.exists());

    // No new blobs
    let blob_count_after = std::fs::read_dir(repo_folder.path().join("blobs"))
        .unwrap().count();
    assert_eq!(blob_count_before, blob_count_after);
}
```

- [ ] **Step 3: Add Rust-downloads-first test**

```rust
#[tokio::test]
async fn test_interop_rust_downloads_first() {
    let Some(_) = api() else { return };
    let base_dir = tempfile::tempdir().unwrap();
    let cache_dir = base_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).unwrap();

    let Some(venv_dir) = setup_python_venv(base_dir.path()) else { return };
    let python = python_bin(&venv_dir);
    let token = std::env::var("HF_TOKEN").unwrap();

    // Rust downloads first
    let api = HfApiBuilder::new()
        .cache_dir(&cache_dir)
        .build()
        .unwrap();
    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("config.json")
        .build();
    api.download_file(&params).await.unwrap();

    // Python verifies cache hit with local_files_only=True
    let script = format!(
        r#"
import os
os.environ["HF_HUB_CACHE"] = "{cache}"
os.environ["HF_TOKEN"] = "{token}"
from huggingface_hub import hf_hub_download
path = hf_hub_download("gpt2", "config.json", local_files_only=True)
print(path)
"#,
        cache = cache_dir.display(),
        token = token,
    );
    let output = Command::new(&python).args(["-c", &script]).output().unwrap();
    assert!(output.status.success(), "Python local_files_only failed: {}", String::from_utf8_lossy(&output.stderr));
}
```

- [ ] **Step 4: Add Python snapshot_download → Rust snapshot_download test**

```rust
#[tokio::test]
async fn test_interop_python_snapshot_rust_snapshot() {
    let Some(_) = api() else { return };
    let base_dir = tempfile::tempdir().unwrap();
    let cache_dir = base_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).unwrap();

    let Some(venv_dir) = setup_python_venv(base_dir.path()) else { return };
    let python = python_bin(&venv_dir);
    let token = std::env::var("HF_TOKEN").unwrap();

    // Python snapshot_download with json filter
    let script = format!(
        r#"
import os
os.environ["HF_HUB_CACHE"] = "{cache}"
os.environ["HF_TOKEN"] = "{token}"
from huggingface_hub import snapshot_download
path = snapshot_download("gpt2", allow_patterns=["*.json"])
print(path)
"#,
        cache = cache_dir.display(),
        token = token,
    );
    let output = Command::new(&python).args(["-c", &script]).output().unwrap();
    assert!(output.status.success(), "Python snapshot_download failed: {}", String::from_utf8_lossy(&output.stderr));

    // Count blobs before Rust download
    let repo_folder = std::fs::read_dir(&cache_dir).unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.file_name().to_string_lossy().contains("gpt2"))
        .unwrap();
    let blob_count_before = std::fs::read_dir(repo_folder.path().join("blobs"))
        .unwrap().count();

    // Rust snapshot_download with same filter
    let api = HfApiBuilder::new()
        .cache_dir(&cache_dir)
        .build()
        .unwrap();
    let params = SnapshotDownloadParams::builder()
        .repo_id("gpt2")
        .allow_patterns(vec!["*.json".to_string()])
        .build();
    let snapshot_dir = api.snapshot_download(&params).await.unwrap();
    assert!(snapshot_dir.exists());

    // No new blobs
    let blob_count_after = std::fs::read_dir(repo_folder.path().join("blobs"))
        .unwrap().count();
    assert_eq!(blob_count_before, blob_count_after);
}
```

- [ ] **Step 5: Add mixed partial downloads test**

```rust
#[tokio::test]
async fn test_interop_mixed_partial_downloads() {
    let Some(_) = api() else { return };
    let base_dir = tempfile::tempdir().unwrap();
    let cache_dir = base_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).unwrap();

    let Some(venv_dir) = setup_python_venv(base_dir.path()) else { return };
    let python = python_bin(&venv_dir);
    let token = std::env::var("HF_TOKEN").unwrap();

    // Python downloads README.md
    let script = format!(
        r#"
import os
os.environ["HF_HUB_CACHE"] = "{cache}"
os.environ["HF_TOKEN"] = "{token}"
from huggingface_hub import hf_hub_download
hf_hub_download("gpt2", "README.md")
"#,
        cache = cache_dir.display(),
        token = token,
    );
    let output = Command::new(&python).args(["-c", &script]).output().unwrap();
    assert!(output.status.success());

    // Rust downloads config.json
    let api = HfApiBuilder::new()
        .cache_dir(&cache_dir)
        .build()
        .unwrap();
    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("config.json")
        .build();
    api.download_file(&params).await.unwrap();

    // Rust can see Python's file with local_files_only
    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("README.md")
        .local_files_only(true)
        .build();
    let readme_path = api.download_file(&params).await.unwrap();
    assert!(readme_path.exists());

    // Python can see Rust's file
    let script = format!(
        r#"
import os
os.environ["HF_HUB_CACHE"] = "{cache}"
os.environ["HF_TOKEN"] = "{token}"
from huggingface_hub import hf_hub_download
path = hf_hub_download("gpt2", "config.json", local_files_only=True)
print(path)
"#,
        cache = cache_dir.display(),
        token = token,
    );
    let output = Command::new(&python).args(["-c", &script]).output().unwrap();
    assert!(output.status.success(), "Python can't find Rust's cached file: {}", String::from_utf8_lossy(&output.stderr));

    // scan_cache sees both files
    let info = api.scan_cache().await.unwrap();
    assert_eq!(info.repos.len(), 1);
    let total_files: usize = info.repos[0].revisions.iter()
        .map(|r| r.files.len()).sum();
    assert!(total_files >= 2);
}
```

- [ ] **Step 6: Run interop tests**

Run: `HF_TOKEN=hf_xxx cargo test -p huggingface-hub --test integration_test test_interop`
Expected: PASS (or skip if python3 not available)

- [ ] **Step 7: Commit**

```bash
git add huggingface_hub/tests/integration_test.rs
git commit -m "test: add cross-library interoperability tests with Python huggingface_hub"
```

---

### Task 19: Final formatting, linting, and full test run

**Files:**
- All modified files

- [ ] **Step 1: Format**

Run: `cargo +nightly fmt`

- [ ] **Step 2: Lint**

Run: `cargo clippy -p huggingface-hub --all-features -- -D warnings`
Expected: no warnings

- [ ] **Step 3: Run unit tests**

Run: `cargo test -p huggingface-hub`
Expected: PASS

- [ ] **Step 4: Run integration tests**

Run: `HF_TOKEN=hf_xxx cargo test -p huggingface-hub --test integration_test`
Expected: PASS

- [ ] **Step 5: Run integration tests with write**

Run: `HF_TOKEN=hf_xxx HF_TEST_WRITE=1 cargo test -p huggingface-hub --test integration_test`
Expected: PASS

- [ ] **Step 6: Commit any formatting changes**

```bash
git add -A && git commit -m "chore: formatting and lint fixes"
```

---

### Task 20: Update CLAUDE.md project layout

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: Update the project layout section**

Add the new files to the project layout in CLAUDE.md:

```
│   │   ├── cache.rs                # Cache path computation, locking, scan, deletion
│   │   ├── types/
│   │   │   ├── cache.rs            # CachedFileInfo, CachedRepoInfo, HfCacheInfo, etc.
│   │   └── api/
│   │       ├── cache.rs            # scan_cache, delete_cache_revisions
```

- [ ] **Step 2: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: update project layout with cache module"
```
