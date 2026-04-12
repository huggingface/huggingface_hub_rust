# Upload Progress Bars: Processing vs Transfer + Overflow Aggregation

**Date:** 2026-04-11
**Branch:** `assaf/progress`
**Status:** Design approved

## Problem

The Rust `hfrs` CLI shows a single aggregate bytes bar during xet uploads, but xet-core's `GroupProgressReport` actually tracks two distinct byte streams:

1. **Processing bytes** (`total_bytes` / `total_bytes_completed`) — dedup/chunking work
2. **Transfer bytes** (`total_transfer_bytes` / `total_transfer_bytes_completed`) — actual network upload

The Python `huggingface_hub` library shows both as separate bars via `XetProgressReporter`. Our Rust implementation currently ignores the transfer fields entirely.

Additionally, when more than 10 files are being uploaded, overflow files are silently queued with no visual indication. Python collapses overflow into a `[+ N files]` aggregate bar in the last visible slot.

## Goals

- Expose processing and transfer byte progress through the library's `UploadEvent::Progress` type so any consumer can use both
- Render two summary bars in the CLI matching Python: "Processing Files (M / N)" and "New Data Upload"
- Show `[+ N files]` aggregate bar when active files exceed `MAX_VISIBLE_UPLOAD_BARS` (10)

## Non-Goals

- Changing download progress (unrelated)
- Changing `FileProgress`, `FileStatus`, or `ProgressHandler` types
- Notebook/console environment detection (Rust is CLI-only)

## Design

### 1. Library types — `UploadEvent::Progress`

Add three fields to the `Progress` variant in `types/progress.rs`:

```rust
Progress {
    phase: UploadPhase,
    // Processing/dedup bytes (existing)
    bytes_completed: u64,
    total_bytes: u64,
    bytes_per_sec: Option<f64>,
    // Actual network transfer bytes (new)
    transfer_bytes_completed: u64,
    transfer_bytes: u64,
    transfer_bytes_per_sec: Option<f64>,
    // Per-file progress (existing)
    files: Vec<FileProgress>,
}
```

The existing `bytes_completed` / `total_bytes` fields retain their names and map to xet-core's processing/dedup counters. The new `transfer_bytes*` fields map to `GroupProgressReport::total_transfer_bytes*`.

### 2. Emit layer — `xet.rs` + `api/files.rs`

**Polling loop (`xet.rs`):** Read both `total_bytes*` and `total_transfer_bytes*` from `GroupProgressReport` and pass through to the `Progress` event.

**Final emit after commit (`xet.rs`):** Same — populate both sets of fields from `results.progress`.

**Non-uploading phases (`files.rs`):** Emit `0` for all transfer fields (Preparing, CheckingUploadMode, Committing phases have no transfer activity).

### 3. CLI renderer — `progress.rs`

#### Two summary bars

Replace the single `bytes_bar` with two bars during upload:

- **`processing_bar`**: Style matches Python's "Processing Files (M / N)" — shows `bytes_completed / total_bytes` with `bytes_per_sec` rate. The description updates with completed file count vs total file count.
- **`transfer_bar`**: "New Data Upload" — shows `transfer_bytes_completed / transfer_bytes` with `transfer_bytes_per_sec` rate.

Both bars are shown for all uploads (single-file and multi-file). They are created during the first `Uploading` phase progress event (not during `Start`, since transfer totals aren't known yet).

#### Overflow aggregation

Replace the current dynamic create/remove bar approach with a fixed slot pool:

- `MAX_VISIBLE_UPLOAD_BARS` (10) bar slots, initially empty
- Each tick, iterate through active (non-complete) files in insertion order
- If active files <= 10, each gets its own slot with description and position updated in-place
- If active files > 10, first 9 get individual bars (slots 0-8), and slot 9 becomes `[+ N files]` showing combined bytes across all overflow files
- Completed files are removed from the active state only when `active_count > MAX_VISIBLE_UPLOAD_BARS`, to make room for incoming files (matches Python's eviction logic)
- Bar slots are reused by overwriting description/total/position rather than creating and removing bars from `MultiProgress`

#### State changes

`ProgressState` changes:
- Remove: `bytes_bar`, `upload_file_bars: HashMap`, `upload_completed_files: HashSet`
- Add: `processing_bar: Option<ProgressBar>`, `transfer_bar: Option<ProgressBar>`
- Add: `upload_file_slots: Vec<Option<ProgressBar>>` (fixed size 10)
- Add: `upload_active_files: OrderedMap<String, FileProgress>` (insertion-ordered, like Python's `OrderedDict`)
- Add: `upload_known_files: HashSet<String>`, `upload_completed_files: HashSet<String>` (for file counting)

Note: Rust's `IndexMap` crate (or `BTreeMap` with insertion index) provides ordered map semantics. Since `indexmap` is already used transitively, it's a natural choice.

#### Cleanup

- `Committing` phase transition: close and remove both summary bars and all file slot bars
- `Complete` event: same cleanup as safety net

### 4. Files changed

| File | Change |
|------|--------|
| `huggingface_hub/src/types/progress.rs` | Add 3 fields to `UploadEvent::Progress`, update tests |
| `huggingface_hub/src/xet.rs` | Polling loop + final emit read both byte streams |
| `huggingface_hub/src/api/files.rs` | Non-uploading phase emits include transfer fields (zeroed) |
| `huggingface_hub/src/bin/hfrs/progress.rs` | Two summary bars, fixed slot pool, overflow aggregation |
| `huggingface_hub/Cargo.toml` | Add `indexmap` dependency if not already present |

### 5. Testing

- Unit tests in `types/progress.rs` updated for new fields
- Manual test: upload a folder with >10 files and verify:
  - Two summary bars appear ("Processing Files" and "New Data Upload")
  - Per-file bars show for first 9 active files
  - 10th slot shows `[+ N files]` with aggregate bytes
  - Completed files are evicted to make room for new ones
  - Both summary bars finish cleanly on commit
- Manual test: single-file upload shows both summary bars
