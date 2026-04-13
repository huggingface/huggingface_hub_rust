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

/// Top-level progress event — either an upload or download event.
#[derive(Debug, Clone)]
pub enum ProgressEvent {
    Upload(UploadEvent),
    Download(DownloadEvent),
}

/// Progress events for upload operations.
///
/// Every variant that represents an in-progress state carries the current
/// `UploadPhase`, so consumers always know the phase from any single event
/// without tracking state across events.
#[derive(Debug, Clone)]
pub enum UploadEvent {
    /// Upload operation has started; total file count and bytes are known.
    Start { total_files: usize, total_bytes: u64 },
    /// Aggregate byte-level progress (xet/LFS upload).
    /// Phase is included so consumers always know the current phase.
    Progress {
        phase: UploadPhase,
        bytes_completed: u64,
        total_bytes: u64,
        bytes_per_sec: Option<f64>,
    },
    /// One or more individual files completed. Batched for efficiency
    /// during multi-file uploads (upload_folder).
    FileComplete {
        files: Vec<String>,
        phase: UploadPhase,
    },
    /// Entire upload operation finished (all files, commit created).
    Complete,
}

/// Progress events for download operations.
#[derive(Debug, Clone)]
pub enum DownloadEvent {
    /// Download operation has started; file count and total bytes known.
    Start { total_files: usize, total_bytes: u64 },
    /// Per-file progress update. Only includes files whose state changed
    /// since the last event (delta, not full snapshot). Batched for
    /// efficiency during multi-file downloads (snapshot_download).
    Progress { files: Vec<FileProgress> },
    /// Aggregate byte-level progress for xet batch transfers.
    /// Separate from per-file Progress because xet provides aggregate
    /// stats, not per-file byte counts.
    AggregateProgress {
        bytes_completed: u64,
        total_bytes: u64,
        bytes_per_sec: Option<f64>,
    },
    /// All downloads finished.
    Complete,
}

/// Per-file progress info, used inside `DownloadEvent::Progress`.
#[derive(Debug, Clone)]
pub struct FileProgress {
    pub filename: String,
    pub bytes_completed: u64,
    pub total_bytes: u64,
    pub status: FileStatus,
}

/// Lifecycle status of a single file within a transfer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStatus {
    Started,
    InProgress,
    Complete,
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

- **Nested enum** (`ProgressEvent::Upload(UploadEvent)` / `Download(DownloadEvent)`) — separates upload and download concerns at the type level. Consumers can match on the outer enum to route, then match the inner enum for specifics.
- **Phase on every upload event** (not a separate `PhaseChange` variant) — each in-progress upload event carries the current `UploadPhase`, so consumers always know the phase from any single event. No state tracking needed, no lost-event problems. Phase transitions are detected by observing the phase field change across events.
- **Delta-only download progress** — `DownloadEvent::Progress` only includes files whose state changed since the last event, not a full snapshot of all in-flight files. Keeps event payloads small during large snapshot downloads.
- **Batched file events** — both `DownloadEvent::Progress` and `UploadEvent::FileComplete` carry `Vec` payloads, allowing multiple file updates in a single event. This supports condensed reporting for `upload_folder` and `snapshot_download` without flooding the handler.
- **Separate `AggregateProgress`** for xet downloads — xet provides aggregate byte stats, not per-file byte counts. Keeping this as a distinct variant from per-file `Progress` avoids conflating two different data sources.
- **`Progress` type alias** (`Option<Arc<dyn ProgressHandler>>`) keeps params structs clean.
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
   → emit Upload(Start { total_files, total_bytes })

2. Call preupload_and_upload_lfs_files() to classify files
   → emit Upload(Progress { phase: CheckingUploadMode, bytes_completed: 0, ... })

3. Upload LFS files via xet_upload()
   → spawn xet polling task (100ms interval)
     → emit Upload(Progress { phase: Uploading, bytes_completed, total_bytes, bytes_per_sec })
   → as xet reports per-item completions:
     → emit Upload(FileComplete { files: [...], phase: Uploading })

4. POST /api/.../commit
   → emit Upload(Progress { phase: Committing, bytes_completed: total_bytes, ... })

5. Return CommitInfo
   → emit Upload(Complete)
```

Phase transitions are implicit — the consumer sees `phase` change from `Preparing` → `CheckingUploadMode` → `Uploading` → `Committing` across successive events. No dedicated phase-change event needed.

### Xet polling bridge (`xet.rs`)

Internal helper that polls xet progress and forwards to the handler:

```rust
fn spawn_xet_upload_poller(
    commit: XetUploadCommit,
    handler: Arc<dyn ProgressHandler>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut prev_completed: HashSet<String> = HashSet::new();
        loop {
            let report = commit.progress();
            // Emit aggregate byte progress
            handler.on_progress(&ProgressEvent::Upload(UploadEvent::Progress {
                phase: UploadPhase::Uploading,
                bytes_completed: report.total_bytes_completed,
                total_bytes: report.total_bytes,
                bytes_per_sec: report.total_bytes_completion_rate,
            }));
            // Emit FileComplete for newly completed files (delta)
            let newly_completed: Vec<String> = report.items.iter()
                .filter(|item| item.is_complete && !prev_completed.contains(&item.name))
                .map(|item| item.name.clone())
                .collect();
            if !newly_completed.is_empty() {
                prev_completed.extend(newly_completed.iter().cloned());
                handler.on_progress(&ProgressEvent::Upload(UploadEvent::FileComplete {
                    files: newly_completed,
                    phase: UploadPhase::Uploading,
                }));
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
}
```

A similar `spawn_xet_download_poller` emits `DownloadEvent::AggregateProgress` events from `XetFileDownloadGroup::progress()`, plus `DownloadEvent::Progress` with `FileStatus::Complete` entries for newly finished files.

The polling task is aborted when the transfer completes (the caller holds the `JoinHandle` and calls `.abort()` after `.commit().await` / `.finish().await` returns).

### Download flow — single file (`api/files.rs`)

**Non-xet path:**

```
1. HEAD request → get ETag, content-length
   → emit Download(Start { total_files: 1, total_bytes })

2. GET request → wrap response byte stream in ProgressStream adapter
   → emit Download(Progress { files: [FileProgress { status: InProgress, ... }] })
     on each chunk (single-element vec)

3. File written
   → emit Download(Progress { files: [FileProgress { status: Complete, ... }] })
   → emit Download(Complete)
```

**Xet path:**

```
1. HEAD request → detect X-Xet-Hash
   → emit Download(Start { total_files: 1, total_bytes })

2. xet_download_to_local_dir / xet_download_to_blob
   → spawn xet download poller
   → emit Download(AggregateProgress { bytes_completed, total_bytes, bytes_per_sec })

3. Complete
   → emit Download(Progress { files: [FileProgress { status: Complete, ... }] })
   → emit Download(Complete)
```

### Download flow — snapshot (`api/files.rs`)

```
1. List files via get_paths_info / list_tree
   → emit Download(Start { total_files, total_bytes })

2. Non-xet files: parallel workers (buffer_unordered, max_workers)
   Each worker emits Download(Progress { files: [FileProgress { ... }] })
   with a single-element vec per file per chunk.
   On completion: FileProgress { status: Complete }
   (handler is Arc, shared safely across tasks)

3. Xet files: xet_download_batch
   → spawn xet download poller
     → emit Download(AggregateProgress { ... }) for aggregate bytes
     → emit Download(Progress { files: [...] }) with FileStatus::Complete
       for newly finished files (delta — only files completed since last poll)

4. All done → emit Download(Complete)
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
                let status = if *this.bytes_read >= *this.total_bytes {
                    FileStatus::Complete
                } else {
                    FileStatus::InProgress
                };
                this.handler.on_progress(&ProgressEvent::Download(DownloadEvent::Progress {
                    files: vec![FileProgress {
                        filename: this.filename.clone(),
                        bytes_completed: *this.bytes_read,
                        total_bytes: *this.total_bytes,
                        status,
                    }],
                }));
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
| `Download(Start { total_files > 1, .. })` | Create files bar: `"Fetching {n} files"` |
| `Download(Start { total_files == 1, .. })` | No files bar |
| `Download(Progress { files })` where status = `Started` | Create per-file bytes bar |
| `Download(Progress { files })` where status = `InProgress` | Update per-file bar position |
| `Download(Progress { files })` where status = `Complete` | Finish + remove per-file bar, increment files bar |
| `Download(AggregateProgress { .. })` | Update aggregate bytes bar (xet batch) |
| `Download(Complete)` | Finish all bars |
| `Upload(Start { total_files, .. })` | Create files bar: `"Upload {n} LFS files"` if >1 |
| `Upload(Progress { phase: Uploading, .. })` | Create/update aggregate bytes bar |
| `Upload(Progress { phase: Committing, .. })` | Finish bytes bar, show spinner: `"Creating commit..."` |
| `Upload(FileComplete { files, .. })` | Increment files bar by `files.len()` |
| `Upload(Complete)` | Finish all bars |

The CLI handler detects phase transitions by comparing the `phase` field against its last-seen phase. When the phase changes (e.g., `Uploading` → `Committing`), it triggers bar transitions (finish bytes bar, start spinner).

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

- **Non-xet files**: each gets its own per-file bytes bar (created on `FileStatus::Started`, updated on `InProgress`, removed on `Complete`). Up to `max_workers` (default 8) per-file bars visible simultaneously. Each `Download(Progress)` event carries a single-element `files` vec per chunk.
- **Xet batch files**: a single aggregate bytes bar from `Download(AggregateProgress)` events. Individual completions arrive as `Download(Progress { files })` with `FileStatus::Complete` entries to increment the files counter.
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
- **Event ordering**: upload emits `Upload(Start) → Upload(Progress { phase: Preparing }) → ... → Upload(Complete)`
- **Phase progression**: phases advance monotonically across events (Preparing → CheckingUploadMode → Uploading → Committing)
- **Download file lifecycle**: `Download(Progress { status: Started }) → Download(Progress { status: InProgress, increasing bytes }) → Download(Progress { status: Complete })`
- **Delta-only delivery**: download progress events only contain files that changed
- **Batched FileComplete**: upload FileComplete can carry multiple filenames
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

- **Download with progress**: download a known small file, attach `RecordingHandler`, verify `Download(Start)`, at least one `Download(Progress)` with `InProgress`, and a final `Download(Progress)` with `Complete` + correct filename
- **Snapshot download with progress**: download 2-3 files, verify `total_files` matches, per-file progress events for each filename, all `Complete` before `Download(Complete)`
- **Upload with progress** (`HF_TEST_WRITE=1`): upload a small file, verify `Upload(Start)` through `Upload(Complete)` with phase progression across events

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
| `huggingface_hub/src/types/mod.rs` | Add `pub mod progress;` and re-export. `lib.rs` already does `pub use types::*` so progress types are automatically publicly re-exported — no changes needed in `lib.rs`. |
| `huggingface_hub/src/repository.rs` | Add `progress: Progress` to 5 params structs |
| `huggingface_hub/src/api/files.rs` | Emit events in `create_commit`, `download_file`, `snapshot_download`; add `ProgressStream` |
| `huggingface_hub/src/xet.rs` | Accept `progress: &Progress`, add xet polling helpers |
| `huggingface_hub/Cargo.toml` | Add `indicatif` optional dep under `cli` feature |
| `huggingface_hub/src/bin/hfrs/commands/download.rs` | Create handler, pass to params |
| `huggingface_hub/src/bin/hfrs/commands/upload.rs` | Create handler, pass to params |
| `huggingface_hub/src/bin/hfrs/main.rs` | Add `mod progress;` |
| `huggingface_hub/tests/integration_test.rs` | Add progress-tracking test variants |

**Note:** The blocking API (`HFClientSync`, `HFRepositorySync`) is generated by `sync_api!` macros in `huggingface_hub/src/macros.rs`. Since this design passes `progress` as a field on the existing params structs (which are taken by `&` reference), the macro-generated blocking wrappers forward it transparently — no changes to `macros.rs` or `blocking.rs` are needed.

### Methods not covered

The following methods are **not** in scope for this design but could be added later:

- `download_file_stream` — returns a raw byte stream `(Option<u64>, Box<dyn Stream<...>>)` where the consumer controls consumption. Progress could be added via a `ProgressStream` wrapper, but the caller already has direct access to chunk-level data.
- `download_file_to_bytes` — thin wrapper around `download_file_stream` that collects to `Bytes`. Would inherit progress if `download_file_stream` gained it.

### Public API additions

All additive, no breaking changes:

```rust
pub trait ProgressHandler: Send + Sync {
    fn on_progress(&self, event: &ProgressEvent);
}
pub enum ProgressEvent { Upload(UploadEvent), Download(DownloadEvent) }
pub enum UploadEvent { Start, Progress, FileComplete, Complete }
pub enum DownloadEvent { Start, Progress, AggregateProgress, Complete }
pub struct FileProgress { filename, bytes_completed, total_bytes, status }
pub enum FileStatus { Started, InProgress, Complete }
pub enum UploadPhase { Preparing, CheckingUploadMode, Uploading, Committing }
pub type Progress = Option<Arc<dyn ProgressHandler>>;
```

### What does NOT change

- Return types of all existing methods
- Behavior when `progress: None` (the default)
- Blocking wrapper (`HFClientSync`, etc.) — blocking methods are generated by `sync_api!` macros in `macros.rs` which forward params by reference; `progress` passes through as part of the params struct with no macro changes needed
- Feature flags — no new flag; `indicatif` is under existing `cli` feature
