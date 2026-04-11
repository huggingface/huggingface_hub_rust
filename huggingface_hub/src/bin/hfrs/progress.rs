use std::collections::{HashMap, HashSet, VecDeque};
use std::io::Write;
use std::sync::Mutex;

use huggingface_hub::{
    DownloadEvent, FileProgress, FileStatus, ProgressEvent, ProgressHandler, UploadEvent, UploadPhase,
};
use indexmap::IndexMap;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

/// Renders indicatif progress bars in the terminal for download and upload operations.
const MAX_VISIBLE_FILE_BARS: usize = 10;
const MAX_VISIBLE_UPLOAD_BARS: usize = 10;

pub struct CliProgressHandler {
    multi: MultiProgress,
    state: Mutex<ProgressState>,
}

struct ProgressState {
    // Download state
    files_bar: Option<ProgressBar>,
    bytes_bar: Option<ProgressBar>,
    file_bars: HashMap<String, ProgressBar>,
    download_queue: VecDeque<(String, u64)>,
    total_files: usize,
    // Upload state
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

fn bytes_style() -> ProgressStyle {
    ProgressStyle::with_template(
        "{msg}: {percent}%|{wide_bar:.cyan/blue}| {bytes}/{total_bytes} [{elapsed}<{eta}, {bytes_per_sec}]",
    )
    .expect("hardcoded template")
    .progress_chars("##-")
}

fn files_style() -> ProgressStyle {
    ProgressStyle::with_template("{msg}: {percent}%|{wide_bar:.green/blue}| {pos}/{len} [{elapsed}<{eta}]")
        .expect("hardcoded template")
        .progress_chars("##-")
}

fn spinner_style() -> ProgressStyle {
    ProgressStyle::with_template("{spinner:.green} {msg}").expect("hardcoded template")
}

fn truncate_filename(name: &str, max_len: usize) -> String {
    if name.len() <= max_len {
        return name.to_string();
    }
    let suffix = &name[name.len() - (max_len - 1)..];
    format!("…{suffix}")
}

impl CliProgressHandler {
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

    fn handle_download(&self, event: &DownloadEvent) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        match event {
            DownloadEvent::Start {
                total_files,
                total_bytes,
            } => {
                state.total_files = *total_files;
                if *total_files > 1 {
                    let bar = self.multi.add(ProgressBar::new(*total_files as u64));
                    bar.set_style(files_style());
                    bar.set_message(format!("Fetching {} files", total_files));
                    state.files_bar = Some(bar);
                }
                if *total_bytes > 0 && *total_files == 1 {
                    let bar = self.multi.add(ProgressBar::new(*total_bytes));
                    bar.set_style(bytes_style());
                    bar.set_message("Downloading");
                    state.bytes_bar = Some(bar);
                }
            },
            DownloadEvent::Progress { files } => {
                for fp in files {
                    match fp.status {
                        FileStatus::Started => {
                            if state.total_files == 1 && state.bytes_bar.is_none() && fp.total_bytes > 0 {
                                let bar = self.multi.add(ProgressBar::new(fp.total_bytes));
                                bar.set_style(bytes_style());
                                bar.set_message("Downloading");
                                state.bytes_bar = Some(bar);
                            } else if state.file_bars.len() < MAX_VISIBLE_FILE_BARS {
                                let bar = self.multi.add(ProgressBar::new(fp.total_bytes));
                                bar.set_style(bytes_style());
                                bar.set_message(truncate_filename(&fp.filename, 40));
                                state.file_bars.insert(fp.filename.clone(), bar);
                            } else {
                                state.download_queue.push_back((fp.filename.clone(), fp.total_bytes));
                            }
                        },
                        FileStatus::InProgress => {
                            if let Some(bar) = state.file_bars.get(&fp.filename) {
                                bar.set_position(fp.bytes_completed);
                            } else if state.file_bars.len() < MAX_VISIBLE_FILE_BARS {
                                let bar = self.multi.add(ProgressBar::new(fp.total_bytes));
                                bar.set_style(bytes_style());
                                bar.set_message(truncate_filename(&fp.filename, 40));
                                bar.set_position(fp.bytes_completed);
                                state.file_bars.insert(fp.filename.clone(), bar);
                                state.download_queue.retain(|(n, _)| n != &fp.filename);
                            } else if let Some(ref bar) = state.bytes_bar {
                                bar.set_position(fp.bytes_completed);
                            }
                        },
                        FileStatus::Complete => {
                            if let Some(bar) = state.file_bars.remove(&fp.filename) {
                                bar.finish_and_clear();
                                self.multi.remove(&bar);
                            }
                            state.download_queue.retain(|(n, _)| n != &fp.filename);
                            if let Some(ref bar) = state.bytes_bar {
                                bar.set_position(fp.bytes_completed);
                            }
                            if let Some(ref bar) = state.files_bar {
                                bar.inc(1);
                            }
                            while state.file_bars.len() < MAX_VISIBLE_FILE_BARS {
                                if let Some((name, total)) = state.download_queue.pop_front() {
                                    let bar = self.multi.add(ProgressBar::new(total));
                                    bar.set_style(bytes_style());
                                    bar.set_message(truncate_filename(&name, 40));
                                    state.file_bars.insert(name, bar);
                                } else {
                                    break;
                                }
                            }
                        },
                    }
                }
            },
            DownloadEvent::AggregateProgress {
                bytes_completed,
                total_bytes,
                ..
            } => {
                if state.bytes_bar.is_none() {
                    let bar = self.multi.add(ProgressBar::new(*total_bytes));
                    bar.set_style(bytes_style());
                    bar.set_message("Downloading");
                    state.bytes_bar = Some(bar);
                }
                if let Some(ref bar) = state.bytes_bar {
                    bar.set_length(*total_bytes);
                    bar.set_position(*bytes_completed);
                }
            },
            DownloadEvent::Complete => {
                if let Some(ref bar) = state.files_bar {
                    bar.finish_and_clear();
                }
                if let Some(ref bar) = state.bytes_bar {
                    bar.finish_and_clear();
                }
                for (_, bar) in state.file_bars.drain() {
                    bar.finish_and_clear();
                }
                state.download_queue.clear();
            },
        }
    }

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
                        UploadPhase::Uploading => {},
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
                        bar.set_message(format!("Processing Files ({} / {})", completed_count, total_count));
                    }

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

                    for fp in files {
                        state.upload_known_files.insert(fp.filename.clone());

                        if fp.bytes_completed == 0 {
                            continue;
                        }

                        if fp.status == FileStatus::Complete {
                            state.upload_completed_files.insert(fp.filename.clone());
                        }

                        state.upload_active_files.insert(fp.filename.clone(), fp.clone());
                    }

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

                    self.render_upload_file_slots(&mut state);
                }

                let _ = bytes_per_sec;
                let _ = transfer_bytes_per_sec;
            },
            UploadEvent::FileComplete { .. } => {},
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

    fn render_upload_file_slots(&self, state: &mut ProgressState) {
        let active_count = state.upload_active_files.len();
        let max_individual = if active_count > MAX_VISIBLE_UPLOAD_BARS {
            MAX_VISIBLE_UPLOAD_BARS - 1
        } else {
            active_count
        };

        while state.upload_file_slots.len() < MAX_VISIBLE_UPLOAD_BARS {
            state.upload_file_slots.push(None);
        }

        let mut overflow_bytes_completed: u64 = 0;
        let mut overflow_total_bytes: u64 = 0;
        let mut overflow_count: usize = 0;

        for (bar_idx, (_name, fp)) in state.upload_active_files.iter().enumerate() {
            if bar_idx < max_individual {
                let slot = &mut state.upload_file_slots[bar_idx];
                if let Some(bar) = slot.as_ref() {
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
                overflow_bytes_completed += fp.bytes_completed;
                overflow_total_bytes += fp.total_bytes;
                overflow_count += 1;
            }
        }

        if overflow_count > 0 {
            let slot_idx = MAX_VISIBLE_UPLOAD_BARS - 1;
            let slot = &mut state.upload_file_slots[slot_idx];
            if let Some(bar) = slot.as_ref() {
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
}

impl ProgressHandler for CliProgressHandler {
    fn on_progress(&self, event: &ProgressEvent) {
        match event {
            ProgressEvent::Download(dl) => self.handle_download(dl),
            ProgressEvent::Upload(ul) => self.handle_upload(ul),
        }
    }
}

pub fn progress_disabled_by_env() -> bool {
    std::env::var("HF_HUB_DISABLE_PROGRESS_BARS").is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

/// An `io::Write` adapter that routes output through `MultiProgress::println()`,
/// ensuring log lines appear above progress bars without visual corruption.
#[derive(Clone)]
pub struct MultiProgressWriter {
    multi: MultiProgress,
    buf: Vec<u8>,
}

impl MultiProgressWriter {
    pub fn new(multi: MultiProgress) -> Self {
        Self { multi, buf: Vec::new() }
    }
}

impl Write for MultiProgressWriter {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        self.buf.extend_from_slice(data);
        while let Some(pos) = self.buf.iter().position(|&b| b == b'\n') {
            let line = String::from_utf8_lossy(&self.buf[..pos]).into_owned();
            self.multi.println(&line).map_err(std::io::Error::other)?;
            self.buf.drain(..=pos);
        }
        Ok(data.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if !self.buf.is_empty() {
            let line = String::from_utf8_lossy(&self.buf).into_owned();
            self.multi.println(&line).map_err(std::io::Error::other)?;
            self.buf.clear();
        }
        Ok(())
    }
}
