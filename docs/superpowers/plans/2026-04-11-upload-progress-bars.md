# Upload Progress Bars: Processing vs Transfer + Overflow Aggregation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show two summary bars (processing + transfer) during xet uploads and collapse overflow per-file bars into `[+ N files]`, matching the Python `huggingface_hub` CLI.

**Architecture:** Add `transfer_bytes*` fields to `UploadEvent::Progress` so any library consumer gets both byte streams. Wire them from xet-core's `GroupProgressReport` in the polling loop. Update the CLI renderer to show two summary bars and use a fixed slot pool with overflow aggregation.

**Tech Stack:** Rust, indicatif, indexmap

**Spec:** `docs/superpowers/specs/2026-04-11-upload-progress-bars-design.md`

---

### File Structure

| File | Role | Change |
|------|------|--------|
| `huggingface_hub/src/types/progress.rs` | Library progress types | Add 3 fields to `UploadEvent::Progress`, update all tests |
| `huggingface_hub/src/xet.rs` | Xet upload polling | Populate new transfer fields from `GroupProgressReport` |
| `huggingface_hub/src/api/files.rs` | Non-xet upload emits | Add zeroed transfer fields to 3 emit sites |
| `huggingface_hub/src/bin/hfrs/progress.rs` | CLI renderer | Two summary bars, fixed slot pool, overflow aggregation |
| `huggingface_hub/Cargo.toml` | Dependencies | Add `indexmap` |

---

### Task 1: Add transfer fields to `UploadEvent::Progress`

**Files:**
- Modify: `huggingface_hub/src/types/progress.rs`

- [ ] **Step 1: Add the three new fields to `UploadEvent::Progress`**

In `huggingface_hub/src/types/progress.rs`, change the `Progress` variant from:

```rust
    Progress {
        phase: UploadPhase,
        bytes_completed: u64,
        total_bytes: u64,
        bytes_per_sec: Option<f64>,
        files: Vec<FileProgress>,
    },
```

to:

```rust
    Progress {
        phase: UploadPhase,
        bytes_completed: u64,
        total_bytes: u64,
        bytes_per_sec: Option<f64>,
        transfer_bytes_completed: u64,
        transfer_bytes: u64,
        transfer_bytes_per_sec: Option<f64>,
        files: Vec<FileProgress>,
    },
```

- [ ] **Step 2: Fix all test sites that construct `UploadEvent::Progress`**

There are 4 tests in `progress.rs` that construct `UploadEvent::Progress`. Add `transfer_bytes_completed: 0, transfer_bytes: 0, transfer_bytes_per_sec: None,` to each:

1. `emit_records_events` test (~line 161):
```rust
        emit(
            &progress,
            ProgressEvent::Upload(UploadEvent::Progress {
                phase: UploadPhase::Uploading,
                bytes_completed: 512,
                total_bytes: 1024,
                bytes_per_sec: Some(100.0),
                transfer_bytes_completed: 0,
                transfer_bytes: 0,
                transfer_bytes_per_sec: None,
                files: vec![],
            }),
        );
```

2. `upload_phase_progression` test (~line 244):
```rust
            emit(
                &progress,
                ProgressEvent::Upload(UploadEvent::Progress {
                    phase: phase.clone(),
                    bytes_completed: 0,
                    total_bytes: 100,
                    bytes_per_sec: None,
                    transfer_bytes_completed: 0,
                    transfer_bytes: 0,
                    transfer_bytes_per_sec: None,
                    files: vec![],
                }),
            );
```

3. `upload_progress_with_per_file_data` test (~line 272):
```rust
        emit(
            &progress,
            ProgressEvent::Upload(UploadEvent::Progress {
                phase: UploadPhase::Uploading,
                bytes_completed: 500,
                total_bytes: 1000,
                bytes_per_sec: Some(100.0),
                transfer_bytes_completed: 250,
                transfer_bytes: 800,
                transfer_bytes_per_sec: Some(50.0),
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
```

Also update the assertion in that test to verify the new fields:
```rust
        if let ProgressEvent::Upload(UploadEvent::Progress {
            files,
            transfer_bytes_completed,
            transfer_bytes,
            transfer_bytes_per_sec,
            ..
        }) = &events[0]
        {
            assert_eq!(files.len(), 2);
            assert_eq!(files[0].filename, "model/weights.bin");
            assert_eq!(files[1].filename, "config.json");
            assert_eq!(*transfer_bytes_completed, 250);
            assert_eq!(*transfer_bytes, 800);
            assert_eq!(*transfer_bytes_per_sec, Some(50.0));
        } else {
            panic!("expected Upload(Progress)");
        }
```

- [ ] **Step 3: Verify compilation and tests pass**

Run:
```bash
cargo test -p huggingface-hub --lib -- progress
```

Expected: All progress tests pass.

- [ ] **Step 4: Commit**

```bash
git add huggingface_hub/src/types/progress.rs
git commit -m "feat: add transfer byte fields to UploadEvent::Progress"
```

---

### Task 2: Wire transfer fields through emit sites

**Files:**
- Modify: `huggingface_hub/src/xet.rs`
- Modify: `huggingface_hub/src/api/files.rs`

- [ ] **Step 1: Update the polling loop emit in `xet.rs`**

In `huggingface_hub/src/xet.rs`, in the polling loop (~line 577), change:

```rust
                    handler.on_progress(&ProgressEvent::Upload(UploadEvent::Progress {
                        phase: UploadPhase::Uploading,
                        bytes_completed: report.total_bytes_completed,
                        total_bytes: report.total_bytes,
                        bytes_per_sec: report.total_bytes_completion_rate,
                        files: file_progress,
                    }));
```

to:

```rust
                    handler.on_progress(&ProgressEvent::Upload(UploadEvent::Progress {
                        phase: UploadPhase::Uploading,
                        bytes_completed: report.total_bytes_completed,
                        total_bytes: report.total_bytes,
                        bytes_per_sec: report.total_bytes_completion_rate,
                        transfer_bytes_completed: report.total_transfer_bytes_completed,
                        transfer_bytes: report.total_transfer_bytes,
                        transfer_bytes_per_sec: report.total_transfer_bytes_completion_rate,
                        files: file_progress,
                    }));
```

- [ ] **Step 2: Update the final emit after commit in `xet.rs`**

In `huggingface_hub/src/xet.rs`, the final emit (~line 606), change:

```rust
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

to:

```rust
        progress::emit(
            progress,
            ProgressEvent::Upload(UploadEvent::Progress {
                phase: UploadPhase::Uploading,
                bytes_completed: results.progress.total_bytes_completed,
                total_bytes: results.progress.total_bytes,
                bytes_per_sec: results.progress.total_bytes_completion_rate,
                transfer_bytes_completed: results.progress.total_transfer_bytes_completed,
                transfer_bytes: results.progress.total_transfer_bytes,
                transfer_bytes_per_sec: results.progress.total_transfer_bytes_completion_rate,
                files: final_files,
            }),
        );
```

- [ ] **Step 3: Update the three non-uploading phase emits in `files.rs`**

In `huggingface_hub/src/api/files.rs`, there are three `UploadEvent::Progress` emits for Preparing (~line 1150), CheckingUploadMode (~line 1164), and Committing (~line 1231). Add zeroed transfer fields to each. For example, the Preparing emit changes from:

```rust
        progress::emit(
            &params.progress,
            ProgressEvent::Upload(UploadEvent::Progress {
                phase: UploadPhase::Preparing,
                bytes_completed: 0,
                total_bytes,
                bytes_per_sec: None,
                files: vec![],
            }),
        );
```

to:

```rust
        progress::emit(
            &params.progress,
            ProgressEvent::Upload(UploadEvent::Progress {
                phase: UploadPhase::Preparing,
                bytes_completed: 0,
                total_bytes,
                bytes_per_sec: None,
                transfer_bytes_completed: 0,
                transfer_bytes: 0,
                transfer_bytes_per_sec: None,
                files: vec![],
            }),
        );
```

Apply the same pattern to the CheckingUploadMode and Committing emits.

- [ ] **Step 4: Verify compilation**

Run:
```bash
cargo clippy -p huggingface-hub --all-features -- -D warnings
```

Expected: No errors or warnings.

- [ ] **Step 5: Commit**

```bash
git add huggingface_hub/src/xet.rs huggingface_hub/src/api/files.rs
git commit -m "feat: populate transfer byte fields from GroupProgressReport"
```

---

### Task 3: Add `indexmap` dependency and update CLI renderer state

**Files:**
- Modify: `huggingface_hub/Cargo.toml`
- Modify: `huggingface_hub/src/bin/hfrs/progress.rs`

- [ ] **Step 1: Add `indexmap` to `huggingface_hub/Cargo.toml`**

Add `indexmap` in the `[dependencies]` section (alphabetical order):

```toml
indexmap = "2"
```

- [ ] **Step 2: Replace upload state fields in `ProgressState`**

In `huggingface_hub/src/bin/hfrs/progress.rs`, update the imports at the top:

```rust
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::Write;
use std::sync::Mutex;

use huggingface_hub::{
    DownloadEvent, FileProgress, FileStatus, ProgressEvent, ProgressHandler, UploadEvent, UploadPhase,
};
use indexmap::IndexMap;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
```

Replace `ProgressState` with:

```rust
struct ProgressState {
    // Download state (unchanged)
    files_bar: Option<ProgressBar>,
    bytes_bar: Option<ProgressBar>,
    file_bars: HashMap<String, ProgressBar>,
    download_queue: VecDeque<(String, u64)>,
    total_files: usize,
    // Upload state (new)
    processing_bar: Option<ProgressBar>,
    transfer_bar: Option<ProgressBar>,
    upload_file_slots: Vec<Option<ProgressBar>>,
    upload_active_files: IndexMap<String, FileProgress>,
    upload_known_files: HashSet<String>,
    upload_completed_files: HashSet<String>,
    last_upload_phase: Option<UploadPhase>,
    spinner: Option<ProgressBar>,
    upload_total_files: usize,
}
```

Update the `CliProgressHandler::new()` constructor to initialize the new fields:

```rust
    pub fn new(multi: MultiProgress) -> Self {
        Self {
            multi,
            state: Mutex::new(ProgressState {
                files_bar: None,
                bytes_bar: None,
                file_bars: HashMap::new(),
                download_queue: VecDeque::new(),
                total_files: 0,
                processing_bar: None,
                transfer_bar: None,
                upload_file_slots: Vec::new(),
                upload_active_files: IndexMap::new(),
                upload_known_files: HashSet::new(),
                upload_completed_files: HashSet::new(),
                last_upload_phase: None,
                spinner: None,
                upload_total_files: 0,
            }),
        }
    }
```

- [ ] **Step 3: Verify it compiles (will have dead code warnings, that's expected)**

Run:
```bash
cargo check -p huggingface-hub --features cli
```

Expected: Compiles (may have warnings about unused fields — fixed in next task).

- [ ] **Step 4: Commit**

```bash
git add huggingface_hub/Cargo.toml huggingface_hub/src/bin/hfrs/progress.rs
git commit -m "refactor: replace upload progress state with fixed slot pool and indexmap"
```

---

### Task 4: Rewrite `handle_upload` with two summary bars and overflow aggregation

**Files:**
- Modify: `huggingface_hub/src/bin/hfrs/progress.rs`

This is the main rendering task. Replace the entire `handle_upload` and `process_upload_file_progress` methods.

- [ ] **Step 1: Replace `handle_upload` method**

Replace the existing `handle_upload` method (and remove `process_upload_file_progress`) with:

```rust
    fn handle_upload(&self, event: &UploadEvent) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        match event {
            UploadEvent::Start {
                total_files,
                total_bytes: _,
            } => {
                state.upload_total_files = *total_files;
                if *total_files > 1 {
                    let bar = self.multi.add(ProgressBar::new(*total_files as u64));
                    bar.set_style(files_style());
                    bar.set_message(format!("Upload {} files", total_files));
                    state.files_bar = Some(bar);
                }
            },
            UploadEvent::Progress {
                phase,
                bytes_completed,
                total_bytes,
                bytes_per_sec,
                transfer_bytes_completed,
                transfer_bytes,
                transfer_bytes_per_sec,
                files,
            } => {
                if state.last_upload_phase.as_ref() != Some(phase) {
                    if let Some(ref spinner) = state.spinner {
                        spinner.finish_and_clear();
                        self.multi.remove(spinner);
                        state.spinner = None;
                    }
                    match phase {
                        UploadPhase::Preparing => {
                            let bar = self.multi.add(ProgressBar::new_spinner());
                            bar.set_style(spinner_style());
                            bar.set_message("Preparing files...");
                            bar.enable_steady_tick(std::time::Duration::from_millis(100));
                            state.spinner = Some(bar);
                        },
                        UploadPhase::CheckingUploadMode => {
                            let bar = self.multi.add(ProgressBar::new_spinner());
                            bar.set_style(spinner_style());
                            bar.set_message("Checking upload mode...");
                            bar.enable_steady_tick(std::time::Duration::from_millis(100));
                            state.spinner = Some(bar);
                        },
                        UploadPhase::Uploading => {
                            // Spinner already cleared above
                        },
                        UploadPhase::Committing => {
                            self.cleanup_upload_bars(&mut state);
                            let bar = self.multi.add(ProgressBar::new_spinner());
                            bar.set_style(spinner_style());
                            bar.set_message("Creating commit...");
                            bar.enable_steady_tick(std::time::Duration::from_millis(100));
                            state.spinner = Some(bar);
                        },
                    }
                    state.last_upload_phase = Some(phase.clone());
                }

                if *phase == UploadPhase::Uploading {
                    // Update or create the processing bar
                    let completed_count = state.upload_completed_files.len();
                    let total_count = state.upload_total_files;
                    if state.processing_bar.is_none() && *total_bytes > 0 {
                        let bar = self.multi.add(ProgressBar::new(*total_bytes));
                        bar.set_style(bytes_style());
                        state.processing_bar = Some(bar);
                    }
                    if let Some(ref bar) = state.processing_bar {
                        bar.set_length(*total_bytes);
                        bar.set_position(*bytes_completed);
                        bar.set_message(format!(
                            "Processing Files ({} / {})",
                            completed_count, total_count
                        ));
                    }

                    // Update or create the transfer bar
                    if state.transfer_bar.is_none() && *transfer_bytes > 0 {
                        let bar = self.multi.add(ProgressBar::new(*transfer_bytes));
                        bar.set_style(bytes_style());
                        bar.set_message("New Data Upload");
                        state.transfer_bar = Some(bar);
                    }
                    if let Some(ref bar) = state.transfer_bar {
                        bar.set_length(*transfer_bytes);
                        bar.set_position(*transfer_bytes_completed);
                    }

                    // Update per-file progress
                    for fp in files {
                        state.upload_known_files.insert(fp.filename.clone());

                        if fp.bytes_completed == 0 {
                            continue;
                        }

                        if fp.status == FileStatus::Complete {
                            state.upload_completed_files.insert(fp.filename.clone());
                        }

                        state
                            .upload_active_files
                            .insert(fp.filename.clone(), fp.clone());
                    }

                    // Evict completed files from active map when we need room
                    if state.upload_active_files.len() > MAX_VISIBLE_UPLOAD_BARS {
                        let completed: Vec<String> = state
                            .upload_active_files
                            .keys()
                            .filter(|k| state.upload_completed_files.contains(*k))
                            .cloned()
                            .collect();
                        for name in completed {
                            state.upload_active_files.swap_remove(&name);
                            if state.upload_active_files.len() <= MAX_VISIBLE_UPLOAD_BARS {
                                break;
                            }
                        }
                    }

                    // Render the fixed slot pool
                    self.render_upload_file_slots(&mut state);
                }

                // Suppress unused variable warnings for rate fields
                let _ = bytes_per_sec;
                let _ = transfer_bytes_per_sec;
            },
            UploadEvent::FileComplete { .. } => {
                // File completion is tracked via FileStatus::Complete in Progress events
            },
            UploadEvent::Complete => {
                self.cleanup_upload_bars(&mut state);
                if let Some(spinner) = state.spinner.take() {
                    spinner.finish_and_clear();
                    self.multi.remove(&spinner);
                }
                if let Some(bar) = state.files_bar.take() {
                    bar.finish_and_clear();
                    self.multi.remove(&bar);
                }
            },
        }
    }
```

- [ ] **Step 2: Add `render_upload_file_slots` method**

Add this method to `CliProgressHandler`:

```rust
    fn render_upload_file_slots(&self, state: &mut ProgressState) {
        let active_count = state.upload_active_files.len();
        let max_individual = if active_count > MAX_VISIBLE_UPLOAD_BARS {
            MAX_VISIBLE_UPLOAD_BARS - 1
        } else {
            active_count
        };

        // Ensure we have enough slots allocated
        while state.upload_file_slots.len() < MAX_VISIBLE_UPLOAD_BARS {
            state.upload_file_slots.push(None);
        }

        let mut bar_idx = 0;
        let mut overflow_bytes_completed: u64 = 0;
        let mut overflow_total_bytes: u64 = 0;
        let mut overflow_count: usize = 0;

        for (_name, fp) in &state.upload_active_files {
            if bar_idx < max_individual {
                // Individual file bar
                let slot = &mut state.upload_file_slots[bar_idx];
                if let Some(ref bar) = slot {
                    bar.set_message(truncate_filename(&fp.filename, 40));
                    bar.set_length(fp.total_bytes);
                    bar.set_position(fp.bytes_completed);
                } else {
                    let bar = self.multi.add(ProgressBar::new(fp.total_bytes));
                    bar.set_style(bytes_style());
                    bar.set_message(truncate_filename(&fp.filename, 40));
                    bar.set_position(fp.bytes_completed);
                    *slot = Some(bar);
                }
            } else {
                // Overflow: accumulate into last slot
                overflow_bytes_completed += fp.bytes_completed;
                overflow_total_bytes += fp.total_bytes;
                overflow_count += 1;
            }
            bar_idx += 1;
        }

        // Render the overflow slot if needed
        if overflow_count > 0 {
            let slot_idx = MAX_VISIBLE_UPLOAD_BARS - 1;
            let slot = &mut state.upload_file_slots[slot_idx];
            if let Some(ref bar) = slot {
                bar.set_message(format!("[+ {} files]", overflow_count));
                bar.set_length(overflow_total_bytes);
                bar.set_position(overflow_bytes_completed);
            } else {
                let bar = self.multi.add(ProgressBar::new(overflow_total_bytes));
                bar.set_style(bytes_style());
                bar.set_message(format!("[+ {} files]", overflow_count));
                bar.set_position(overflow_bytes_completed);
                *slot = Some(bar);
            }
        }

        // Clear any slots beyond what we currently need
        let needed_slots = if overflow_count > 0 {
            MAX_VISIBLE_UPLOAD_BARS
        } else {
            active_count
        };
        for i in needed_slots..state.upload_file_slots.len() {
            if let Some(bar) = state.upload_file_slots[i].take() {
                bar.finish_and_clear();
                self.multi.remove(&bar);
            }
        }
    }
```

- [ ] **Step 3: Add `cleanup_upload_bars` method**

Add this method to `CliProgressHandler`:

```rust
    fn cleanup_upload_bars(&self, state: &mut ProgressState) {
        for slot in &mut state.upload_file_slots {
            if let Some(bar) = slot.take() {
                bar.finish_and_clear();
                self.multi.remove(&bar);
            }
        }
        state.upload_active_files.clear();
        state.upload_known_files.clear();
        state.upload_completed_files.clear();
        if let Some(bar) = state.processing_bar.take() {
            bar.finish_and_clear();
            self.multi.remove(&bar);
        }
        if let Some(bar) = state.transfer_bar.take() {
            bar.finish_and_clear();
            self.multi.remove(&bar);
        }
    }
```

- [ ] **Step 4: Remove the old `process_upload_file_progress` method**

Delete the `process_upload_file_progress` method entirely (it's been replaced by `render_upload_file_slots`).

- [ ] **Step 5: Verify compilation and lint**

Run:
```bash
cargo clippy -p huggingface-hub --all-features -- -D warnings
```

Expected: No errors or warnings.

- [ ] **Step 6: Format**

Run:
```bash
cargo +nightly fmt
```

- [ ] **Step 7: Build the release binary**

Run:
```bash
cargo build -p huggingface-hub --release --features cli
```

- [ ] **Step 8: Commit**

```bash
git add huggingface_hub/src/bin/hfrs/progress.rs
git commit -m "feat: two summary bars (processing + transfer) and overflow [+ N files] aggregation"
```
