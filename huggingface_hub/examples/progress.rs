//! Download a file with progress tracking.
//!
//! Demonstrates implementing the `ProgressHandler` trait to receive
//! real-time progress callbacks during file transfers.
//!
//! Run: cargo run -p huggingface-hub --example progress

use std::sync::Arc;

use huggingface_hub::{DownloadEvent, FileStatus, HFClient, ProgressEvent, ProgressHandler, RepoDownloadFileParams};

struct PrintProgressHandler;

impl ProgressHandler for PrintProgressHandler {
    fn on_progress(&self, event: &ProgressEvent) {
        match event {
            ProgressEvent::Download(dl) => match dl {
                DownloadEvent::Start {
                    total_files,
                    total_bytes,
                } => {
                    println!("Starting download: {total_files} file(s), {total_bytes} bytes");
                },
                DownloadEvent::Progress { files } => {
                    for f in files {
                        let pct = if f.total_bytes > 0 {
                            f.bytes_completed * 100 / f.total_bytes
                        } else {
                            0
                        };
                        let status = match f.status {
                            FileStatus::Started => "started",
                            FileStatus::InProgress => "downloading",
                            FileStatus::Complete => "complete",
                        };
                        println!("  {}: {pct}% ({}/{}) [{status}]", f.filename, f.bytes_completed, f.total_bytes);
                    }
                },
                DownloadEvent::AggregateProgress {
                    bytes_completed,
                    total_bytes,
                    ..
                } => {
                    println!("  aggregate: {bytes_completed}/{total_bytes}");
                },
                DownloadEvent::Complete => {
                    println!("Download complete.");
                },
            },
            ProgressEvent::Upload(ul) => {
                println!("Upload event: {ul:?}");
            },
        }
    }
}

#[tokio::main]
async fn main() -> huggingface_hub::Result<()> {
    let api = HFClient::new()?;
    let model = api.model("openai-community", "gpt2");

    let tmp_dir = tempfile::tempdir().expect("failed to create tempdir");

    let path = model
        .download_file(
            &RepoDownloadFileParams::builder()
                .filename("config.json")
                .local_dir(tmp_dir.path().to_path_buf())
                .progress(Some(Arc::new(PrintProgressHandler)))
                .build(),
        )
        .await?;

    println!("File saved to: {}", path.display());
    Ok(())
}
