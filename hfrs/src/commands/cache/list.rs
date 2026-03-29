use anyhow::Result;
use clap::Args as ClapArgs;
use huggingface_hub::HfApi;
use serde_json::json;

use crate::cli::OutputFormat;
use crate::output::{CommandOutput, CommandResult};

fn format_bytes(bytes: u64) -> String {
    const GB: u64 = 1_073_741_824;
    const MB: u64 = 1_048_576;
    const KB: u64 = 1_024;
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

fn default_cache_dir() -> std::path::PathBuf {
    if let Ok(val) = std::env::var("HF_HOME") {
        return std::path::PathBuf::from(val).join("hub");
    }
    if let Ok(val) = std::env::var("HUGGINGFACE_HUB_CACHE") {
        return std::path::PathBuf::from(val);
    }
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".cache")
        .join("huggingface")
        .join("hub")
}

struct RepoCacheEntry {
    repo_id: String,
    repo_type: String,
    size_on_disk: u64,
    revision_count: usize,
    last_accessed: String,
}

async fn scan_cache(cache_dir: &std::path::Path) -> Result<Vec<RepoCacheEntry>> {
    let mut entries = Vec::new();
    let mut dir = match tokio::fs::read_dir(cache_dir).await {
        Ok(d) => d,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(entries),
        Err(e) => return Err(e.into()),
    };

    while let Some(entry) = dir.next_entry().await? {
        let folder = entry.file_name().to_string_lossy().to_string();
        let (repo_type, repo_id) = match parse_repo_folder(&folder) {
            Some(v) => v,
            None => continue,
        };

        let repo_path = entry.path();
        let snapshots_dir = repo_path.join("snapshots");
        let mut revision_count = 0usize;
        let mut total_size: u64 = 0;
        let mut last_accessed = String::new();

        if let Ok(mut snap_dir) = tokio::fs::read_dir(&snapshots_dir).await {
            while let Ok(Some(snap)) = snap_dir.next_entry().await {
                if snap.path().is_dir() {
                    revision_count += 1;
                    if let Ok(meta) = snap.metadata().await {
                        if let Ok(accessed) = meta.accessed() {
                            let secs = accessed.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
                            let ts = format_timestamp(secs);
                            if ts > last_accessed {
                                last_accessed = ts;
                            }
                        }
                    }
                    total_size += dir_size(&snap.path()).await;
                }
            }
        }

        entries.push(RepoCacheEntry {
            repo_id,
            repo_type,
            size_on_disk: total_size,
            revision_count,
            last_accessed,
        });
    }

    Ok(entries)
}

fn parse_repo_folder(folder: &str) -> Option<(String, String)> {
    if let Some(rest) = folder.strip_prefix("models--") {
        return Some(("model".to_string(), rest.replace("--", "/")));
    }
    if let Some(rest) = folder.strip_prefix("datasets--") {
        return Some(("dataset".to_string(), rest.replace("--", "/")));
    }
    if let Some(rest) = folder.strip_prefix("spaces--") {
        return Some(("space".to_string(), rest.replace("--", "/")));
    }
    None
}

fn format_timestamp(secs: u64) -> String {
    if secs == 0 {
        return String::new();
    }
    format!("{secs}")
}

async fn dir_size(path: &std::path::Path) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let mut rd = match tokio::fs::read_dir(&dir).await {
            Ok(d) => d,
            Err(_) => continue,
        };
        while let Ok(Some(e)) = rd.next_entry().await {
            let p = e.path();
            if p.is_symlink() {
                if let Ok(meta) = tokio::fs::metadata(&p).await {
                    total += meta.len();
                }
            } else if p.is_dir() {
                stack.push(p);
            } else if let Ok(meta) = e.metadata().await {
                total += meta.len();
            }
        }
    }
    total
}

/// List cached repositories and files
#[derive(ClapArgs)]
pub struct Args {
    /// Output format
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,
}

pub async fn execute(_api: &HfApi, args: Args) -> Result<CommandResult> {
    let cache_dir = default_cache_dir();
    let repos = scan_cache(&cache_dir).await?;

    let headers = vec![
        "Repo".to_string(),
        "Type".to_string(),
        "Size".to_string(),
        "Revisions".to_string(),
        "Last Accessed".to_string(),
    ];

    let rows = repos
        .iter()
        .map(|r| {
            vec![
                r.repo_id.clone(),
                r.repo_type.clone(),
                format_bytes(r.size_on_disk),
                r.revision_count.to_string(),
                r.last_accessed.clone(),
            ]
        })
        .collect();

    let json_value: serde_json::Value = repos
        .iter()
        .map(|r| {
            json!({
                "repo_id": r.repo_id,
                "repo_type": r.repo_type,
                "size_on_disk": r.size_on_disk,
                "revision_count": r.revision_count,
                "last_accessed": r.last_accessed,
            })
        })
        .collect::<Vec<_>>()
        .into();

    let output = CommandOutput {
        headers,
        rows,
        json_value,
        quiet_values: vec![],
    };
    Ok(CommandResult::Formatted {
        output,
        format: args.format,
        quiet: false,
    })
}
