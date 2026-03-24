# File System Cache Design

Interoperable file system cache for the Rust `huggingface-hub` library, matching the layout used by the Python `huggingface_hub` library and documented at https://huggingface.co/docs/hub/local-cache.

## Cache Directory Resolution

Priority order for determining the cache root:

1. `HfApiBuilder::cache_dir(path)` — explicit programmatic value
2. `HF_HUB_CACHE` environment variable
3. `$HF_HOME/hub` (where `HF_HOME` defaults to `~/.cache/huggingface`)

The resolved path is stored in `HfApiInner::cache_dir: PathBuf`.

## Cache Layout

```
{cache_root}/
├── .locks/{repo_folder}/{etag}.lock
└── {repo_folder}/
    ├── blobs/{etag}
    ├── refs/{revision_name}
    ├── snapshots/{commit_hash}/{relative_path} -> ../../blobs/{etag}
    └── .no_exist/{commit_hash}/{relative_path}
```

### Repository Folder Naming

`{type}s--{repo_id with / replaced by --}`

| repo_id | repo_type | folder |
|---------|-----------|--------|
| `google/bert` | model | `models--google--bert` |
| `squad` | dataset | `datasets--squad` |
| `user/app` | space | `spaces--user--app` |

### Blobs

Content-addressed storage. Filenames are the file's etag from the Hub:
- Git-tracked files: SHA-1 (40 hex chars)
- Git LFS files: SHA-256 (64 hex chars)

Flat directory, no subdirectories. Identical content across revisions is stored once.

### Refs

Plain text files mapping branch/tag names to commit hashes. Each file contains a single line: the 40-character commit hash.

Nested refs (e.g., PR refs) use subdirectories: `refs/refs/pr/1`.

### Snapshots

One subdirectory per commit hash. Files are relative symlinks pointing to `../../blobs/{etag}`. Subdirectories within the snapshot mirror the repo's file structure.

On Windows where symlinks are unavailable, files are copied directly into snapshots instead of symlinked. The `blobs/` directory is not used in this mode.

### .no_exist

Empty marker files indicating a file was requested but does not exist on the Hub at a given revision. Structure mirrors snapshots: `{commit_hash}/{relative_path}`.

### Lock Files

Stored at cache root level: `{cache_root}/.locks/{repo_folder}/{etag}.lock`. Prevents concurrent processes from downloading the same blob.

## Changes to Existing Types

### Breaking Change: `DownloadFileParams`

`local_dir` changes from a required `PathBuf` to `Option<PathBuf>`. This is a breaking API change — all existing callers must wrap their `local_dir` value in `Some(...)`. Internal callers that access `params.local_dir` directly (including `xet.rs`) must be updated to handle the `Option`.

```rust
pub struct DownloadFileParams {
    pub repo_id: String,
    pub filename: String,
    pub local_dir: Option<PathBuf>,       // was: required PathBuf
    pub repo_type: Option<RepoType>,
    pub revision: Option<String>,
    pub force_download: Option<bool>,      // new
    pub local_files_only: Option<bool>,    // new
}
```

When `local_dir` is `None`, the cache is used. When `Some`, files download directly to that directory (no cache involvement).

### HfApiBuilder

New method:
```rust
pub fn cache_dir(mut self, path: impl Into<PathBuf>) -> Self
```

### HfApiInner

New field:
```rust
pub(crate) cache_dir: PathBuf
```

Resolved during `build()` using the priority order above.

### HfError

New variants:
```rust
LocalEntryNotFound { path: String }    // local_files_only mode, file not cached
CacheLockTimeout { path: PathBuf }     // file lock acquisition timed out
```

## Download Flow (Cache Mode)

When `local_dir` is `None`:

1. Compute repo folder name and cache paths
2. If revision is a 40-char hex commit hash, check `snapshots/{hash}/{filename}` — if exists and `force_download` is not set, return path immediately
3. If `local_files_only` is set, also try resolving via `refs/{revision}` → commit hash → snapshot check, then return `LocalEntryNotFound` on miss
4. Send GET request to `{endpoint}/{prefix}{repo_id}/resolve/{revision}/{filename}`. Extract metadata from **response headers**: `ETag` (or `X-Linked-Etag` for LFS, which takes priority), `X-Repo-Commit` for commit hash. Strip quotes from etag values. If the blob already exists in cache (and `force_download` is not set), discard the response body early.
5. Write commit hash to `refs/{revision}` if revision is not already a commit hash
6. Check if `blobs/{etag}` exists — if so and `force_download` is not set, skip to step 9
7. Acquire file lock at `{cache_root}/.locks/{repo_folder}/{etag}.lock` using `fs4`. Lock timeout: 10 seconds (matching Python). `fs4` uses advisory locks (`flock`/`LockFileEx`) which auto-release on process exit, handling crash recovery.
8. Stream the response body (from step 4) to `blobs/{etag}.incomplete`, then atomic rename to `blobs/{etag}`. On Unix, `rename(2)` is atomic within the same filesystem. The `.incomplete` and final blob are always in the same directory, guaranteeing same-filesystem.
9. Create relative symlink: `snapshots/{commit_hash}/{filename}` -> `../../blobs/{etag}`. This happens inside the lock scope to prevent races with concurrent processes creating the same snapshot directory.
10. Release lock, return the symlink path

This uses a single HTTP request (GET) instead of HEAD+GET. The response headers provide the etag and commit hash needed for cache placement. If the blob already exists (step 6), the response body is discarded without reading it fully.

When `local_dir` is `Some(path)`: download directly to `path/filename` (current behavior, no cache).

### 404 Handling

When the GET request returns 404, create `.no_exist/{commit_hash}/{filename}` marker (if commit hash is known from the response) and return `EntryNotFound`.

### Xet Integration

The existing xet download path (feature-gated behind `xet`) requires a HEAD request to detect the `X-Xet-Hash` header. In cache mode with xet enabled, the flow becomes: send HEAD first, check for `X-Xet-Hash`. If present, use the xet protocol to download directly to `blobs/{etag}.incomplete` (the xet download function must be updated to accept a target blob path instead of `local_dir`), then atomic rename and symlink as normal. If no xet header, proceed with the standard GET flow above. The cache structure is the same regardless of transfer protocol.

## Snapshot Download

New method `snapshot_download` on `HfApi`.

### SnapshotDownloadParams

```rust
pub struct SnapshotDownloadParams {
    pub repo_id: String,
    pub repo_type: Option<RepoType>,
    pub revision: Option<String>,
    pub allow_patterns: Option<Vec<String>>,
    pub ignore_patterns: Option<Vec<String>>,
    pub local_dir: Option<PathBuf>,
    pub force_download: Option<bool>,
    pub local_files_only: Option<bool>,
    pub max_workers: Option<usize>,        // default: 8
}
```

### Flow

1. Resolve the revision to a commit hash upfront. If revision is already a 40-char hex string, use it directly. Otherwise, make a single API call (e.g., `repo_info` or a HEAD request to any file) to resolve the branch/tag to a commit hash. This pinned commit hash is used for all subsequent downloads, ensuring all files come from the same revision even if the branch moves during the download.
2. List all files via `list_repo_tree` at the pinned commit hash (recursive)
3. Filter by `allow_patterns` / `ignore_patterns` using existing `matches_any_glob`
4. Download concurrently with `futures::stream::buffer_unordered(max_workers)`, passing the pinned commit hash as the revision to each `download_file` call
5. Return snapshot directory: `{cache_root}/{repo_folder}/snapshots/{commit_hash}/`

In `local_dir` mode, return the `local_dir` path.

## Cache Management

### Types

```rust
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

### Methods

`scan_cache(&self) -> Result<HfCacheInfo>`: Walk the cache directory. For each repo folder, enumerate revisions from `snapshots/`, follow symlinks to determine blob sizes, read `refs/` to map branch/tag names. Collect warnings for malformed entries.

`delete_cache_revisions(&self, revisions: &[DeleteCacheRevision]) -> Result<()>`: Remove specified snapshot directories, update refs, delete orphaned blobs (blobs not referenced by any remaining snapshot symlink). Non-fatal errors (revision not found in cache, individual file deletion failures) are logged as warnings and skipped — the operation continues with remaining items rather than failing entirely.

## New Module: `src/cache.rs`

Internal helpers (all `pub(crate)`):

- `repo_folder_name(repo_id: &str, repo_type: Option<RepoType>) -> String`
- `blob_path(cache_dir: &Path, repo_folder: &str, etag: &str) -> PathBuf`
- `snapshot_path(cache_dir: &Path, repo_folder: &str, commit_hash: &str, filename: &str) -> PathBuf`
- `ref_path(cache_dir: &Path, repo_folder: &str, revision: &str) -> PathBuf`
- `lock_path(cache_dir: &Path, repo_folder: &str, etag: &str) -> PathBuf`
- `no_exist_path(cache_dir: &Path, repo_folder: &str, commit_hash: &str, filename: &str) -> PathBuf`
- Lock acquisition/release using `fs4`
- Cache scan and deletion logic (called by `HfApi` methods)

## Dependencies

- `fs4` with tokio feature — cross-process file locking with async support (Windows, Linux, macOS)

No new feature flags. Caching is always available.

## Testing

### Unit Tests (in `src/cache.rs`)

- Path computation functions produce correct output matching Python cache layout
- Ref file read/write roundtrip
- Cache scan on a manually constructed tempdir cache
- Deletion: orphaned blobs removed, shared blobs preserved

### Integration Tests (in `tests/integration_test.rs`)

- `download_file` without `local_dir` creates correct cache structure
- Cache hit: second download returns immediately without HTTP
- Two revisions sharing a blob: one blob file, two symlinks
- `force_download` re-downloads even when cached
- `local_files_only` returns cached file or `LocalEntryNotFound`
- `.no_exist` marker on 404
- `snapshot_download` with pattern filtering
- `scan_cache` returns correct info after downloads
- `delete_cache_revisions` cleans up correctly

### Platform Guards

- Symlink-specific assertions use `#[cfg(not(windows))]`
- Windows tests verify file copy fallback behavior
