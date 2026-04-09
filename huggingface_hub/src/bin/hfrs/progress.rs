use std::collections::HashMap;
use std::sync::Mutex;

use huggingface_hub::{DownloadEvent, FileStatus, ProgressEvent, ProgressHandler, UploadEvent, UploadPhase};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

pub struct CliProgressHandler {
    multi: MultiProgress,
    state: Mutex<ProgressState>,
}

struct ProgressState {
    files_bar: Option<ProgressBar>,
    bytes_bar: Option<ProgressBar>,
    file_bars: HashMap<String, ProgressBar>,
    last_upload_phase: Option<UploadPhase>,
    spinner: Option<ProgressBar>,
}

fn bytes_style() -> ProgressStyle {
    ProgressStyle::with_template(
        "{msg}: {percent}%|{wide_bar:.cyan/blue}| {bytes}/{total_bytes} [{elapsed}<{eta}, {bytes_per_sec}]",
    )
    .unwrap()
    .progress_chars("##-")
}

fn files_style() -> ProgressStyle {
    ProgressStyle::with_template("{msg}: {percent}%|{wide_bar:.green/blue}| {pos}/{len} [{elapsed}<{eta}]")
        .unwrap()
        .progress_chars("##-")
}

fn spinner_style() -> ProgressStyle {
    ProgressStyle::with_template("{spinner:.green} {msg}").unwrap()
}

fn truncate_filename(name: &str, max_len: usize) -> String {
    if name.len() <= max_len {
        return name.to_string();
    }
    let suffix = &name[name.len() - (max_len - 1)..];
    format!("…{suffix}")
}

impl CliProgressHandler {
    pub fn new() -> Self {
        Self {
            multi: MultiProgress::new(),
            state: Mutex::new(ProgressState {
                files_bar: None,
                bytes_bar: None,
                file_bars: HashMap::new(),
                last_upload_phase: None,
                spinner: None,
            }),
        }
    }

    fn handle_download(&self, event: &DownloadEvent) {
        let mut state = self.state.lock().unwrap();
        match event {
            DownloadEvent::Start {
                total_files,
                total_bytes,
            } => {
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
                            let bar = self.multi.add(ProgressBar::new(fp.total_bytes));
                            bar.set_style(bytes_style());
                            bar.set_message(truncate_filename(&fp.filename, 40));
                            state.file_bars.insert(fp.filename.clone(), bar);
                        },
                        FileStatus::InProgress => {
                            if let Some(bar) = state.file_bars.get(&fp.filename) {
                                bar.set_position(fp.bytes_completed);
                            } else if let Some(ref bar) = state.bytes_bar {
                                bar.set_position(fp.bytes_completed);
                            }
                        },
                        FileStatus::Complete => {
                            if let Some(bar) = state.file_bars.remove(&fp.filename) {
                                bar.finish_and_clear();
                                self.multi.remove(&bar);
                            }
                            if let Some(ref bar) = state.bytes_bar {
                                bar.set_position(fp.bytes_completed);
                            }
                            if let Some(ref bar) = state.files_bar {
                                bar.inc(1);
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
            },
        }
    }

    fn handle_upload(&self, event: &UploadEvent) {
        let mut state = self.state.lock().unwrap();
        match event {
            UploadEvent::Start {
                total_files,
                total_bytes,
            } => {
                if *total_files > 1 {
                    let bar = self.multi.add(ProgressBar::new(*total_files as u64));
                    bar.set_style(files_style());
                    bar.set_message(format!("Upload {} LFS files", total_files));
                    state.files_bar = Some(bar);
                }
                if *total_bytes > 0 {
                    let bar = self.multi.add(ProgressBar::new(*total_bytes));
                    bar.set_style(bytes_style());
                    bar.set_message("Uploading");
                    state.bytes_bar = Some(bar);
                }
            },
            UploadEvent::Progress {
                phase,
                bytes_completed,
                total_bytes,
                ..
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
                            if let Some(ref spinner) = state.spinner {
                                spinner.finish_and_clear();
                                self.multi.remove(spinner);
                                state.spinner = None;
                            }
                        },
                        UploadPhase::Committing => {
                            if let Some(ref bar) = state.bytes_bar {
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
                    }
                    state.last_upload_phase = Some(phase.clone());
                }

                if *phase == UploadPhase::Uploading
                    && let Some(ref bar) = state.bytes_bar
                {
                    bar.set_length(*total_bytes);
                    bar.set_position(*bytes_completed);
                }
            },
            UploadEvent::FileComplete { files, .. } => {
                if let Some(ref bar) = state.files_bar {
                    bar.inc(files.len() as u64);
                }
            },
            UploadEvent::Complete => {
                if let Some(ref spinner) = state.spinner {
                    spinner.finish_and_clear();
                }
                if let Some(ref bar) = state.files_bar {
                    bar.finish_and_clear();
                }
                if let Some(ref bar) = state.bytes_bar {
                    bar.finish_and_clear();
                }
            },
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

pub fn progress_disabled() -> bool {
    std::env::var("HF_HUB_DISABLE_PROGRESS_BARS").is_ok()
}
