# Bucket Sync Design

## Summary

Add a `sync()` method to `HFBucket` that synchronizes files between a local directory and a bucket, mirroring the Python `huggingface_hub` library's `sync_bucket()` API. Includes comparison modes and filtering. Plan persistence (`--plan`/`--apply`/`--dry-run`) is deferred to a future iteration.

## Public API

### `HFBucket::sync`

```rust
impl HFBucket {
    /// Sync files between a local directory and this bucket.
    ///
    /// Computes a plan by comparing local and remote file listings,
    /// executes the plan (uploads, downloads, deletes), and returns
    /// the plan for inspection.
    pub async fn sync(&self, params: &BucketSyncParams, progress: &Progress) -> Result<SyncPlan>;
}
```

Returns the computed `SyncPlan` so callers can inspect the summary (counts, bytes transferred).

### `BucketSyncParams` — `types/bucket_params.rs`

```rust
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncDirection {
    /// Local directory -> bucket (upload).
    Upload,
    /// Bucket -> local directory (download).
    Download,
}
```

### `SyncPlan` and `SyncOperation` — new `types/sync.rs`

```rust
#[derive(Debug, Clone)]
pub struct SyncPlan {
    pub direction: SyncDirection,
    pub operations: Vec<SyncOperation>,
    /// Bucket tree entries for download operations (used internally during execution).
    pub(crate) download_entries: HashMap<String, BucketTreeEntry>,
}

impl SyncPlan {
    pub fn uploads(&self) -> usize;
    pub fn downloads(&self) -> usize;
    pub fn deletes(&self) -> usize;
    pub fn skips(&self) -> usize;
    /// Total bytes to transfer (upload + download operations).
    pub fn transfer_bytes(&self) -> u64;
}

#[derive(Debug, Clone)]
pub struct SyncOperation {
    pub action: SyncAction,
    pub path: String,
    pub size: Option<u64>,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncAction {
    Upload,
    Download,
    Delete,
    Skip,
}
```

## Algorithm

### Plan Computation

1. **List local files**: recursive `std::fs` walk of `local_path`, collecting `HashMap<String, (u64, f64)>` of `(rel_path -> (size, mtime_ms))`. Paths normalized to forward slashes.

2. **List remote files**: `self.list_tree(prefix, recursive=true)`, collecting `HashMap<String, (u64, f64)>` of `(rel_path -> (size, mtime_ms))` plus a `HashMap<String, BucketTreeEntry>` for download metadata. Prefix is stripped from paths. Directories are skipped.

3. **Filter**: compile `include`/`exclude` strings into `globset::Glob` matchers at the start (return `InvalidParameter` on malformed patterns). Apply to both local and remote listings:
   - Check exclude patterns first — if any match, exclude.
   - Check include patterns — if any match, include.
   - If include patterns were specified but none matched, exclude.
   - Default: include.

4. **Compare**: union the key sets, iterate sorted. For each path:
   - **Source only**: action (upload/download) with reason "new file", unless `existing` is set (skip).
   - **Both exist**: compare using the comparison function (see below).
   - **Dest only**: delete with reason "not in source (--delete)" if `delete` is set; otherwise omitted.

5. **Skips**: only included in `operations` if `verbose` is true (keeps the plan small by default).

### File Comparison

Identical to the Python implementation:

- **Safety window**: `SYNC_TIME_WINDOW_MS = 1000` (1 second). Source is "newer" only if `source_mtime - dest_mtime > 1000`.
- **Default** (compare both): transfer if size differs OR source is newer. Skip reason: "identical".
- **`ignore_times`**: transfer only if size differs. Skip reason: "same size".
- **`ignore_sizes`**: transfer only if source is newer. Skip reason: "same mtime" or "remote/local newer".
- **`ignore_existing`**: always skip with reason "exists on receiver".

### Plan Execution

- **Upload**: collect upload paths into `Vec<(PathBuf, String)>` (prepending prefix to remote paths), call `self.upload_files()`. Then collect delete paths and call `self.delete_files()`.
- **Download**: build `BucketDownloadFilesParams` from stored `BucketTreeEntry` data (avoids re-fetching metadata). Call `self.download_files()`. Then delete local files and clean up empty parent directories.

### Mtime Handling

- **Local files**: `std::fs::metadata().modified()` converted to milliseconds since epoch.
- **Remote files**: `BucketTreeEntry::File::mtime` is `Option<String>` (ISO 8601). Parsed to milliseconds. Defaults to 0 if absent.

## CLI

New subcommand: `hfrs buckets sync <source> <dest> [flags]`

```
hfrs buckets sync <source> <dest>
    [--delete]
    [--ignore-times]
    [--ignore-sizes]
    [--existing]
    [--ignore-existing]
    [--include <PATTERN>]...
    [--exclude <PATTERN>]...
    [--verbose / -v]
    [--quiet / -q]
```

- One of `source`/`dest` must be `hf://buckets/ns/name(/prefix)`, the other a local directory path.
- The CLI parses the bucket path to extract namespace, name, and optional prefix, determines `SyncDirection`, and builds `BucketSyncParams`.
- On completion, prints a summary line: `Synced: N uploaded, N downloaded, N deleted, N skipped (X bytes transferred)`.
- With `--verbose`, prints per-file operation lines before the summary.
- With `--quiet`, no output.

### Validation (CLI layer)

- Both source and dest required.
- One must be a bucket path, one must be local.
- `--ignore-times` and `--ignore-sizes` are mutually exclusive.
- `--existing` and `--ignore-existing` are mutually exclusive.
- Local source (for upload) must be an existing directory.

## Dependencies

- `globset` (already a dependency) for fnmatch-style pattern matching.
- ISO 8601 mtime parsing: use manual parsing or `time` crate. No new dependency required if parsing is simple enough (the format is fixed).

## Files to Create/Modify

- **New**: `huggingface_hub/src/types/sync.rs` — `SyncPlan`, `SyncOperation`, `SyncAction`
- **New**: `huggingface_hub/src/api/sync.rs` — `HFBucket::sync()` implementation + private helpers
- **New**: `huggingface_hub/src/bin/hfrs/commands/buckets/sync.rs` — CLI subcommand
- **Modify**: `huggingface_hub/src/types/bucket_params.rs` — add `BucketSyncParams`, `SyncDirection`
- **Modify**: `huggingface_hub/src/types/mod.rs` — add `pub mod sync;` and re-export
- **Modify**: `huggingface_hub/src/api/mod.rs` — add `pub mod sync;`
- **Modify**: `huggingface_hub/src/api/buckets.rs` — add `sync` to `sync_api!` block
- **Modify**: `huggingface_hub/src/lib.rs` — re-export new public types
- **Modify**: `huggingface_hub/src/bin/hfrs/commands/buckets/mod.rs` — add `Sync` variant + dispatch
- **Modify**: `huggingface_hub/Cargo.toml` — add dependencies if needed (e.g., `time` crate for mtime parsing)

## Not In Scope (Future Work)

- Plan persistence: `--plan FILE`, `--apply FILE`, `--dry-run`
- `--filter-from FILE` (filter file parsing)
- Bidirectional sync
- Example file in `huggingface_hub/examples/`
