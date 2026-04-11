# Per-File Upload Progress Bars Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show per-file progress bars during xet/LFS uploads instead of a single aggregate bar, mirroring the existing per-file download progress pattern.

**Architecture:** Add a `Vec<FileProgress>` field to `UploadEvent::Progress` for per-file data. In `xet.rs`, poll each `XetFileUpload` handle's `.progress()` in the 100ms loop, mapping `ItemProgressReport.item_name` back to `path_in_repo` via a `HashMap`. The CLI renderer creates/removes per-file indicatif bars with a cap of 10 visible, plus a summary line. Regular (non-LFS) files do not get progress bars.

**Tech Stack:** Rust, indicatif, hf-xet 1.5.1 (`ItemProgressReport`, `XetFileUpload`)

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `huggingface_hub/src/types/progress.rs` | Modify | Add `files: Vec<FileProgress>` to `UploadEvent::Progress` |
| `huggingface_hub/src/xet.rs` | Modify | Build item_name-to-repo-path map, poll per-file handles, emit per-file events |
| `huggingface_hub/src/api/files.rs` | Modify | Update all `UploadEvent::Progress` emit sites to include empty `files: vec![]` |
| `huggingface_hub/src/bin/hfrs/progress.rs` | Modify | Render per-file upload bars (max 10 visible, remove on complete, summary line) |

---

### Task 1: Extend `UploadEvent::Progress` with per-file data

**Files:**
- Modify: `huggingface_hub/src/types/progress.rs:27-42`

- [ ] **Step 1: Write the failing test**

Add a test in the `#[cfg(test)]` module that constructs an `UploadEvent::Progress` with a `files` field:

```rust
#[test]
fn upload_progress_with_per_file_data() {
    let handler = Arc::new(RecordingHandler::new());
    let progress: Progress = Some(handler.clone());

    emit(
        &progress,
        ProgressEvent::Upload(UploadEvent::Progress {
            phase: UploadPhase::Uploading,
            bytes_completed: 500,
            total_bytes: 1000,
            bytes_per_sec: Some(100.0),
            files: vec![
                FileProgress {
                    filename: "model/weights.bin".to_string(),
                    bytes_completed: 300,
                    total_bytes: 600,
                    status: FileStatus::InProgress,
                },
                FileProgress {
                    filename: "config.json".to_string(),
                    bytes_completed: 200,
                    total_bytes: 400,
                    status: FileStatus::InProgress,
                },
            ],
        }),
    );

    let events = handler.events();
    assert_eq!(events.len(), 1);
    if let ProgressEvent::Upload(UploadEvent::Progress { files, .. }) = &events[0] {
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].filename, "model/weights.bin");
        assert_eq!(files[1].filename, "config.json");
    } else {
        panic!("expected Upload(Progress)");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p huggingface-hub upload_progress_with_per_file_data`
Expected: FAIL — `UploadEvent::Progress` does not have a `files` field.

- [ ] **Step 3: Add `files` field to `UploadEvent::Progress`**

In `huggingface_hub/src/types/progress.rs`, change the `Progress` variant from:

```rust
/// Aggregate byte-level progress during xet/LFS upload.
Progress {
    phase: UploadPhase,
    bytes_completed: u64,
    total_bytes: u64,
    bytes_per_sec: Option<f64>,
},
```

to:

```rust
/// Byte-level progress during xet/LFS upload.
/// `files` contains per-file progress for xet uploads (may be empty
/// for phases without per-file granularity).
Progress {
    phase: UploadPhase,
    bytes_completed: u64,
    total_bytes: u64,
    bytes_per_sec: Option<f64>,
    files: Vec<FileProgress>,
},
```

- [ ] **Step 4: Fix all existing emit sites that construct `UploadEvent::Progress`**

Every existing construction of `UploadEvent::Progress` will now fail to compile because it's missing the `files` field. Add `files: vec![]` to each site:

In `huggingface_hub/src/api/files.rs` — there are 3 emit sites (Preparing at ~line 1182, CheckingUploadMode at ~line 1195, Committing at ~line 1260). Add `files: vec![]` to each.

In `huggingface_hub/src/xet.rs` — there are 2 emit sites (the poll loop at ~line 425, the final emit at ~line 444). Add `files: vec![]` to each for now (Task 2 will populate them).

- [ ] **Step 5: Fix existing tests that pattern-match on `UploadEvent::Progress`**

In `huggingface_hub/src/types/progress.rs`, the `upload_phase_progression` test constructs `UploadEvent::Progress` without `files`. Add `files: vec![]` to line ~160:

```rust
ProgressEvent::Upload(UploadEvent::Progress {
    phase: phase.clone(),
    bytes_completed: 0,
    total_bytes: 100,
    bytes_per_sec: None,
    files: vec![],
}),
```

Also fix the `emit_records_events` test around line ~158:

```rust
ProgressEvent::Upload(UploadEvent::Progress {
    phase: UploadPhase::Uploading,
    bytes_completed: 512,
    total_bytes: 1024,
    bytes_per_sec: Some(100.0),
    files: vec![],
}),
```

- [ ] **Step 6: Run all tests to verify everything compiles and passes**

Run: `cargo test -p huggingface-hub`
Expected: PASS (all existing tests + new test).

- [ ] **Step 7: Run fmt and clippy**

```bash
cargo +nightly fmt
cargo clippy -p huggingface-hub --all-features -- -D warnings
```

- [ ] **Step 8: Commit**

```bash
git add huggingface_hub/src/types/progress.rs huggingface_hub/src/api/files.rs huggingface_hub/src/xet.rs
git commit -m "feat: add per-file progress data to UploadEvent::Progress"
```

---

### Task 2: Emit per-file progress from xet upload polling loop

**Files:**
- Modify: `huggingface_hub/src/xet.rs:400-450`

- [ ] **Step 1: Write the failing test**

This is an integration-level change in async code that requires a real xet session, so we can't easily unit test it in isolation. Instead, we'll verify correctness by:
1. Ensuring the code compiles with `--all-features`
2. The existing integration tests still pass
3. Manual verification with a real upload (documented at end of plan)

Skip writing a new test for this task — the type system enforces correctness of the `FileProgress` construction, and Task 1's test already validates the event shape.

- [ ] **Step 2: Build the `item_name` to `path_in_repo` mapping**

In `xet_upload()` in `huggingface_hub/src/xet.rs`, replace the `task_ids_in_order` Vec with a `HashMap` that maps xet-core's `item_name` (the value `ItemProgressReport.item_name` will contain) to `path_in_repo`.

For `AddSource::File(path)`: xet-core sets `item_name` to `std::path::absolute(path).to_str()`. Mimic this.
For `AddSource::Bytes(_)`: xet-core uses the `tracking_name` we pass. Currently we pass `None`. Pass `Some(path_in_repo.clone())` instead so `item_name` equals the repo path directly.

Replace the loop at lines ~400-415:

```rust
// Map from xet-core's item_name to path_in_repo.
// For File uploads, xet-core sets item_name to std::path::absolute(path).
// We mimic that logic here to build the reverse mapping.
// For Bytes uploads, we pass path_in_repo as tracking_name so item_name == path_in_repo.
let mut item_name_to_repo_path: HashMap<String, String> = HashMap::with_capacity(files.len());
let mut task_ids_in_order = Vec::with_capacity(files.len());

for (path_in_repo, source) in files {
    tracing::info!(path = path_in_repo.as_str(), "queuing xet upload");
    let handle = match source {
        AddSource::File(path) => {
            // Mimic xet-core's item_name derivation: std::path::absolute(path).to_str()
            // See xet-data upload_commit.rs XetUploadCommitInner::upload_from_path
            if let Ok(abs) = std::path::absolute(path) {
                if let Some(s) = abs.to_str() {
                    item_name_to_repo_path.insert(s.to_owned(), path_in_repo.clone());
                }
            }
            commit
                .upload_from_path(path.clone(), Sha256Policy::Compute)
                .await
                .map_err(|e| HFError::Other(format!("Xet upload failed: {e}")))?
        },
        AddSource::Bytes(bytes) => {
            item_name_to_repo_path.insert(path_in_repo.clone(), path_in_repo.clone());
            commit
                .upload_bytes(bytes.clone(), Sha256Policy::Compute, Some(path_in_repo.clone()))
                .await
                .map_err(|e| HFError::Other(format!("Xet upload failed: {e}")))?
        },
    };
    task_ids_in_order.push(handle.task_id());
}
```

Add `use std::collections::HashMap;` to the top of `xet.rs` if not already present.

- [ ] **Step 3: Update the polling loop to emit per-file progress**

Access the per-file upload handles via `commit`'s internal `file_handles` field. The `XetUploadCommit` stores file handles in a `Mutex<Vec<XetFileUpload>>` — we need to read them in the poll loop.

Check if `XetUploadCommit` exposes the file handles publicly. If not, we'll need to store our own `Vec<XetFileUpload>` from the return values.

Actually, `upload_from_path` and `upload_bytes` both return `XetFileUpload`. We already have the handles — we just don't keep them. Change the loop to collect them:

```rust
let mut upload_handles: Vec<(String, XetFileUpload)> = Vec::with_capacity(files.len());

for (path_in_repo, source) in files {
    tracing::info!(path = path_in_repo.as_str(), "queuing xet upload");
    let handle = match source {
        AddSource::File(path) => {
            if let Ok(abs) = std::path::absolute(path) {
                if let Some(s) = abs.to_str() {
                    item_name_to_repo_path.insert(s.to_owned(), path_in_repo.clone());
                }
            }
            commit
                .upload_from_path(path.clone(), Sha256Policy::Compute)
                .await
                .map_err(|e| HFError::Other(format!("Xet upload failed: {e}")))?
        },
        AddSource::Bytes(bytes) => {
            item_name_to_repo_path.insert(path_in_repo.clone(), path_in_repo.clone());
            commit
                .upload_bytes(bytes.clone(), Sha256Policy::Compute, Some(path_in_repo.clone()))
                .await
                .map_err(|e| HFError::Other(format!("Xet upload failed: {e}")))?
        },
    };
    task_ids_in_order.push(handle.task_id());
    upload_handles.push((path_in_repo.clone(), handle));
}
```

Wait — `XetFileUpload` may not be `Clone` or `Send` in a way that lets us share it with the polling task. Let me check. Looking at the xet-core source, `XetFileUpload` wraps an `Arc<XetFileUploadInner>` and a `TaskRuntime`, both of which are `Clone + Send + Sync`. And `XetFileUpload` itself has `.progress() -> Option<ItemProgressReport>` which is lock-free (atomic reads). So we can clone the handles and share them.

Replace the poll loop at lines ~418-433:

```rust
tracing::info!(file_count = files.len(), "committing xet uploads");
let poll_handle = progress.as_ref().map(|handler| {
    let handler = handler.clone();
    let commit = commit.clone();
    let handles = upload_handles.clone();
    let name_map = item_name_to_repo_path.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            let report = commit.progress();

            let mut file_progress: Vec<FileProgress> = Vec::new();
            for (_repo_path, handle) in &handles {
                if let Some(item_report) = handle.progress() {
                    let repo_path = name_map
                        .get(&item_report.item_name)
                        .cloned()
                        .unwrap_or(item_report.item_name.clone());
                    let status = if item_report.bytes_completed == 0 {
                        FileStatus::Started
                    } else if item_report.bytes_completed >= item_report.total_bytes
                        && item_report.total_bytes > 0
                    {
                        FileStatus::Complete
                    } else {
                        FileStatus::InProgress
                    };
                    file_progress.push(FileProgress {
                        filename: repo_path,
                        bytes_completed: item_report.bytes_completed,
                        total_bytes: item_report.total_bytes,
                        status,
                    });
                }
            }

            handler.on_progress(&ProgressEvent::Upload(UploadEvent::Progress {
                phase: UploadPhase::Uploading,
                bytes_completed: report.total_bytes_completed,
                total_bytes: report.total_bytes,
                bytes_per_sec: report.total_bytes_completion_rate,
                files: file_progress,
            }));
        }
    })
});
```

- [ ] **Step 4: Update the final progress emit after commit completes**

The emit at ~line 442 also needs per-file data. At this point all files are complete, so emit all files as `FileStatus::Complete`:

```rust
let final_files: Vec<FileProgress> = upload_handles
    .iter()
    .map(|(repo_path, _)| FileProgress {
        filename: repo_path.clone(),
        bytes_completed: 0, // exact value doesn't matter, status is Complete
        total_bytes: 0,
        status: FileStatus::Complete,
    })
    .collect();

progress::emit(
    progress,
    ProgressEvent::Upload(UploadEvent::Progress {
        phase: UploadPhase::Uploading,
        bytes_completed: results.progress.total_bytes_completed,
        total_bytes: results.progress.total_bytes,
        bytes_per_sec: results.progress.total_bytes_completion_rate,
        files: final_files,
    }),
);
```

- [ ] **Step 5: Verify compilation**

Run: `cargo build -p huggingface-hub --all-features`
Expected: compiles successfully.

- [ ] **Step 6: Run fmt and clippy**

```bash
cargo +nightly fmt
cargo clippy -p huggingface-hub --all-features -- -D warnings
```

- [ ] **Step 7: Commit**

```bash
git add huggingface_hub/src/xet.rs
git commit -m "feat: emit per-file progress from xet upload polling loop"
```

---

### Task 3: Render per-file upload progress bars in the CLI

**Files:**
- Modify: `huggingface_hub/src/bin/hfrs/progress.rs`

- [ ] **Step 1: Add upload file bar tracking to `ProgressState`**

Add a field to track the upload file bars and a constant for the max visible bars. In `progress.rs`:

```rust
const MAX_VISIBLE_UPLOAD_BARS: usize = 10;
```

Add to `ProgressState`:

```rust
struct ProgressState {
    files_bar: Option<ProgressBar>,
    bytes_bar: Option<ProgressBar>,
    file_bars: HashMap<String, ProgressBar>,
    upload_file_bars: HashMap<String, ProgressBar>,
    last_upload_phase: Option<UploadPhase>,
    spinner: Option<ProgressBar>,
    total_files: usize,
}
```

Initialize `upload_file_bars: HashMap::new()` in `CliProgressHandler::new`.

- [ ] **Step 2: Update `handle_upload` to process per-file progress**

In the `UploadEvent::Progress` match arm, after the existing aggregate bar update (lines ~224-229), add per-file bar management:

```rust
if *phase == UploadPhase::Uploading {
    // Update aggregate bar
    if let Some(ref bar) = state.bytes_bar {
        bar.set_length(*total_bytes);
        bar.set_position(*bytes_completed);
    }

    // Per-file bars
    for fp in files {
        match fp.status {
            FileStatus::Started => {
                if !state.upload_file_bars.contains_key(&fp.filename)
                    && state.upload_file_bars.len() < MAX_VISIBLE_UPLOAD_BARS
                {
                    let bar = self.multi.add(ProgressBar::new(fp.total_bytes));
                    bar.set_style(bytes_style());
                    bar.set_message(truncate_filename(&fp.filename, 40));
                    state.upload_file_bars.insert(fp.filename.clone(), bar);
                }
            },
            FileStatus::InProgress => {
                // Create bar if we haven't yet and there's room
                if !state.upload_file_bars.contains_key(&fp.filename)
                    && state.upload_file_bars.len() < MAX_VISIBLE_UPLOAD_BARS
                {
                    let bar = self.multi.add(ProgressBar::new(fp.total_bytes));
                    bar.set_style(bytes_style());
                    bar.set_message(truncate_filename(&fp.filename, 40));
                    state.upload_file_bars.insert(fp.filename.clone(), bar);
                }
                if let Some(bar) = state.upload_file_bars.get(&fp.filename) {
                    bar.set_position(fp.bytes_completed);
                }
            },
            FileStatus::Complete => {
                if let Some(bar) = state.upload_file_bars.remove(&fp.filename) {
                    bar.finish_and_clear();
                    self.multi.remove(&bar);
                }
                if let Some(ref bar) = state.files_bar {
                    bar.inc(1);
                }
            },
        }
    }
}
```

- [ ] **Step 3: Clean up upload file bars on `UploadEvent::Complete`**

In the `Complete` match arm, add cleanup for upload file bars:

```rust
UploadEvent::Complete => {
    if let Some(spinner) = state.spinner.take() {
        spinner.finish_and_clear();
        self.multi.remove(&spinner);
    }
    if let Some(bar) = state.files_bar.take() {
        bar.finish_and_clear();
        self.multi.remove(&bar);
    }
    if let Some(bar) = state.bytes_bar.take() {
        bar.finish_and_clear();
        self.multi.remove(&bar);
    }
    for (_, bar) in state.upload_file_bars.drain() {
        bar.finish_and_clear();
        self.multi.remove(&bar);
    }
},
```

- [ ] **Step 4: Remove per-file bars when transitioning away from Uploading phase**

When the phase changes from `Uploading` to `Committing`, clear all per-file upload bars. In the phase transition block (~line 179), add to the `Committing` arm before creating the spinner:

```rust
UploadPhase::Committing => {
    // Clear per-file upload bars
    for (_, bar) in state.upload_file_bars.drain() {
        bar.finish_and_clear();
        self.multi.remove(&bar);
    }
    if let Some(ref bar) = state.bytes_bar {
        bar.set_position(bar.length().unwrap_or(0));
        bar.finish_and_clear();
        self.multi.remove(bar);
    }
    state.bytes_bar = None;
    let bar = self.multi.add(ProgressBar::new_spinner());
    bar.set_style(spinner_style());
    bar.set_message("Creating commit...");
    bar.enable_steady_tick(std::time::Duration::from_millis(100));
    state.spinner = Some(bar);
},
```

- [ ] **Step 5: Verify compilation**

Run: `cargo build -p huggingface-hub --all-features`
Expected: compiles successfully.

- [ ] **Step 6: Run fmt and clippy**

```bash
cargo +nightly fmt
cargo clippy -p huggingface-hub --all-features -- -D warnings
```

- [ ] **Step 7: Commit**

```bash
git add huggingface_hub/src/bin/hfrs/progress.rs
git commit -m "feat: render per-file upload progress bars in CLI (max 10 visible)"
```

---

### Task 4: Remove aggregate bytes bar for multi-file uploads

Now that we have per-file bars, showing both per-file bars and an aggregate bytes bar is redundant for multi-file uploads. Keep the aggregate bar only for single-file uploads (where we don't show per-file bars anyway).

**Files:**
- Modify: `huggingface_hub/src/bin/hfrs/progress.rs`

- [ ] **Step 1: Conditionally create the aggregate bytes bar**

In `UploadEvent::Start`, only create the bytes bar when there's a single file:

```rust
UploadEvent::Start {
    total_files,
    total_bytes,
} => {
    state.total_files = *total_files;
    if *total_files > 1 {
        let bar = self.multi.add(ProgressBar::new(*total_files as u64));
        bar.set_style(files_style());
        bar.set_message(format!("Upload {} files", total_files));
        state.files_bar = Some(bar);
    }
    if *total_bytes > 0 && *total_files <= 1 {
        let bar = self.multi.add(ProgressBar::new(*total_bytes));
        bar.set_style(bytes_style());
        bar.set_message("Uploading");
        state.bytes_bar = Some(bar);
    }
},
```

- [ ] **Step 2: Verify compilation and run tests**

```bash
cargo build -p huggingface-hub --all-features
cargo test -p huggingface-hub
```

- [ ] **Step 3: Run fmt and clippy**

```bash
cargo +nightly fmt
cargo clippy -p huggingface-hub --all-features -- -D warnings
```

- [ ] **Step 4: Commit**

```bash
git add huggingface_hub/src/bin/hfrs/progress.rs
git commit -m "feat: show aggregate bytes bar only for single-file uploads"
```

---

### Task 5: Manual verification

- [ ] **Step 1: Build the binary in release mode**

```bash
cargo build -p huggingface-hub --release --all-features
```

- [ ] **Step 2: Test single file upload**

Upload a single file to a test repo. Verify:
- Spinner shows for Preparing and CheckingUploadMode phases
- Single aggregate bytes bar shows during Uploading (no per-file bar)
- Spinner shows for Committing phase

- [ ] **Step 3: Test multi-file upload**

Upload 3+ LFS files. Verify:
- Files bar shows "Upload N files" count
- Per-file bars appear during Uploading phase with repo-relative filenames
- Completed file bars disappear, making room for pending files
- No aggregate bytes bar shown
- Phase transitions (Committing spinner) clean up all bars

- [ ] **Step 4: Test with >10 files**

Upload 12+ LFS files. Verify:
- Max 10 per-file bars visible at once
- As files complete and bars are removed, new files get bars
- Files bar count increments correctly

---

## Design Decisions

1. **`files` field is a `Vec<FileProgress>`, not `Option`**: Empty vec for phases without per-file data (Preparing, CheckingUploadMode, Committing). Avoids Option nesting.

2. **Name mapping via `HashMap<String, String>`**: Maps xet-core's `item_name` (absolute local path for file uploads, `path_in_repo` for bytes uploads) to `path_in_repo`. Comment documents that we mimic xet-core's `std::path::absolute()` logic.

3. **Regular files skip progress bars**: They're base64-inlined in the NDJSON body — there's no streaming to track.

4. **Max 10 visible bars**: Completed bars are removed from display immediately, freeing slots for pending files. The files bar serves as the summary line showing overall file count progress.

5. **Aggregate bytes bar only for single-file uploads**: When per-file bars are shown, the aggregate is redundant.
