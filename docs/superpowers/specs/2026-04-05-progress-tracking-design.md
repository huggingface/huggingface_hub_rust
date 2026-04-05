# Progress Tracking for Upload and Download

**Date:** 2026-04-05
**Status:** Draft

## Overview

Add callback-based progress tracking to the huggingface-hub library's upload and download interfaces. The library emits structured `ProgressEvent` variants to a caller-provided `ProgressHandler` trait object. The first consumer is the hfrs CLI, which renders indicatif progress bars matching the Python `hf` CLI's tqdm style.

## Goals

- Per-file byte-level progress for downloads
- Aggregate byte-level progress for uploads (xet provides aggregate, not per-file)
- Distinguish upload phases: preparing, checking upload mode, uploading, committing
- Utilize hf-xet's `GroupProgressReport` and `ItemProgressReport` via polling bridge
- hfrs CLI progress bars visually match the Python `hf` CLI
- Fully additive — no breaking changes to existing API

## Non-Goals

- Per-file byte-level upload progress (xet `GroupProgressReport` only provides aggregate)
- Progress for non-transfer operations (repo creation, deletion, listing)
- Persistent progress state or resume tracking
- Custom bar format configuration in the library (that's the consumer's job)

---

## Core Types

New file: `huggingface_hub/src/types/progress.rs`

```rust
use std::sync::Arc;

/// Trait implemented by consumers to receive progress updates.
/// Implementations must be fast — avoid blocking I/O in on_progress().
pub trait ProgressHandler: Send + Sync {
    fn on_progress(&self, event: &ProgressEvent);
}

/// A clonable, optional handle to a progress handler.
pub type Progress = Option<Arc<dyn ProgressHandler>>;

/// All progress events emitted by upload and download operations.
#[derive(Debug, Clone)]
pub enum ProgressEvent {
    // === Upload events ===
    /// Upload operation has started; total file count and bytes are known.
    UploadStart { total_files: usize, total_bytes: u64 },
    /// A new upload phase has begun.
    UploadPhaseChange { phase: UploadPhase },
    /// Bytes progress for xet/LFS upload (aggregated across all files).
    UploadProgress { bytes_completed: u64, total_bytes: u64, bytes_per_sec: Option<f64> },
    /// Upload operation finished.
    UploadComplete,

    // === Download events ===
    /// Download operation has started; file count and total bytes known.
    DownloadStart { total_files: usize, total_bytes: u64 },
    /// A single file download has begun.
    DownloadFileStart { filename: String, total_bytes: u64 },
    /// Byte-level progress for a single file.
    DownloadFileProgress { filename: String, bytes_completed: u64, total_bytes: u64 },
    /// A single file download completed.
    DownloadFileComplete { filename: String },
    /// Aggregate download progress (xet batch transfers).
    DownloadProgress { bytes_completed: u64, total_bytes: u64, bytes_per_sec: Option<f64> },
    /// All downloads finished.
    DownloadComplete,
}

/// Phases of an upload operation, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadPhase {
    /// Scanning local files and computing sizes.
    Preparing,
    /// Calling preupload API to classify files as LFS vs regular.
    CheckingUploadMode,
    /// Transferring file data (xet or inline).
    Uploading,
    /// Creating the commit on the Hub.
    Committing,
}

/// Emit a progress event if a handler is present.
pub(crate) fn emit(handler: &Progress, event: ProgressEvent) {
    if let Some(h) = handler {
        h.on_progress(&event);
    }
}
```

### Design decisions

- **Flat enum** for `ProgressEvent` rather than nested structs — simple to match, easy to extend.
- **`Progress` type alias** (`Option<Arc<dyn ProgressHandler>>`) keeps params structs clean.
- **Upload progress is aggregate** because xet's `GroupProgressReport` doesn't provide per-file byte progress. Per-file upload tracking would require wrapping file reads, which conflicts with xet's internal batching.
- **Download progress is per-file** because we control byte streams (non-xet) and xet provides `ItemProgressReport`.
- **`bytes_per_sec` is `Option<f64>`** because xet requires >=4 observations before reporting rates.
- **`emit()` helper** avoids `if let Some(h) = &progress { ... }` at every call site.

---

## Library Integration Points

### Params struct changes

Add `progress: Progress` with `#[builder(default)]` to these existing structs in `repository.rs`:

- `RepoUploadFileParams`
- `RepoUploadFolderParams`
- `RepoCreateCommitParams`
- `RepoDownloadFileParams`
- `RepoSnapshotDownloadParams`

Existing callers are unaffected — `progress` defaults to `None`.

### Upload flow (`api/files.rs` — `create_commit`)

Event emission points in the existing flow:

```
1. Collect operations, compute total sizes
   → emit UploadStart { total_files, total_bytes }
   → emit UploadPhaseChange { Preparing }

2. Call preupload_and_upload_lfs_files() to classify files
   → emit UploadPhaseChange { CheckingUploadMode }

3. Upload LFS files via xet_upload()
   → emit UploadPhaseChange { Uploading }
   → spawn xet polling task (100ms interval)
     → emit UploadProgress { bytes_completed, total_bytes, bytes_per_sec }

4. POST /api/.../commit
   → emit UploadPhaseChange { Committing }

5. Return CommitInfo
   → emit UploadComplete
```

### Xet polling bridge (`xet.rs`)

Internal helper that polls xet progress and forwards to the handler:

```rust
fn spawn_xet_upload_poller(
    commit: XetUploadCommit,
    handler: Arc<dyn ProgressHandler>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let report = commit.progress();
            handler.on_progress(&ProgressEvent::UploadProgress {
                bytes_completed: report.total_bytes_completed,
                total_bytes: report.total_bytes,
                bytes_per_sec: report.total_bytes_completion_rate,
            });
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
}
```

A similar `spawn_xet_download_poller` emits `DownloadProgress` events from `XetFileDownloadGroup::progress()`.

The polling task is aborted when the transfer completes (the caller holds the `JoinHandle` and calls `.abort()` after `.commit().await` / `.finish().await` returns).

### Download flow — single file (`api/files.rs`)

**Non-xet path:**

```
1. HEAD request → get ETag, content-length
   → emit DownloadStart { total_files: 1, total_bytes }
   → emit DownloadFileStart { filename, total_bytes }

2. GET request → wrap response byte stream in ProgressStream adapter
   → emit DownloadFileProgress on each chunk

3. File written
   → emit DownloadFileComplete { filename }
   → emit DownloadComplete
```

**Xet path:**

```
1. HEAD request → detect X-Xet-Hash
   → emit DownloadStart / DownloadFileStart

2. xet_download_to_local_dir / xet_download_to_blob
   → spawn xet download poller
   → emit DownloadProgress from GroupProgressReport

3. Complete
   → emit DownloadFileComplete / DownloadComplete
```

### Download flow — snapshot (`api/files.rs`)

```
1. List files via get_paths_info / list_tree
   → emit DownloadStart { total_files, total_bytes }

2. Non-xet files: parallel workers (buffer_unordered, max_workers)
   Each worker emits DownloadFileStart / DownloadFileProgress / DownloadFileComplete
   (handler is Arc, shared safely across tasks)

3. Xet files: xet_download_batch
   → spawn xet download poller → emit DownloadProgress
   → emit DownloadFileComplete for each file as xet finishes it

4. All done → emit DownloadComplete
```

### ProgressStream adapter

A thin stream wrapper for non-xet HTTP downloads that emits per-chunk progress. Uses `pin-project-lite` for pin projection (already a transitive dependency via tokio).

```rust
pin_project_lite::pin_project! {
    struct ProgressStream<S> {
        #[pin]
        inner: S,
        handler: Arc<dyn ProgressHandler>,
        filename: String,
        total_bytes: u64,
        bytes_read: u64,
    }
}

impl<S: Stream<Item = Result<Bytes>>> Stream for ProgressStream<S> {
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        match this.inner.poll_next(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                *this.bytes_read += chunk.len() as u64;
                this.handler.on_progress(&ProgressEvent::DownloadFileProgress {
                    filename: this.filename.clone(),
                    bytes_completed: *this.bytes_read,
                    total_bytes: *this.total_bytes,
                });
                Poll::Ready(Some(Ok(chunk)))
            }
            other => other,
        }
    }
}
```

### Xet function signature changes

All internal xet functions accept an additional `progress: &Progress` parameter:

- `xet_upload(api, files, repo_id, repo_type, revision, progress)`
- `xet_download_batch(api, repo_id, repo_type, revision, files, progress)`
- `xet_download_to_local_dir(api, repo_id, ..., head_response, progress)`
- `xet_download_to_blob(api, repo_id, ..., file_hash, file_size, path, progress)`

When `progress` is `None`, no polling task is spawned and the functions behave identically to current code.

---

## hfrs CLI Progress Bars

### Dependency

Add to `Cargo.toml`:

```toml
indicatif = { version = "0.17", optional = true }

[features]
cli = [
    # ... existing deps ...
    "dep:indicatif",
]
```

### Visual targets (matching Python `hf` CLI)

The Python CLI uses standard tqdm formatting. The indicatif bars match this style:

**Single file download:**
```
config.json: 100%|██████████████████████████████████| 665/665 [00:00<00:00, 764kB/s]
```

**Multi-file download (two simultaneous bars):**
```
Fetching 23 files:  65%|████████████████           | 15/23 [01:02<00:33, 4.12s/it]
model.safetensors:  45%|████████                   | 2.22G/4.93G [01:02<01:16, 36.4MB/s]
```

**Multi-file upload (two bars):**
```
Upload 5 LFS files:  40%|████████                  | 2/5 [00:12<00:18, 6.1s/it]
model.safetensors: 100%|███████████████████████████| 4.93G/4.93G [02:15<00:00, 36.4MB/s]
```

**Formatting rules:**
- Filenames truncated at 40 chars with `(…)` prefix
- Byte bars use auto-scaled units (kB, MB, GB)
- File-count bars show items/sec
- All bars show `[elapsed<remaining, rate]`

### CliProgressHandler

New file: `src/bin/hfrs/progress.rs`

```rust
use std::collections::HashMap;
use std::sync::Mutex;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use huggingface_hub::{ProgressEvent, ProgressHandler};

pub struct CliProgressHandler {
    multi: MultiProgress,
    state: Mutex<ProgressState>,
}

struct ProgressState {
    /// Overall files bar (for multi-file operations)
    files_bar: Option<ProgressBar>,
    /// Aggregate bytes bar (for xet transfers or overall bytes tracking)
    bytes_bar: Option<ProgressBar>,
    /// Per-file bars for individual file byte progress
    file_bars: HashMap<String, ProgressBar>,
}
```

### Event-to-bar mapping

| Event | Bar action |
|---|---|
| `DownloadStart { total_files > 1, .. }` | Create files bar: `"Fetching {n} files"` |
| `DownloadStart { total_files == 1, .. }` | No files bar |
| `DownloadFileStart { filename, total_bytes }` | Create per-file bytes bar |
| `DownloadFileProgress { filename, .. }` | Update per-file bar position |
| `DownloadFileComplete { filename }` | Finish + remove per-file bar, increment files bar |
| `DownloadProgress { .. }` | Update aggregate bytes bar (xet batch) |
| `DownloadComplete` | Finish all bars |
| `UploadStart { total_files, .. }` | Create files bar: `"Upload {n} LFS files"` if >1 |
| `UploadPhaseChange { Uploading }` | Create aggregate bytes bar |
| `UploadProgress { .. }` | Update aggregate bytes bar |
| `UploadPhaseChange { Committing }` | Finish bytes bar, show spinner: `"Creating commit..."` |
| `UploadComplete` | Finish all bars |

### indicatif style templates

```rust
// Byte-level bar (matching tqdm default with unit="B", unit_scale=True)
let bytes_style = ProgressStyle::with_template(
    "{msg}: {percent}%|{wide_bar}| {bytes}/{total_bytes} [{elapsed}<{eta}, {bytes_per_sec}]"
);

// File-count bar (matching tqdm default with items)
let files_style = ProgressStyle::with_template(
    "{msg}: {percent}%|{wide_bar}| {pos}/{len} [{elapsed}<{eta}, {per_sec}]"
);

// Spinner for phases without byte progress
let spinner_style = ProgressStyle::with_template("{spinner} {msg}");
```

### CLI wiring

In `commands/download.rs` and `commands/upload.rs`:

```rust
let progress: Progress = if args.quiet || env_var_disables_progress() {
    None
} else {
    Some(Arc::new(CliProgressHandler::new()))
};
```

The `env_var_disables_progress()` check reads `HF_HUB_DISABLE_PROGRESS_BARS` to match Python behavior.

### Multi-file download bar behavior

For `snapshot_download` with mixed xet and non-xet files:

- **Non-xet files**: each gets its own per-file bytes bar (created on `DownloadFileStart`, updated on `DownloadFileProgress`, removed on `DownloadFileComplete`). Up to `max_workers` (default 8) per-file bars visible simultaneously.
- **Xet batch files**: a single aggregate bytes bar from `DownloadProgress` events. Individual completions still fire `DownloadFileComplete` to increment the files counter.
- **Files count bar** stays at the top, counting completions from both paths.

---

## Testing Strategy

### Unit tests

**Location:** `huggingface_hub/src/types/progress.rs` (in `#[cfg(test)]` module)

A `RecordingHandler` captures events for assertions:

```rust
struct RecordingHandler {
    events: Mutex<Vec<ProgressEvent>>,
}
impl ProgressHandler for RecordingHandler {
    fn on_progress(&self, event: &ProgressEvent) {
        self.events.lock().unwrap().push(event.clone());
    }
}
```

Tests:
- **Event ordering**: upload emits `UploadStart → UploadPhaseChange(Preparing) → ... → UploadComplete`
- **Phase completeness**: every `UploadPhase` variant is emitted during a mock upload flow
- **Download file lifecycle**: `DownloadFileStart → DownloadFileProgress(increasing) → DownloadFileComplete`
- **None handler is no-op**: `progress: None` doesn't panic or change behavior
- **Handler is Send + Sync**: compile-time check that `Arc<RecordingHandler>` satisfies bounds

### ProgressStream adapter tests

```rust
#[tokio::test]
async fn test_progress_stream_emits_events() {
    // Create synthetic byte stream, wrap in ProgressStream,
    // consume all chunks, assert DownloadFileProgress events
    // have monotonically increasing bytes_completed
}
```

### Integration tests

**Location:** `huggingface_hub/tests/integration_test.rs`

Gated on `HF_TOKEN` (and `HF_TEST_WRITE=1` for upload tests):

- **Download with progress**: download a known small file, attach `RecordingHandler`, verify `DownloadStart`, at least one `DownloadFileProgress`, and `DownloadFileComplete` with correct filename
- **Snapshot download with progress**: download 2-3 files, verify `total_files` matches, per-file events for each filename
- **Upload with progress** (`HF_TEST_WRITE=1`): upload a small file, verify `UploadStart` through `UploadComplete` with all phases

### CLI manual verification

No automated CLI rendering tests. Manual steps:

1. `hfrs download gpt2 config.json` — one bytes bar appears and completes
2. `hfrs download gpt2` — files count bar + per-file bytes bars
3. `hfrs download gpt2 --quiet` — no bars, only path printed
4. `hfrs upload <test-repo> ./small-file.txt` — phase progression visible
5. `hfrs upload <test-repo> ./test-dir/` — files bar + bytes bar
6. `HF_HUB_DISABLE_PROGRESS_BARS=1 hfrs download gpt2 config.json` — bars suppressed
7. Large xet-backed repo — aggregate bytes bar with rate display

---

## Module Structure

### New files

| File | Purpose |
|---|---|
| `huggingface_hub/src/types/progress.rs` | Core types: `ProgressEvent`, `UploadPhase`, `ProgressHandler`, `Progress`, `emit()` |
| `huggingface_hub/src/bin/hfrs/progress.rs` | `CliProgressHandler` with indicatif |

### Modified files

| File | Changes |
|---|---|
| `huggingface_hub/src/types/mod.rs` | Add `pub mod progress;` and re-export |
| `huggingface_hub/src/lib.rs` | Re-export `ProgressEvent`, `UploadPhase`, `ProgressHandler`, `Progress` |
| `huggingface_hub/src/repository.rs` | Add `progress: Progress` to 5 params structs |
| `huggingface_hub/src/api/files.rs` | Emit events in `create_commit`, `download_file`, `snapshot_download`; add `ProgressStream` |
| `huggingface_hub/src/xet.rs` | Accept `progress: &Progress`, add xet polling helpers |
| `huggingface_hub/Cargo.toml` | Add `indicatif` optional dep under `cli` feature |
| `huggingface_hub/src/bin/hfrs/commands/download.rs` | Create handler, pass to params |
| `huggingface_hub/src/bin/hfrs/commands/upload.rs` | Create handler, pass to params |
| `huggingface_hub/src/bin/hfrs/main.rs` | Add `mod progress;` |
| `huggingface_hub/tests/integration_test.rs` | Add progress-tracking test variants |

### Public API additions

All additive, no breaking changes:

```rust
pub trait ProgressHandler: Send + Sync {
    fn on_progress(&self, event: &ProgressEvent);
}
pub enum ProgressEvent { /* 11 variants */ }
pub enum UploadPhase { Preparing, CheckingUploadMode, Uploading, Committing }
pub type Progress = Option<Arc<dyn ProgressHandler>>;
```

### What does NOT change

- Return types of all existing methods
- Behavior when `progress: None` (the default)
- Blocking wrapper (`HFClientSync`, etc.) — progress works the same
- Feature flags — no new flag; `indicatif` is under existing `cli` feature
