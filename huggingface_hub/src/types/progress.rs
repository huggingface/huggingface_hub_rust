use std::sync::Arc;

/// Trait implemented by consumers to receive progress updates.
/// Implementations must be fast — avoid blocking I/O in on_progress().
pub trait ProgressHandler: Send + Sync {
    fn on_progress(&self, event: &ProgressEvent);
}

/// A clonable, optional handle to a progress handler.
pub type Progress = Option<Arc<dyn ProgressHandler>>;

#[derive(Debug, Clone)]
pub enum ProgressEvent {
    Upload(UploadEvent),
    Download(DownloadEvent),
}

/// Every variant that represents an in-progress state carries the current
/// `UploadPhase`, so consumers always know the phase from any single event
/// without tracking state across events.
#[derive(Debug, Clone)]
pub enum UploadEvent {
    Start {
        total_files: usize,
        total_bytes: u64,
    },
    Progress {
        phase: UploadPhase,
        bytes_completed: u64,
        total_bytes: u64,
        bytes_per_sec: Option<f64>,
    },
    /// Batched for efficiency during multi-file uploads (upload_folder).
    FileComplete {
        files: Vec<String>,
        phase: UploadPhase,
    },
    Complete,
}

#[derive(Debug, Clone)]
pub enum DownloadEvent {
    Start {
        total_files: usize,
        total_bytes: u64,
    },
    /// Only includes files whose state changed since the last event
    /// (delta, not full snapshot). Batched for efficiency during
    /// multi-file downloads (snapshot_download).
    Progress {
        files: Vec<FileProgress>,
    },
    /// Separate from per-file Progress because xet provides aggregate
    /// stats, not per-file byte counts.
    AggregateProgress {
        bytes_completed: u64,
        total_bytes: u64,
        bytes_per_sec: Option<f64>,
    },
    Complete,
}

#[derive(Debug, Clone)]
pub struct FileProgress {
    pub filename: String,
    pub bytes_completed: u64,
    pub total_bytes: u64,
    pub status: FileStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStatus {
    Started,
    InProgress,
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadPhase {
    Preparing,
    CheckingUploadMode,
    Uploading,
    Committing,
}

pub(crate) fn emit(handler: &Progress, event: ProgressEvent) {
    if let Some(h) = handler {
        h.on_progress(&event);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    struct RecordingHandler {
        events: Mutex<Vec<ProgressEvent>>,
    }

    impl RecordingHandler {
        fn new() -> Self {
            Self {
                events: Mutex::new(Vec::new()),
            }
        }

        fn events(&self) -> Vec<ProgressEvent> {
            self.events.lock().unwrap().clone()
        }
    }

    impl ProgressHandler for RecordingHandler {
        fn on_progress(&self, event: &ProgressEvent) {
            self.events.lock().unwrap().push(event.clone());
        }
    }

    #[test]
    fn handler_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Arc<RecordingHandler>>();
    }

    #[test]
    fn emit_with_none_is_noop() {
        let progress: Progress = None;
        emit(&progress, ProgressEvent::Download(DownloadEvent::Complete));
    }

    #[test]
    fn emit_records_events() {
        let handler = Arc::new(RecordingHandler::new());
        let progress: Progress = Some(handler.clone());

        emit(
            &progress,
            ProgressEvent::Upload(UploadEvent::Start {
                total_files: 2,
                total_bytes: 1024,
            }),
        );
        emit(
            &progress,
            ProgressEvent::Upload(UploadEvent::Progress {
                phase: UploadPhase::Uploading,
                bytes_completed: 512,
                total_bytes: 1024,
                bytes_per_sec: Some(100.0),
            }),
        );
        emit(&progress, ProgressEvent::Upload(UploadEvent::Complete));

        let events = handler.events();
        assert_eq!(events.len(), 3);
        assert!(matches!(events[0], ProgressEvent::Upload(UploadEvent::Start { .. })));
        assert!(matches!(events[1], ProgressEvent::Upload(UploadEvent::Progress { .. })));
        assert!(matches!(events[2], ProgressEvent::Upload(UploadEvent::Complete)));
    }

    #[test]
    fn download_file_lifecycle() {
        let handler = Arc::new(RecordingHandler::new());
        let progress: Progress = Some(handler.clone());

        emit(
            &progress,
            ProgressEvent::Download(DownloadEvent::Start {
                total_files: 1,
                total_bytes: 1000,
            }),
        );
        emit(
            &progress,
            ProgressEvent::Download(DownloadEvent::Progress {
                files: vec![FileProgress {
                    filename: "file.bin".to_string(),
                    bytes_completed: 0,
                    total_bytes: 1000,
                    status: FileStatus::Started,
                }],
            }),
        );
        emit(
            &progress,
            ProgressEvent::Download(DownloadEvent::Progress {
                files: vec![FileProgress {
                    filename: "file.bin".to_string(),
                    bytes_completed: 500,
                    total_bytes: 1000,
                    status: FileStatus::InProgress,
                }],
            }),
        );
        emit(
            &progress,
            ProgressEvent::Download(DownloadEvent::Progress {
                files: vec![FileProgress {
                    filename: "file.bin".to_string(),
                    bytes_completed: 1000,
                    total_bytes: 1000,
                    status: FileStatus::Complete,
                }],
            }),
        );
        emit(&progress, ProgressEvent::Download(DownloadEvent::Complete));

        let events = handler.events();
        assert_eq!(events.len(), 5);
    }

    #[test]
    fn upload_phase_progression() {
        let handler = Arc::new(RecordingHandler::new());
        let progress: Progress = Some(handler.clone());

        let phases = [
            UploadPhase::Preparing,
            UploadPhase::CheckingUploadMode,
            UploadPhase::Uploading,
            UploadPhase::Committing,
        ];

        for phase in &phases {
            emit(
                &progress,
                ProgressEvent::Upload(UploadEvent::Progress {
                    phase: phase.clone(),
                    bytes_completed: 0,
                    total_bytes: 100,
                    bytes_per_sec: None,
                }),
            );
        }

        let events = handler.events();
        assert_eq!(events.len(), 4);
        for (i, phase) in phases.iter().enumerate() {
            if let ProgressEvent::Upload(UploadEvent::Progress { phase: p, .. }) = &events[i] {
                assert_eq!(p, phase);
            } else {
                panic!("expected Upload(Progress) at index {i}");
            }
        }
    }

    #[test]
    fn batched_file_complete() {
        let handler = Arc::new(RecordingHandler::new());
        let progress: Progress = Some(handler.clone());

        emit(
            &progress,
            ProgressEvent::Upload(UploadEvent::FileComplete {
                files: vec!["a.bin".to_string(), "b.bin".to_string(), "c.bin".to_string()],
                phase: UploadPhase::Uploading,
            }),
        );

        let events = handler.events();
        assert_eq!(events.len(), 1);
        if let ProgressEvent::Upload(UploadEvent::FileComplete { files, .. }) = &events[0] {
            assert_eq!(files.len(), 3);
        } else {
            panic!("expected FileComplete");
        }
    }
}
