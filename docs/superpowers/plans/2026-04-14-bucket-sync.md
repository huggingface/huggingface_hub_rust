# Bucket Sync Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `HFBucket::sync()` that synchronizes files between a local directory and a bucket, with comparison modes and glob filtering.

**Architecture:** A `sync()` method on `HFBucket` computes a plan (compare local vs remote files by size/mtime), executes it (upload/download/delete via existing methods), and returns the plan for inspection. A CLI subcommand `hfrs buckets sync` wraps this with argument parsing and summary output.

**Tech Stack:** Rust, `globset` for pattern matching, `futures::StreamExt` for async stream consumption, existing `upload_files`/`download_files`/`delete_files` on `HFBucket`.

**Spec:** `docs/superpowers/specs/2026-04-14-bucket-sync-design.md`

---

### Task 1: Add `SyncDirection` and `BucketSyncParams` types

**Files:**
- Modify: `huggingface_hub/src/types/bucket_params.rs`

- [ ] **Step 1: Add `SyncDirection` enum and `BucketSyncParams` struct**

Add at the bottom of `huggingface_hub/src/types/bucket_params.rs`:

```rust
use crate::types::progress::Progress;

/// Direction for a bucket sync operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncDirection {
    /// Local directory -> bucket (upload).
    Upload,
    /// Bucket -> local directory (download).
    Download,
}

/// Parameters for syncing files between a local directory and a bucket.
///
/// Used with [`HFBucket::sync`](crate::bucket::HFBucket::sync).
#[derive(Debug, Clone, TypedBuilder)]
pub struct BucketSyncParams {
    /// Local directory path.
    pub local_path: PathBuf,
    /// Sync direction.
    pub direction: SyncDirection,
    /// Optional prefix within the bucket (subdirectory).
    #[builder(default, setter(into, strip_option))]
    pub prefix: Option<String>,
    /// Delete destination files not present in source.
    #[builder(default = false)]
    pub delete: bool,
    /// Only compare sizes, ignore modification times.
    #[builder(default = false)]
    pub ignore_times: bool,
    /// Only compare modification times, ignore sizes.
    #[builder(default = false)]
    pub ignore_sizes: bool,
    /// Only sync files that already exist at destination.
    #[builder(default = false)]
    pub existing: bool,
    /// Skip files that already exist at destination.
    #[builder(default = false)]
    pub ignore_existing: bool,
    /// Include patterns (fnmatch/glob-style).
    #[builder(default)]
    pub include: Vec<String>,
    /// Exclude patterns (fnmatch/glob-style).
    #[builder(default)]
    pub exclude: Vec<String>,
    /// Include skip operations in the returned plan.
    #[builder(default = false)]
    pub verbose: bool,
    /// Progress handler for upload/download tracking.
    #[builder(default)]
    pub progress: Progress,
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p huggingface-hub --all-features`
Expected: compiles successfully (the types are defined but not yet used).

- [ ] **Step 3: Commit**

```bash
git add huggingface_hub/src/types/bucket_params.rs
git commit -m "feat: add BucketSyncParams and SyncDirection types"
```

---

### Task 2: Add `SyncPlan`, `SyncOperation`, `SyncAction` types

**Files:**
- Create: `huggingface_hub/src/types/sync.rs`
- Modify: `huggingface_hub/src/types/mod.rs`

- [ ] **Step 1: Create `huggingface_hub/src/types/sync.rs`**

```rust
use std::collections::HashMap;

use crate::types::bucket_params::SyncDirection;
use crate::types::buckets::BucketTreeEntry;

/// Action to perform for a single file during sync.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncAction {
    Upload,
    Download,
    Delete,
    Skip,
}

/// A single operation within a sync plan.
#[derive(Debug, Clone)]
pub struct SyncOperation {
    /// What action to take.
    pub action: SyncAction,
    /// Relative file path (forward-slash separated).
    pub path: String,
    /// File size in bytes, if known.
    pub size: Option<u64>,
    /// Human-readable reason for this action (e.g. "new file", "size differs", "identical").
    pub reason: String,
}

/// The computed sync plan — describes what will happen (or has happened) during a sync.
///
/// Returned by [`HFBucket::sync`](crate::bucket::HFBucket::sync).
#[derive(Debug, Clone)]
pub struct SyncPlan {
    /// Sync direction that produced this plan.
    pub direction: SyncDirection,
    /// All operations in the plan.
    pub operations: Vec<SyncOperation>,
    /// Bucket tree entries for download operations, keyed by relative path.
    /// Used internally during execution to avoid re-fetching metadata.
    pub(crate) download_entries: HashMap<String, BucketTreeEntry>,
}

impl SyncPlan {
    pub fn uploads(&self) -> usize {
        self.operations.iter().filter(|op| op.action == SyncAction::Upload).count()
    }

    pub fn downloads(&self) -> usize {
        self.operations.iter().filter(|op| op.action == SyncAction::Download).count()
    }

    pub fn deletes(&self) -> usize {
        self.operations.iter().filter(|op| op.action == SyncAction::Delete).count()
    }

    pub fn skips(&self) -> usize {
        self.operations.iter().filter(|op| op.action == SyncAction::Skip).count()
    }

    /// Total bytes to transfer (upload + download operations).
    pub fn transfer_bytes(&self) -> u64 {
        self.operations
            .iter()
            .filter(|op| op.action == SyncAction::Upload || op.action == SyncAction::Download)
            .filter_map(|op| op.size)
            .sum()
    }
}
```

- [ ] **Step 2: Register the module in `huggingface_hub/src/types/mod.rs`**

Add `pub mod sync;` after the `pub mod buckets;` line. Add `pub use sync::*;` after the `pub use buckets::*;` line.

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p huggingface-hub --all-features`
Expected: compiles successfully.

- [ ] **Step 4: Commit**

```bash
git add huggingface_hub/src/types/sync.rs huggingface_hub/src/types/mod.rs
git commit -m "feat: add SyncPlan, SyncOperation, SyncAction types"
```

---

### Task 3: Add unit tests for `SyncPlan` summary methods

**Files:**
- Modify: `huggingface_hub/src/types/sync.rs`

- [ ] **Step 1: Add tests module at the bottom of `sync.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn make_plan(ops: Vec<(SyncAction, Option<u64>)>) -> SyncPlan {
        SyncPlan {
            direction: SyncDirection::Upload,
            operations: ops
                .into_iter()
                .enumerate()
                .map(|(i, (action, size))| SyncOperation {
                    action,
                    path: format!("file_{i}.txt"),
                    size,
                    reason: "test".to_string(),
                })
                .collect(),
            download_entries: HashMap::new(),
        }
    }

    #[test]
    fn test_plan_counts() {
        let plan = make_plan(vec![
            (SyncAction::Upload, Some(100)),
            (SyncAction::Upload, Some(200)),
            (SyncAction::Download, Some(300)),
            (SyncAction::Delete, Some(50)),
            (SyncAction::Skip, None),
        ]);
        assert_eq!(plan.uploads(), 2);
        assert_eq!(plan.downloads(), 1);
        assert_eq!(plan.deletes(), 1);
        assert_eq!(plan.skips(), 1);
    }

    #[test]
    fn test_transfer_bytes() {
        let plan = make_plan(vec![
            (SyncAction::Upload, Some(100)),
            (SyncAction::Download, Some(300)),
            (SyncAction::Delete, Some(50)),
            (SyncAction::Skip, None),
        ]);
        assert_eq!(plan.transfer_bytes(), 400);
    }

    #[test]
    fn test_empty_plan() {
        let plan = make_plan(vec![]);
        assert_eq!(plan.uploads(), 0);
        assert_eq!(plan.downloads(), 0);
        assert_eq!(plan.deletes(), 0);
        assert_eq!(plan.skips(), 0);
        assert_eq!(plan.transfer_bytes(), 0);
    }
}
```

- [ ] **Step 2: Run the tests**

Run: `cargo test -p huggingface-hub -- types::sync::tests`
Expected: all 3 tests pass.

- [ ] **Step 3: Commit**

```bash
git add huggingface_hub/src/types/sync.rs
git commit -m "test: add unit tests for SyncPlan summary methods"
```

---

### Task 4: Implement sync core — plan computation and execution

This is the largest task. It implements the `sync()` method on `HFBucket` with all private helpers: local file listing, remote file listing, filter matching, file comparison, plan computation, and plan execution.

**Files:**
- Create: `huggingface_hub/src/api/sync.rs`
- Modify: `huggingface_hub/src/api/mod.rs`

- [ ] **Step 1: Create `huggingface_hub/src/api/sync.rs`**

```rust
use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::time::UNIX_EPOCH;

use futures::StreamExt;
use globset::Glob;

use crate::bucket::HFBucket;
use crate::error::{HFError, Result};
use crate::types::{
    BucketDownloadFilesParams, BucketSyncParams, BucketTreeEntry, ListBucketTreeParams, SyncAction, SyncDirection,
    SyncOperation, SyncPlan,
};

const SYNC_TIME_WINDOW_MS: f64 = 1000.0;

impl HFBucket {
    /// Sync files between a local directory and this bucket.
    ///
    /// Computes a plan by comparing local and remote file listings,
    /// executes the plan (uploads, downloads, deletes), and returns
    /// the plan for inspection.
    #[cfg(feature = "xet")]
    pub async fn sync(&self, params: &BucketSyncParams) -> Result<SyncPlan> {
        validate_params(params)?;

        let include_matchers = compile_patterns(&params.include)?;
        let exclude_matchers = compile_patterns(&params.exclude)?;

        let mut plan = match params.direction {
            SyncDirection::Upload => self.compute_upload_plan(params, &include_matchers, &exclude_matchers).await?,
            SyncDirection::Download => self.compute_download_plan(params, &include_matchers, &exclude_matchers).await?,
        };

        self.execute_plan(&mut plan, params).await?;
        Ok(plan)
    }

    /// Sync stub when xet feature is disabled.
    #[cfg(not(feature = "xet"))]
    pub async fn sync(&self, _params: &BucketSyncParams) -> Result<SyncPlan> {
        Err(HFError::XetNotEnabled)
    }

    #[cfg(feature = "xet")]
    async fn compute_upload_plan(
        &self,
        params: &BucketSyncParams,
        include: &[globset::GlobMatcher],
        exclude: &[globset::GlobMatcher],
    ) -> Result<SyncPlan> {
        let local_files = list_local_files(&params.local_path, include, exclude)?;
        let (remote_files, _entry_map) = self.list_remote_files(params.prefix.as_deref(), include, exclude).await?;

        let all_paths: BTreeSet<&String> = local_files.keys().chain(remote_files.keys()).collect();
        let mut operations = Vec::new();

        for path in all_paths {
            let local = local_files.get(path);
            let remote = remote_files.get(path);

            match (local, remote) {
                (Some((size, _mtime)), None) => {
                    if params.existing {
                        if params.verbose {
                            operations.push(SyncOperation {
                                action: SyncAction::Skip,
                                path: path.clone(),
                                size: Some(*size),
                                reason: "new file (--existing)".to_string(),
                            });
                        }
                    } else {
                        operations.push(SyncOperation {
                            action: SyncAction::Upload,
                            path: path.clone(),
                            size: Some(*size),
                            reason: "new file".to_string(),
                        });
                    }
                },
                (Some((local_size, local_mtime)), Some((remote_size, remote_mtime))) => {
                    if let Some(op) = compare_files(
                        path,
                        SyncAction::Upload,
                        *local_size,
                        *local_mtime,
                        *remote_size,
                        *remote_mtime,
                        params,
                    ) {
                        operations.push(op);
                    }
                },
                (None, Some((size, _mtime))) => {
                    if params.delete {
                        operations.push(SyncOperation {
                            action: SyncAction::Delete,
                            path: path.clone(),
                            size: Some(*size),
                            reason: "not in source (--delete)".to_string(),
                        });
                    }
                },
                (None, None) => unreachable!(),
            }
        }

        Ok(SyncPlan {
            direction: SyncDirection::Upload,
            operations,
            download_entries: HashMap::new(),
        })
    }

    #[cfg(feature = "xet")]
    async fn compute_download_plan(
        &self,
        params: &BucketSyncParams,
        include: &[globset::GlobMatcher],
        exclude: &[globset::GlobMatcher],
    ) -> Result<SyncPlan> {
        let (remote_files, entry_map) = self.list_remote_files(params.prefix.as_deref(), include, exclude).await?;

        let local_files = if params.local_path.is_dir() {
            list_local_files(&params.local_path, include, exclude)?
        } else {
            HashMap::new()
        };

        let all_paths: BTreeSet<&String> = remote_files.keys().chain(local_files.keys()).collect();
        let mut operations = Vec::new();
        let mut download_entries = HashMap::new();

        for path in all_paths {
            let remote = remote_files.get(path);
            let local = local_files.get(path);

            match (remote, local) {
                (Some((size, _mtime)), None) => {
                    if params.existing {
                        if params.verbose {
                            operations.push(SyncOperation {
                                action: SyncAction::Skip,
                                path: path.clone(),
                                size: Some(*size),
                                reason: "new file (--existing)".to_string(),
                            });
                        }
                    } else {
                        operations.push(SyncOperation {
                            action: SyncAction::Download,
                            path: path.clone(),
                            size: Some(*size),
                            reason: "new file".to_string(),
                        });
                        if let Some(entry) = entry_map.get(path) {
                            download_entries.insert(path.clone(), entry.clone());
                        }
                    }
                },
                (Some((remote_size, remote_mtime)), Some((local_size, local_mtime))) => {
                    if let Some(op) = compare_files(
                        path,
                        SyncAction::Download,
                        *remote_size,
                        *remote_mtime,
                        *local_size,
                        *local_mtime,
                        params,
                    ) {
                        if op.action == SyncAction::Download {
                            if let Some(entry) = entry_map.get(path) {
                                download_entries.insert(path.clone(), entry.clone());
                            }
                        }
                        operations.push(op);
                    }
                },
                (None, Some((size, _mtime))) => {
                    if params.delete {
                        operations.push(SyncOperation {
                            action: SyncAction::Delete,
                            path: path.clone(),
                            size: Some(*size),
                            reason: "not in source (--delete)".to_string(),
                        });
                    }
                },
                (None, None) => unreachable!(),
            }
        }

        Ok(SyncPlan {
            direction: SyncDirection::Download,
            operations,
            download_entries,
        })
    }

    #[cfg(feature = "xet")]
    async fn execute_plan(&self, plan: &mut SyncPlan, params: &BucketSyncParams) -> Result<()> {
        match plan.direction {
            SyncDirection::Upload => self.execute_upload_plan(plan, params).await,
            SyncDirection::Download => self.execute_download_plan(plan, params).await,
        }
    }

    #[cfg(feature = "xet")]
    async fn execute_upload_plan(&self, plan: &SyncPlan, params: &BucketSyncParams) -> Result<()> {
        let prefix = params.prefix.as_deref().map(|p| p.trim_end_matches('/'));

        let upload_files: Vec<(std::path::PathBuf, String)> = plan
            .operations
            .iter()
            .filter(|op| op.action == SyncAction::Upload)
            .map(|op| {
                let local_path = params.local_path.join(&op.path);
                let remote_path = match prefix {
                    Some(p) => format!("{}/{}", p, op.path),
                    None => op.path.clone(),
                };
                (local_path, remote_path)
            })
            .collect();

        let delete_paths: Vec<String> = plan
            .operations
            .iter()
            .filter(|op| op.action == SyncAction::Delete)
            .map(|op| match prefix {
                Some(p) => format!("{}/{}", p, op.path),
                None => op.path.clone(),
            })
            .collect();

        if !upload_files.is_empty() {
            self.upload_files(&upload_files, &params.progress).await?;
        }
        if !delete_paths.is_empty() {
            self.delete_files(&delete_paths).await?;
        }

        Ok(())
    }

    #[cfg(feature = "xet")]
    async fn execute_download_plan(&self, plan: &SyncPlan, params: &BucketSyncParams) -> Result<()> {
        let prefix = params.prefix.as_deref().map(|p| p.trim_end_matches('/'));

        let download_files: Vec<(String, std::path::PathBuf)> = plan
            .operations
            .iter()
            .filter(|op| op.action == SyncAction::Download)
            .map(|op| {
                let local_path = params.local_path.join(&op.path);
                let remote_path = match prefix {
                    Some(p) => format!("{}/{}", p, op.path),
                    None => op.path.clone(),
                };
                (remote_path, local_path)
            })
            .collect();

        if !download_files.is_empty() {
            // Create parent directories for all download targets
            for (_, local_path) in &download_files {
                if let Some(parent) = local_path.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        HFError::Io(std::io::Error::new(e.kind(), format!("Failed to create directory {}: {e}", parent.display())))
                    })?;
                }
            }
            let dl_params = BucketDownloadFilesParams::builder().files(download_files).build();
            self.download_files(&dl_params, &params.progress).await?;
        }

        // Delete local files
        let delete_paths: Vec<&SyncOperation> = plan
            .operations
            .iter()
            .filter(|op| op.action == SyncAction::Delete)
            .collect();

        for op in &delete_paths {
            let local_path = params.local_path.join(&op.path);
            if local_path.exists() {
                std::fs::remove_file(&local_path).map_err(|e| {
                    HFError::Io(std::io::Error::new(
                        e.kind(),
                        format!("Failed to delete {}: {e}", local_path.display()),
                    ))
                })?;
                // Clean up empty parent directories
                let mut parent = local_path.parent();
                while let Some(dir) = parent {
                    if dir == params.local_path {
                        break;
                    }
                    if std::fs::remove_dir(dir).is_err() {
                        break;
                    }
                    parent = dir.parent();
                }
            }
        }

        Ok(())
    }

    /// List remote files, stripping the prefix to produce relative paths.
    #[cfg(feature = "xet")]
    async fn list_remote_files(
        &self,
        prefix: Option<&str>,
        include: &[globset::GlobMatcher],
        exclude: &[globset::GlobMatcher],
    ) -> Result<(HashMap<String, (u64, f64)>, HashMap<String, BucketTreeEntry>)> {
        let list_params = ListBucketTreeParams {
            prefix: prefix.map(|s| s.to_string()),
            recursive: Some(true),
        };

        let stream = self.list_tree(&list_params)?;
        futures::pin_mut!(stream);

        let mut files: HashMap<String, (u64, f64)> = HashMap::new();
        let mut entry_map: HashMap<String, BucketTreeEntry> = HashMap::new();

        while let Some(entry) = stream.next().await {
            let entry = entry?;
            if let BucketTreeEntry::File {
                ref path,
                size,
                ref mtime,
                ..
            } = entry
            {
                let rel_path = strip_prefix(path, prefix);
                if let Some(rel_path) = rel_path {
                    if matches_filters(&rel_path, include, exclude) {
                        let mtime_ms = mtime.as_deref().map(parse_iso_mtime).unwrap_or(0.0);
                        files.insert(rel_path.clone(), (size, mtime_ms));
                        entry_map.insert(rel_path, entry.clone());
                    }
                }
            }
        }

        Ok((files, entry_map))
    }
}

fn validate_params(params: &BucketSyncParams) -> Result<()> {
    if params.ignore_times && params.ignore_sizes {
        return Err(HFError::InvalidParameter(
            "Cannot specify both ignore_times and ignore_sizes".to_string(),
        ));
    }
    if params.existing && params.ignore_existing {
        return Err(HFError::InvalidParameter(
            "Cannot specify both existing and ignore_existing".to_string(),
        ));
    }
    if params.direction == SyncDirection::Upload && !params.local_path.is_dir() {
        return Err(HFError::InvalidParameter(format!(
            "Local path must be an existing directory for upload: {}",
            params.local_path.display()
        )));
    }
    Ok(())
}

fn compile_patterns(patterns: &[String]) -> Result<Vec<globset::GlobMatcher>> {
    patterns
        .iter()
        .map(|p| {
            Glob::new(p)
                .map(|g| g.compile_matcher())
                .map_err(|e| HFError::InvalidParameter(format!("Invalid glob pattern '{p}': {e}")))
        })
        .collect()
}

fn matches_filters(path: &str, include: &[globset::GlobMatcher], exclude: &[globset::GlobMatcher]) -> bool {
    for pattern in exclude {
        if pattern.is_match(path) {
            return false;
        }
    }
    for pattern in include {
        if pattern.is_match(path) {
            return true;
        }
    }
    if !include.is_empty() {
        return false;
    }
    true
}

/// List local files recursively, returning (relative_path -> (size, mtime_ms)).
fn list_local_files(
    local_path: &Path,
    include: &[globset::GlobMatcher],
    exclude: &[globset::GlobMatcher],
) -> Result<HashMap<String, (u64, f64)>> {
    let mut files = HashMap::new();
    list_local_files_recursive(local_path, local_path, include, exclude, &mut files)?;
    Ok(files)
}

fn list_local_files_recursive(
    base: &Path,
    dir: &Path,
    include: &[globset::GlobMatcher],
    exclude: &[globset::GlobMatcher],
    files: &mut HashMap<String, (u64, f64)>,
) -> Result<()> {
    let entries = std::fs::read_dir(dir).map_err(|e| {
        HFError::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to read directory {}: {e}", dir.display()),
        ))
    })?;

    for entry in entries {
        let entry = entry.map_err(HFError::Io)?;
        let path = entry.path();
        if path.is_dir() {
            list_local_files_recursive(base, &path, include, exclude, files)?;
        } else if path.is_file() {
            let rel_path = path
                .strip_prefix(base)
                .map_err(|e| HFError::Other(format!("Failed to compute relative path: {e}")))?;
            // Normalize to forward slashes
            let rel_str: String = rel_path.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/");

            if matches_filters(&rel_str, include, exclude) {
                let metadata = std::fs::metadata(&path).map_err(HFError::Io)?;
                let size = metadata.len();
                let mtime_ms = metadata
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as f64)
                    .unwrap_or(0.0);
                files.insert(rel_str, (size, mtime_ms));
            }
        }
    }
    Ok(())
}

/// Strip a prefix from a bucket path to get the relative path.
/// Returns None if the path doesn't belong under the prefix.
fn strip_prefix(path: &str, prefix: Option<&str>) -> Option<String> {
    match prefix {
        Some(p) => {
            if path.starts_with(&format!("{p}/")) {
                Some(path[p.len() + 1..].to_string())
            } else if path == p {
                // Exact match: use the filename portion
                let filename = path.rsplit('/').next().unwrap_or(path);
                Some(filename.to_string())
            } else {
                None
            }
        },
        None => Some(path.to_string()),
    }
}

/// Compare two files and return a SyncOperation, or None if verbose is off and it's a skip.
fn compare_files(
    path: &str,
    action: SyncAction,
    source_size: u64,
    source_mtime: f64,
    dest_size: u64,
    dest_mtime: f64,
    params: &BucketSyncParams,
) -> Option<SyncOperation> {
    if params.ignore_existing {
        return if params.verbose {
            Some(SyncOperation {
                action: SyncAction::Skip,
                path: path.to_string(),
                size: Some(source_size),
                reason: "exists on receiver (--ignore-existing)".to_string(),
            })
        } else {
            None
        };
    }

    let size_differs = source_size != dest_size;
    let source_newer = (source_mtime - dest_mtime) > SYNC_TIME_WINDOW_MS;

    let (should_transfer, reason) = if params.ignore_sizes {
        if source_newer {
            let label = match action {
                SyncAction::Upload => "local newer",
                SyncAction::Download => "remote newer",
                _ => "source newer",
            };
            (true, label.to_string())
        } else {
            let dest_newer = (dest_mtime - source_mtime) > SYNC_TIME_WINDOW_MS;
            let reason = if dest_newer {
                match action {
                    SyncAction::Upload => "remote newer",
                    SyncAction::Download => "local newer",
                    _ => "dest newer",
                }
            } else {
                "same mtime"
            };
            (false, reason.to_string())
        }
    } else if params.ignore_times {
        if size_differs {
            (true, "size differs".to_string())
        } else {
            (false, "same size".to_string())
        }
    } else {
        // Default: compare both
        if size_differs || source_newer {
            let reason = if size_differs { "size differs" } else {
                match action {
                    SyncAction::Upload => "local newer",
                    SyncAction::Download => "remote newer",
                    _ => "source newer",
                }
            };
            (true, reason.to_string())
        } else {
            (false, "identical".to_string())
        }
    };

    if should_transfer {
        Some(SyncOperation {
            action,
            path: path.to_string(),
            size: Some(source_size),
            reason,
        })
    } else if params.verbose {
        Some(SyncOperation {
            action: SyncAction::Skip,
            path: path.to_string(),
            size: Some(source_size),
            reason,
        })
    } else {
        None
    }
}

/// Parse an ISO 8601 datetime string to milliseconds since epoch.
/// Returns 0.0 on parse failure.
fn parse_iso_mtime(s: &str) -> f64 {
    // Expected format: "2024-01-15T10:30:00Z" or "2024-01-15T10:30:00.000Z"
    // or with offset: "2024-01-15T10:30:00+00:00"
    //
    // We use a simple approach: try to parse with the `time` crate-free method.
    // The Hub returns ISO 8601 with 'Z' suffix or '+00:00'.
    let s = s.trim();
    let s = s.strip_suffix('Z').or_else(|| s.strip_suffix("+00:00")).unwrap_or(s);

    // Parse "YYYY-MM-DDTHH:MM:SS" or "YYYY-MM-DDTHH:MM:SS.fff"
    let parts: Vec<&str> = s.splitn(2, 'T').collect();
    if parts.len() != 2 {
        return 0.0;
    }
    let date_parts: Vec<&str> = parts[0].split('-').collect();
    let time_str = parts[1];

    if date_parts.len() != 3 {
        return 0.0;
    }

    let year: i64 = date_parts[0].parse().unwrap_or(0);
    let month: u32 = date_parts[1].parse().unwrap_or(0);
    let day: u32 = date_parts[2].parse().unwrap_or(0);

    let time_parts: Vec<&str> = time_str.splitn(2, '.').collect();
    let hms: Vec<&str> = time_parts[0].split(':').collect();
    if hms.len() != 3 {
        return 0.0;
    }
    let hour: u32 = hms[0].parse().unwrap_or(0);
    let minute: u32 = hms[1].parse().unwrap_or(0);
    let second: u32 = hms[2].parse().unwrap_or(0);

    let frac_ms: f64 = if time_parts.len() > 1 {
        let frac_str = time_parts[1];
        let frac: f64 = frac_str.parse().unwrap_or(0.0);
        frac / 10f64.powi(frac_str.len() as i32) * 1000.0
    } else {
        0.0
    };

    // Convert to days since epoch using a simplified algorithm
    let days = days_from_civil(year, month, day);
    let secs = days * 86400 + (hour as i64) * 3600 + (minute as i64) * 60 + (second as i64);

    (secs as f64) * 1000.0 + frac_ms
}

/// Days from 1970-01-01. Civil calendar algorithm.
/// Adapted from Howard Hinnant's date algorithms.
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let m = if month <= 2 { month + 9 } else { month - 3 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u32;
    let doy = (153 * m + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe as i64 - 719468
}
```

- [ ] **Step 2: Register the module in `huggingface_hub/src/api/mod.rs`**

Add `pub mod sync;` (after `pub mod buckets;` or at the end).

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p huggingface-hub --all-features`
Expected: compiles successfully.

- [ ] **Step 4: Commit**

```bash
git add huggingface_hub/src/api/sync.rs huggingface_hub/src/api/mod.rs
git commit -m "feat: implement HFBucket::sync() with plan computation and execution"
```

---

### Task 5: Add unit tests for sync helpers

**Files:**
- Modify: `huggingface_hub/src/api/sync.rs`

- [ ] **Step 1: Add tests module at the bottom of `sync.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_filters_no_patterns() {
        assert!(matches_filters("anything.txt", &[], &[]));
    }

    #[test]
    fn test_matches_filters_include_only() {
        let include = compile_patterns(&["*.txt".to_string()]).unwrap();
        assert!(matches_filters("file.txt", &include, &[]));
        assert!(!matches_filters("file.bin", &include, &[]));
    }

    #[test]
    fn test_matches_filters_exclude_only() {
        let exclude = compile_patterns(&["*.log".to_string()]).unwrap();
        assert!(matches_filters("file.txt", &[], &exclude));
        assert!(!matches_filters("file.log", &[], &exclude));
    }

    #[test]
    fn test_matches_filters_exclude_takes_precedence() {
        let include = compile_patterns(&["*.txt".to_string()]).unwrap();
        let exclude = compile_patterns(&["secret.*".to_string()]).unwrap();
        assert!(matches_filters("file.txt", &include, &exclude));
        assert!(!matches_filters("secret.txt", &include, &exclude));
    }

    #[test]
    fn test_strip_prefix_with_prefix() {
        assert_eq!(strip_prefix("data/subdir/file.txt", Some("data")), Some("subdir/file.txt".to_string()));
    }

    #[test]
    fn test_strip_prefix_exact_match() {
        assert_eq!(strip_prefix("data/file.txt", Some("data/file.txt")), Some("file.txt".to_string()));
    }

    #[test]
    fn test_strip_prefix_no_match() {
        assert_eq!(strip_prefix("other/file.txt", Some("data")), None);
    }

    #[test]
    fn test_strip_prefix_similar_name_no_match() {
        // "submarine.txt" should NOT match prefix "sub"
        assert_eq!(strip_prefix("submarine.txt", Some("sub")), None);
    }

    #[test]
    fn test_strip_prefix_no_prefix() {
        assert_eq!(strip_prefix("file.txt", None), Some("file.txt".to_string()));
    }

    #[test]
    fn test_parse_iso_mtime_z_suffix() {
        let ms = parse_iso_mtime("2024-01-15T10:30:00Z");
        assert!(ms > 0.0);
        // 2024-01-15T10:30:00Z = 1705312200000 ms
        assert!((ms - 1705312200000.0).abs() < 1000.0);
    }

    #[test]
    fn test_parse_iso_mtime_offset_suffix() {
        let ms = parse_iso_mtime("2024-01-15T10:30:00+00:00");
        assert!(ms > 0.0);
        assert!((ms - 1705312200000.0).abs() < 1000.0);
    }

    #[test]
    fn test_parse_iso_mtime_with_fractional() {
        let ms = parse_iso_mtime("2024-01-15T10:30:00.500Z");
        // Should be ~500ms more than the integer version
        let base = parse_iso_mtime("2024-01-15T10:30:00Z");
        assert!((ms - base - 500.0).abs() < 10.0);
    }

    #[test]
    fn test_parse_iso_mtime_invalid() {
        assert_eq!(parse_iso_mtime("not-a-date"), 0.0);
        assert_eq!(parse_iso_mtime(""), 0.0);
    }

    #[test]
    fn test_compare_files_default_identical() {
        let params = BucketSyncParams::builder()
            .local_path(std::path::PathBuf::from("/tmp"))
            .direction(SyncDirection::Upload)
            .build();
        let mtime = 1000000.0;
        let result = compare_files("file.txt", SyncAction::Upload, 100, mtime, 100, mtime, &params);
        // With verbose=false, skips return None
        assert!(result.is_none());
    }

    #[test]
    fn test_compare_files_default_identical_verbose() {
        let params = BucketSyncParams::builder()
            .local_path(std::path::PathBuf::from("/tmp"))
            .direction(SyncDirection::Upload)
            .verbose(true)
            .build();
        let mtime = 1000000.0;
        let result = compare_files("file.txt", SyncAction::Upload, 100, mtime, 100, mtime, &params).unwrap();
        assert_eq!(result.action, SyncAction::Skip);
        assert_eq!(result.reason, "identical");
    }

    #[test]
    fn test_compare_files_size_differs() {
        let params = BucketSyncParams::builder()
            .local_path(std::path::PathBuf::from("/tmp"))
            .direction(SyncDirection::Upload)
            .build();
        let result = compare_files("file.txt", SyncAction::Upload, 200, 1000000.0, 100, 1000000.0, &params).unwrap();
        assert_eq!(result.action, SyncAction::Upload);
        assert_eq!(result.reason, "size differs");
    }

    #[test]
    fn test_compare_files_source_newer() {
        let params = BucketSyncParams::builder()
            .local_path(std::path::PathBuf::from("/tmp"))
            .direction(SyncDirection::Upload)
            .build();
        // source_mtime is >1000ms newer than dest
        let result = compare_files("file.txt", SyncAction::Upload, 100, 5000.0, 100, 2000.0, &params).unwrap();
        assert_eq!(result.action, SyncAction::Upload);
        assert_eq!(result.reason, "local newer");
    }

    #[test]
    fn test_compare_files_within_safety_window() {
        let params = BucketSyncParams::builder()
            .local_path(std::path::PathBuf::from("/tmp"))
            .direction(SyncDirection::Upload)
            .verbose(true)
            .build();
        // source_mtime is only 500ms newer — within safety window
        let result = compare_files("file.txt", SyncAction::Upload, 100, 2500.0, 100, 2000.0, &params).unwrap();
        assert_eq!(result.action, SyncAction::Skip);
        assert_eq!(result.reason, "identical");
    }

    #[test]
    fn test_compare_files_ignore_times() {
        let params = BucketSyncParams::builder()
            .local_path(std::path::PathBuf::from("/tmp"))
            .direction(SyncDirection::Upload)
            .ignore_times(true)
            .verbose(true)
            .build();
        // Same size, different mtime — should skip
        let result = compare_files("file.txt", SyncAction::Upload, 100, 5000.0, 100, 1000.0, &params).unwrap();
        assert_eq!(result.action, SyncAction::Skip);
        assert_eq!(result.reason, "same size");
    }

    #[test]
    fn test_compare_files_ignore_sizes() {
        let params = BucketSyncParams::builder()
            .local_path(std::path::PathBuf::from("/tmp"))
            .direction(SyncDirection::Upload)
            .ignore_sizes(true)
            .verbose(true)
            .build();
        // Different size, same mtime — should skip
        let result = compare_files("file.txt", SyncAction::Upload, 200, 1000.0, 100, 1000.0, &params).unwrap();
        assert_eq!(result.action, SyncAction::Skip);
        assert_eq!(result.reason, "same mtime");
    }

    #[test]
    fn test_compare_files_ignore_existing() {
        let params = BucketSyncParams::builder()
            .local_path(std::path::PathBuf::from("/tmp"))
            .direction(SyncDirection::Upload)
            .ignore_existing(true)
            .verbose(true)
            .build();
        let result = compare_files("file.txt", SyncAction::Upload, 200, 5000.0, 100, 1000.0, &params).unwrap();
        assert_eq!(result.action, SyncAction::Skip);
        assert_eq!(result.reason, "exists on receiver (--ignore-existing)");
    }

    #[test]
    fn test_compare_files_download_direction_labels() {
        let params = BucketSyncParams::builder()
            .local_path(std::path::PathBuf::from("/tmp"))
            .direction(SyncDirection::Download)
            .build();
        let result = compare_files("file.txt", SyncAction::Download, 100, 5000.0, 100, 2000.0, &params).unwrap();
        assert_eq!(result.action, SyncAction::Download);
        assert_eq!(result.reason, "remote newer");
    }

    #[test]
    fn test_validate_params_conflicting_ignore() {
        let params = BucketSyncParams::builder()
            .local_path(std::path::PathBuf::from("/tmp"))
            .direction(SyncDirection::Upload)
            .ignore_times(true)
            .ignore_sizes(true)
            .build();
        assert!(validate_params(&params).is_err());
    }

    #[test]
    fn test_validate_params_conflicting_existing() {
        let params = BucketSyncParams::builder()
            .local_path(std::path::PathBuf::from("/tmp"))
            .direction(SyncDirection::Upload)
            .existing(true)
            .ignore_existing(true)
            .build();
        assert!(validate_params(&params).is_err());
    }

    #[test]
    fn test_validate_params_upload_nonexistent_dir() {
        let params = BucketSyncParams::builder()
            .local_path(std::path::PathBuf::from("/nonexistent/path/for/test"))
            .direction(SyncDirection::Upload)
            .build();
        assert!(validate_params(&params).is_err());
    }

    #[test]
    fn test_validate_params_download_nonexistent_ok() {
        let params = BucketSyncParams::builder()
            .local_path(std::path::PathBuf::from("/nonexistent/path/for/test"))
            .direction(SyncDirection::Download)
            .build();
        assert!(validate_params(&params).is_ok());
    }

    #[test]
    fn test_list_local_files_basic() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "hello").unwrap();
        std::fs::create_dir_all(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub/b.txt"), "world!").unwrap();

        let files = list_local_files(dir.path(), &[], &[]).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files["a.txt"].0, 5); // "hello" = 5 bytes
        assert_eq!(files["sub/b.txt"].0, 6); // "world!" = 6 bytes
    }

    #[test]
    fn test_list_local_files_with_filter() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "hello").unwrap();
        std::fs::write(dir.path().join("b.log"), "world").unwrap();

        let include = compile_patterns(&["*.txt".to_string()]).unwrap();
        let files = list_local_files(dir.path(), &include, &[]).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files.contains_key("a.txt"));
    }
}
```

- [ ] **Step 2: Run the tests**

Run: `cargo test -p huggingface-hub -- api::sync::tests`
Expected: all tests pass.

- [ ] **Step 3: Commit**

```bash
git add huggingface_hub/src/api/sync.rs
git commit -m "test: add unit tests for sync helpers"
```

---

### Task 6: Add `sync` to the blocking `sync_api!` block

**Files:**
- Modify: `huggingface_hub/src/api/buckets.rs`

- [ ] **Step 1: Add `sync` to the `HFBucket` `sync_api!` block**

In the `sync_api!` block for `HFBucket -> HFBucketSync` (around line 452), add:

```rust
fn sync(&self, params: &BucketSyncParams) -> Result<SyncPlan>;
```

This requires adding the imports. Add `SyncPlan` to the existing imports at the top of `buckets.rs` (it's already re-exported through `types::*`).

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p huggingface-hub --all-features`
Expected: compiles successfully.

- [ ] **Step 3: Commit**

```bash
git add huggingface_hub/src/api/buckets.rs
git commit -m "feat: add sync to blocking API wrapper"
```

---

### Task 7: Add CLI `sync` subcommand

**Files:**
- Create: `huggingface_hub/src/bin/hfrs/commands/buckets/sync.rs`
- Modify: `huggingface_hub/src/bin/hfrs/commands/buckets/mod.rs`

- [ ] **Step 1: Create `huggingface_hub/src/bin/hfrs/commands/buckets/sync.rs`**

```rust
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::{BucketSyncParams, HFClient, Progress, SyncAction, SyncDirection};

use crate::output::CommandResult;
use crate::progress::CliProgressHandler;

#[derive(ClapArgs)]
pub struct Args {
    /// Source: local directory or hf://buckets/ns/name(/prefix)
    pub source: String,

    /// Destination: local directory or hf://buckets/ns/name(/prefix)
    pub dest: String,

    /// Delete destination files not present in source
    #[arg(long)]
    pub delete: bool,

    /// Only compare sizes, ignore modification times
    #[arg(long)]
    pub ignore_times: bool,

    /// Only compare modification times, ignore sizes
    #[arg(long)]
    pub ignore_sizes: bool,

    /// Only sync files that already exist at destination
    #[arg(long)]
    pub existing: bool,

    /// Skip files that already exist at destination
    #[arg(long)]
    pub ignore_existing: bool,

    /// Include files matching pattern (can be repeated)
    #[arg(long)]
    pub include: Vec<String>,

    /// Exclude files matching pattern (can be repeated)
    #[arg(long)]
    pub exclude: Vec<String>,

    /// Show per-file operations
    #[arg(short, long)]
    pub verbose: bool,

    /// Suppress output
    #[arg(short, long)]
    pub quiet: bool,
}

struct BucketRef {
    namespace: String,
    bucket_name: String,
    prefix: Option<String>,
}

fn parse_bucket_path(input: &str) -> Option<BucketRef> {
    let rest = input.strip_prefix("hf://buckets/")?;
    let parts: Vec<&str> = rest.splitn(3, '/').collect();
    if parts.len() < 2 || parts[0].is_empty() || parts[1].is_empty() {
        return None;
    }
    let prefix = if parts.len() == 3 && !parts[2].is_empty() {
        Some(parts[2].to_string())
    } else {
        None
    };
    Some(BucketRef {
        namespace: parts[0].to_string(),
        bucket_name: parts[1].to_string(),
        prefix,
    })
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GiB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

pub async fn execute(api: &HFClient, args: Args, multi: Option<indicatif::MultiProgress>) -> Result<CommandResult> {
    let src_is_bucket = args.source.starts_with("hf://buckets/");
    let dst_is_bucket = args.dest.starts_with("hf://buckets/");

    if src_is_bucket && dst_is_bucket {
        anyhow::bail!("Remote-to-remote sync is not supported.");
    }
    if !src_is_bucket && !dst_is_bucket {
        anyhow::bail!("One of source or dest must be a bucket path (hf://buckets/...).");
    }

    let (bucket_ref, local_path, direction) = if dst_is_bucket {
        let b = parse_bucket_path(&args.dest).ok_or_else(|| anyhow::anyhow!("Invalid bucket path: {}", args.dest))?;
        (b, PathBuf::from(&args.source), SyncDirection::Upload)
    } else {
        let b =
            parse_bucket_path(&args.source).ok_or_else(|| anyhow::anyhow!("Invalid bucket path: {}", args.source))?;
        (b, PathBuf::from(&args.dest), SyncDirection::Download)
    };

    let handler: Progress = if args.quiet {
        None
    } else if let Some(multi) = multi {
        Some(Arc::new(CliProgressHandler::new(multi)))
    } else {
        None
    };

    let bucket = api.bucket(&bucket_ref.namespace, &bucket_ref.bucket_name);
    let mut builder = BucketSyncParams::builder()
        .local_path(local_path)
        .direction(direction)
        .delete(args.delete)
        .ignore_times(args.ignore_times)
        .ignore_sizes(args.ignore_sizes)
        .existing(args.existing)
        .ignore_existing(args.ignore_existing)
        .include(args.include)
        .exclude(args.exclude)
        .verbose(args.verbose)
        .progress(handler);
    if let Some(prefix) = bucket_ref.prefix {
        builder = builder.prefix(prefix);
    }
    let params = builder.build();

    let plan = bucket.sync(&params).await?;

    if args.quiet {
        return Ok(CommandResult::Silent);
    }

    if args.verbose {
        for op in &plan.operations {
            let action_str = match op.action {
                SyncAction::Upload => "upload",
                SyncAction::Download => "download",
                SyncAction::Delete => "delete",
                SyncAction::Skip => "skip",
            };
            println!("  {}: {} ({})", action_str, op.path, op.reason);
        }
    }

    let summary = format!(
        "Synced: {} uploaded, {} downloaded, {} deleted, {} skipped ({})",
        plan.uploads(),
        plan.downloads(),
        plan.deletes(),
        plan.skips(),
        format_bytes(plan.transfer_bytes()),
    );

    Ok(CommandResult::Raw(summary))
}
```

- [ ] **Step 2: Register the subcommand in `mod.rs`**

In `huggingface_hub/src/bin/hfrs/commands/buckets/mod.rs`:

Add `pub mod sync;` at the top with the other module declarations.

Add the `Sync` variant to the `BucketsCommand` enum:

```rust
    /// Sync files between a local directory and a bucket
    Sync(sync::Args),
```

Add the dispatch in the `execute` function's match:

```rust
        BucketsCommand::Sync(a) => sync::execute(api, a, multi).await,
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p huggingface-hub --all-features`
Expected: compiles successfully.

- [ ] **Step 4: Commit**

```bash
git add huggingface_hub/src/bin/hfrs/commands/buckets/sync.rs huggingface_hub/src/bin/hfrs/commands/buckets/mod.rs
git commit -m "feat: add hfrs buckets sync CLI subcommand"
```

---

### Task 8: Format, lint, build release

**Files:** None (validation only)

- [ ] **Step 1: Format**

Run: `cargo +nightly fmt`

- [ ] **Step 2: Lint**

Run: `cargo clippy -p huggingface-hub --all-features -- -D warnings`
Expected: no warnings.

- [ ] **Step 3: Run all unit tests**

Run: `cargo test -p huggingface-hub`
Expected: all tests pass.

- [ ] **Step 4: Build release binary**

Run: `cargo build -p huggingface-hub --release`
Expected: compiles successfully.

- [ ] **Step 5: Verify CLI help**

Run: `./target/release/hfrs buckets sync --help`
Expected: shows usage with all flags (--delete, --ignore-times, --ignore-sizes, --existing, --ignore-existing, --include, --exclude, --verbose, --quiet).

- [ ] **Step 6: Fix any issues and commit**

If formatting or clippy produced changes:

```bash
git add -A
git commit -m "fix: formatting and clippy fixes for bucket sync"
```

---

### Task 9: Update CLAUDE.md project layout

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: Update the Project Layout section**

Add to the `types/` listing:
```
│   │   │   ├── sync.rs             # SyncPlan, SyncOperation, SyncAction types
```

Add to the `api/` listing:
```
│   │       ├── sync.rs             # HFBucket::sync() — plan computation and execution
```

Add to the `commands/buckets/` listing:
```
│   │   ├── sync.rs
```

- [ ] **Step 2: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: update project layout with sync module"
```
