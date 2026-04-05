# Progress Tracking for Download & Upload

**Date:** 2026-04-05
**Status:** Draft

## Summary

Add progress tracking to the huggingface-hub Rust library's download and upload interfaces via a polling-based `ProgressHandle`. The handle is returned as part of a `TransferTask<T>` wrapper that implements `IntoFuture`, preserving backward compatibility with existing `.await` call sites. The first consumer is the hfrs CLI, which renders indicatif progress bars matching the Python `hf` CLI style.

## Goals

1. Per-file and aggregate byte-level progress for downloads and uploads
2. Phase tracking for uploads (preparing → transferring → finalizing)
3. Zero-cost for callers who don't use progress (just `.await` as before)
4. Leverage hf-xet's existing `GroupProgressReport` / `ItemProgressReport` directly — no intermediate polling tasks
5. hfrs CLI progress bars visually match the Python `hf` CLI

## Non-Goals

- Push-based callbacks or channel-based event streams
- Progress tracking for non-transfer operations (repo creation, listing, etc.)
- Backward-compatible `async fn` signatures (the change to `fn -> TransferTask` is a type-level breaking change, but source-compatible for `.await` callers)

---

## Architecture

### Core Types

New file: `huggingface_hub/src/types/progress.rs`

```rust
/// Snapshot of transfer progress at a point in time.
#[derive(Debug, Clone)]
pub struct TransferProgress {
    /// Current phase of the operation.
    pub phase: TransferPhase,
    /// Total logical bytes across all files.
    pub total_bytes: u64,
    /// Logical bytes completed so far.
    pub bytes_completed: u64,
    /// Logical bytes/sec throughput. None until enough samples collected.
    pub bytes_per_sec: Option<f64>,
    /// Total network/transfer bytes (may differ from logical bytes due to
    /// compression and deduplication in xet transfers).
    pub total_transfer_bytes: u64,
    /// Transfer bytes completed.
    pub transfer_bytes_completed: u64,
    /// Transfer bytes/sec throughput.
    pub transfer_bytes_per_sec: Option<f64>,
    /// Per-file progress snapshots (only files currently in-flight or completed).
    pub files: Vec<FileProgress>,
    /// Number of files that have fully completed.
    pub files_completed: usize,
    /// Total number of files in the operation.
    pub total_files: usize,
}

/// Progress for a single file.
#[derive(Debug, Clone)]
pub struct FileProgress {
    pub filename: String,
    pub total_bytes: u64,
    pub bytes_completed: u64,
}

/// Phase of a transfer operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferPhase {
    /// Preparing: file collection, hashing, preupload classification.
    Preparing,
    /// Actively transferring data (xet or HTTP).
    Transferring,
    /// Post-transfer: creating commit, renaming files.
    Finalizing,
    /// Operation complete.
    Complete,
}
```

**Design decisions:**

- `TransferProgress` mirrors xet's `GroupProgressReport` closely (total_bytes / transfer_bytes split, rates) so the xet bridge is near-zero cost.
- `files: Vec<FileProgress>` is allocated on each `.progress()` call — for CLI polling at 100ms this is fine.
- `TransferPhase` is a simple enum. Short phases may be missed between polls, but the caller sees the current state and the progression.

### ProgressHandle

```rust
/// A cloneable, thread-safe handle for polling transfer progress.
///
/// Internally backed by Arc — cloning is cheap.
/// All reads are lock-free for xet transfers (atomic reads on the xet objects).
#[derive(Clone)]
pub struct ProgressHandle {
    inner: Arc<ProgressState>,
}

impl ProgressHandle {
    /// Create a new progress handle. Called internally by the library.
    pub(crate) fn new() -> Self { ... }

    /// Snapshot current progress. Cheap and safe from any thread.
    pub fn progress(&self) -> TransferProgress { ... }
}
```

#### Internal State

```rust
struct ProgressState {
    phase: AtomicU8,

    // Lazily set by the library when the xet session is built.
    // OnceLock because it's set once and read many times.
    xet_source: OnceLock<XetProgressSource>,

    // For non-xet HTTP transfers. Updated directly by stream wrappers.
    http_bytes_completed: AtomicU64,
    http_total_bytes: AtomicU64,

    // Per-file tracking.
    files: RwLock<Vec<FileProgress>>,
    total_files: AtomicUsize,
    files_completed: AtomicUsize,
}

enum XetProgressSource {
    DownloadGroup(XetFileDownloadGroup),  // Arc-backed, clone is cheap
    UploadCommit(XetUploadCommit),        // Arc-backed, clone is cheap
}
```

When `ProgressHandle::progress()` is called:

1. If `xet_source` is set: call `.progress()` on the xet object (lock-free atomic reads), map `GroupProgressReport` fields into the aggregate `TransferProgress` fields.
2. Add `http_*` atomic counters (for non-xet files in mixed snapshot downloads).
3. Build the `files` vec by merging two sources:
   - For xet transfers: read `ItemProgressReport` from xet task handles (stored alongside the xet source).
   - For non-xet transfers: read the `files` RwLock (updated by HTTP stream workers).
4. Read phase from atomic.

**No background polling tasks.** The xet objects are polled lazily on demand only when the caller reads `.progress()`.

### TransferTask

```rust
/// A transfer operation that can be awaited for its result or polled for progress.
///
/// Implements `IntoFuture` so existing `.await` call sites work unchanged.
/// Clone the progress handle before awaiting, since `.await` consumes self.
pub struct TransferTask<T> {
    future: Pin<Box<dyn Future<Output = Result<T>> + Send>>,
    progress: ProgressHandle,
}

impl<T> TransferTask<T> {
    /// Get a reference to the progress handle. Clone it to poll from another task.
    pub fn progress(&self) -> &ProgressHandle {
        &self.progress
    }
}

impl<T> IntoFuture for TransferTask<T> {
    type Output = Result<T>;
    type IntoFuture = Pin<Box<dyn Future<Output = Result<T>> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        self.future
    }
}
```

---

## API Changes

The following methods change from `async fn` to `fn -> TransferTask<T>`. The `.await` call pattern is source-compatible — existing code like `repo.download_file(&params).await?` compiles without changes because `.await` uses `IntoFuture`.

| Method | Before | After |
|--------|--------|-------|
| `HFRepository::download_file` | `async fn(...) -> Result<PathBuf>` | `fn(...) -> TransferTask<PathBuf>` |
| `HFRepository::snapshot_download` | `async fn(...) -> Result<PathBuf>` | `fn(...) -> TransferTask<PathBuf>` |
| `HFRepository::upload_file` | `async fn(...) -> Result<CommitInfo>` | `fn(...) -> TransferTask<CommitInfo>` |
| `HFRepository::upload_folder` | `async fn(...) -> Result<CommitInfo>` | `fn(...) -> TransferTask<CommitInfo>` |
| `HFRepository::create_commit` | `async fn(...) -> Result<CommitInfo>` | `fn(...) -> TransferTask<CommitInfo>` |

Equivalent `HFClient` methods follow the same change.

### Method Internals

```rust
pub fn download_file(&self, params: &RepoDownloadFileParams) -> TransferTask<PathBuf> {
    let progress = ProgressHandle::new();
    let repo = self.clone();          // cheap Arc clone
    let params = params.clone();      // small struct
    let ph = progress.clone();
    let future = Box::pin(async move {
        repo.download_file_inner(&params, &ph).await
    });
    TransferTask { future, progress }
}
```

The actual logic moves to private `*_inner` methods that accept a `&ProgressHandle`. Params structs gain `Clone` derives where not already present.

---

## Upload Phase Mapping

The upload flow in `create_commit_inner` maps to phases:

| Phase | What happens |
|-------|-------------|
| **Preparing** | Collect files, compute sizes, call preupload endpoint to classify files as LFS vs regular |
| **Transferring** | Xet upload for LFS files. For regular files, base64 encoding (fast, brief) |
| **Finalizing** | POST to `/api/.../commit` endpoint |
| **Complete** | Commit succeeded |

The `ProgressHandle` transitions through these phases via its internal atomic.

### Xet Upload Progress

During the **Transferring** phase for xet uploads:

- The `XetUploadCommit` object is stored in `xet_source` via `OnceLock`.
- `ProgressHandle::progress()` reads `commit.progress()` which returns `GroupProgressReport` with:
  - `total_bytes` / `total_bytes_completed` — logical data processed
  - `total_transfer_bytes` / `total_transfer_bytes_completed` — network bytes (new data after dedup)
  - `total_bytes_completion_rate` / `total_transfer_bytes_completion_rate` — throughput
- Per-file progress comes from xet's `ItemProgressReport` (via upload task handles stored in the `files` RwLock).

This provides the two aggregate metrics needed to display "Processing Files" and "New Data Upload" bars matching the Python CLI.

---

## Download Progress

### Single File (non-xet HTTP)

Progress tracked by incrementing `http_*` atomics as chunks arrive from the byte stream:

```rust
// In download_file_inner's HTTP streaming path
while let Some(chunk) = stream.next().await {
    let chunk = chunk?;
    file.write_all(&chunk).await?;
    ph.report_http_bytes(chunk.len() as u64);
}
```

Phase transitions: `Preparing` (HEAD request, cache check) → `Transferring` (streaming) → `Complete`.

### Single File (xet)

The `XetFileDownloadGroup` is stored in `xet_source` via `OnceLock`. Progress is read lazily from the group on each `.progress()` call — no polling task needed.

### Snapshot Download

`snapshot_download` involves parallel workers processing a mix of xet and non-xet files:

- **Xet files**: batched into a single `XetFileDownloadGroup`. Its `GroupProgressReport` feeds the aggregate xet bytes via lazy polling.
- **Non-xet files**: each parallel worker increments the shared `http_*` atomics.
- **Per-file tracking**: each worker adds/updates entries in the `files` RwLock as files start and complete.
- **File count**: `total_files` set after file listing, `files_completed` incremented atomically as each file finishes.

Phase transitions: `Preparing` (API call to list files, cache checks) → `Transferring` (parallel downloads) → `Complete`.

---

## hfrs CLI Progress Bars

### New Dependency

```toml
indicatif = { version = "0.17", optional = true }

[features]
cli = [
    # ... existing deps ...
    "dep:indicatif",
]
```

### Visual Targets (matching Python `hf` CLI)

**Single file download:**
```
model.safetensors:  45%|████▌     | 1.23 GB/2.71 GB [38.5 MB/s]
```

**Snapshot (multi-file) download:**
```
Fetching 12 files:  75%|███████▌  | 9/12
Downloading:        45%|████▌     | 1.23 GB/2.71 GB [38.5 MB/s]
```

Two bars matching Python's `snapshot_download`: file count + aggregate bytes.

**Upload (xet):**
```
Processing Files (3 / 10)  |████▌     | 1.23 GB/3.45 GB   38.5 MB/s
New Data Upload            |██▌       |  500 MB/2.10 GB   25.0 MB/s
  model-00001.safetensors  |████████  | 1.00 GB/1.20 GB
  model-00002.safetensors  |███▌      |  230 MB/1.20 GB
  [+ 3 files]              |██        |  780 MB/3.60 GB
```

Matches Python's `XetProgressReporter`: two summary bars (processing + transfer) plus a scrolling window of up to 10 per-file bars. When more files are active, the last bar aggregates overflow into `[+ N files]`.

**Upload (non-xet / small regular files):**
Simple spinner during the brief base64-encode-and-POST, since there's no meaningful byte-level tracking for inline uploads.

### New File: `src/bin/hfrs/progress.rs`

```rust
pub struct DownloadProgressDisplay { ... }
pub struct UploadProgressDisplay { ... }

/// Spawn a tokio task that polls the ProgressHandle and updates indicatif bars.
pub fn spawn_progress_poll(
    progress: ProgressHandle,
    display: impl ProgressDisplay + Send + 'static,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let p = progress.progress();
            display.update(&p);
            if p.phase == TransferPhase::Complete {
                display.finish(&p);
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
}
```

The `ProgressDisplay` trait abstracts indicatif rendering. Each display type owns an `indicatif::MultiProgress` with the appropriate bar layout.

### indicatif Style Templates

```rust
// Byte-level bar (matching tqdm unit="B", unit_scale=True style)
let bytes_style = ProgressStyle::with_template(
    "{msg}: {percent}%|{wide_bar}| {bytes}/{total_bytes} [{bytes_per_sec}]"
);

// File-count bar
let files_style = ProgressStyle::with_template(
    "{msg}: {percent}%|{wide_bar}| {pos}/{len}"
);

// Upload processing bar (matching Python XetProgressReporter)
let processing_style = ProgressStyle::with_template(
    "{msg}  |{wide_bar}| {bytes}/{total_bytes} {bytes_per_sec}"
);
```

### CLI Command Wiring

```rust
// In commands/download.rs
let task = repo.download_file(&params);
let display_handle = if args.quiet {
    None
} else {
    Some(spawn_progress_poll(
        task.progress().clone(),
        SingleFileDownloadDisplay::new(),
    ))
};
let path = task.await?;
if let Some(h) = display_handle { h.abort(); }
```

The `--quiet` flag and `HF_HUB_DISABLE_PROGRESS_BARS` env var both suppress the display.

---

## Blocking API

The `HFClientSync` / `HFRepoSync` blocking wrappers return `TransferTask<T>` as well. `TransferTask` gains a blocking method:

```rust
impl<T> TransferTask<T> {
    /// Block until the transfer completes. For use with the blocking API.
    pub fn block(self) -> Result<T> {
        // Uses the existing blocking runtime
        runtime.block_on(self.future)
    }
}
```

Callers can poll `.progress()` from a separate thread before calling `.block()`.

---

## Testing Strategy

### Unit Tests (`types/progress.rs`)

- `ProgressHandle` can be created, cloned, polled from multiple threads
- Phase transitions reflected in `.progress()` snapshots
- HTTP byte counters increment correctly under concurrent updates
- Per-file entries correctly added, updated, and reported
- `TransferTask` implements `IntoFuture` correctly — `.await` returns the inner result
- Progress handle always created even when nobody polls (no overhead beyond Arc allocation)

### Unit Tests (`bin/hfrs/progress.rs`)

- Download display formatting produces expected bar templates
- File count bar reflects `files_completed` / `total_files`
- Upload display shows both processing and transfer bars
- Per-file scrolling window caps at 10 and aggregates overflow into `[+ N files]`
- Quiet mode produces no output

### Integration Tests (`tests/integration_test.rs`)

Gated on `HF_TOKEN` (and `HF_TEST_WRITE=1` for uploads):

- **Download single file with progress**: verify `bytes_completed` reaches `total_bytes`, phase ends at `Complete`
- **Snapshot download with progress**: verify `total_files` matches expected count, `files_completed` reaches `total_files`
- **Upload file with progress** (`HF_TEST_WRITE=1`): verify phase transitions `Preparing → Transferring → Finalizing → Complete`
- **Upload folder with progress** (`HF_TEST_WRITE=1`): same phase transitions, `total_files` matches folder contents
- **No-progress path**: verify plain `.await` without polling still completes correctly (no regressions)

### CLI Manual Verification

1. `hfrs download gpt2 config.json` — one bytes bar appears and completes
2. `hfrs download gpt2` — files count bar + aggregate bytes bar
3. `hfrs download gpt2 --quiet` — no bars, only path printed
4. `hfrs upload <test-repo> ./small-file.txt` — phase progression visible
5. `hfrs upload <test-repo> ./test-dir/` — files bar + bytes bar
6. `HF_HUB_DISABLE_PROGRESS_BARS=1 hfrs download gpt2` — bars suppressed
7. Large xet-backed repo — aggregate bytes bar with rate display, per-file bars scroll

---

## Files Changed

### New Files

| File | Purpose |
|------|---------|
| `huggingface_hub/src/types/progress.rs` | `TransferProgress`, `FileProgress`, `TransferPhase`, `ProgressHandle`, `TransferTask` |
| `huggingface_hub/src/bin/hfrs/progress.rs` | `DownloadProgressDisplay`, `UploadProgressDisplay`, `spawn_progress_poll` |

### Modified Files

| File | Changes |
|------|---------|
| `huggingface_hub/src/types/mod.rs` | Add `pub mod progress` and re-exports |
| `huggingface_hub/src/lib.rs` | Re-export progress types: `TransferProgress`, `FileProgress`, `TransferPhase`, `ProgressHandle`, `TransferTask` |
| `huggingface_hub/src/api/files.rs` | Change download/upload methods from `async fn` to `fn -> TransferTask`, move logic to `*_inner` methods, add HTTP stream progress reporting |
| `huggingface_hub/src/api/commits.rs` | Change `create_commit` to `fn -> TransferTask`, move logic to `create_commit_inner` |
| `huggingface_hub/src/xet.rs` | Accept `&ProgressHandle`, store xet objects in `OnceLock`, update per-file state |
| `huggingface_hub/src/repository.rs` | Add `Clone` to params structs, update method signatures to return `TransferTask` |
| `huggingface_hub/src/bin/hfrs/commands/download.rs` | Clone progress handle, spawn display, await task |
| `huggingface_hub/src/bin/hfrs/commands/upload.rs` | Clone progress handle, spawn display, await task |
| `huggingface_hub/src/bin/hfrs/main.rs` | Add `mod progress` |
| `huggingface_hub/Cargo.toml` | Add `indicatif` optional dependency under `cli` feature |
| `huggingface_hub/tests/integration_test.rs` | Add progress-tracking integration tests |
