//! Integration tests for the file system cache.
//!
//! Tests require HF_TOKEN and network access, skip if not set.
//! Interop tests additionally require python3, skip if not found.
//! Xet cache tests require --features xet.
//!
//! Run: HF_TOKEN=hf_xxx cargo test -p huggingface-hub --test cache_test
//! Run with xet: HF_TOKEN=hf_xxx cargo test -p huggingface-hub --features xet --test cache_test

use huggingface_hub::types::{DeleteCacheRevision, DownloadFileParams, SnapshotDownloadParams};
use huggingface_hub::{HfApi, HfApiBuilder};

fn api() -> Option<HfApi> {
    if std::env::var("HF_TOKEN").is_err() {
        return None;
    }
    Some(HfApiBuilder::new().build().expect("Failed to create HfApi"))
}

// =============================================================================
// Cache-mode download tests
// =============================================================================

#[tokio::test]
async fn test_download_file_to_cache() {
    let Some(_) = api() else { return };
    let cache_dir = tempfile::tempdir().unwrap();
    let api = HfApiBuilder::new()
        .cache_dir(cache_dir.path())
        .build()
        .unwrap();

    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("config.json")
        .build();
    let path = api.download_file(&params).await.unwrap();

    assert!(path.exists());
    let content = std::fs::read_to_string(&path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(json.get("model_type").is_some());
    assert!(path.to_string_lossy().contains("snapshots"));

    let repo_folder = std::fs::read_dir(cache_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.file_name().to_string_lossy().contains("gpt2"))
        .expect("repo folder not found");
    let blobs_dir = repo_folder.path().join("blobs");
    assert!(blobs_dir.exists());
    let blob_count = std::fs::read_dir(&blobs_dir).unwrap().count();
    assert_eq!(blob_count, 1);
}

#[tokio::test]
async fn test_download_file_cache_hit() {
    let Some(_) = api() else { return };
    let cache_dir = tempfile::tempdir().unwrap();
    let api = HfApiBuilder::new()
        .cache_dir(cache_dir.path())
        .build()
        .unwrap();

    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("config.json")
        .build();
    let path1 = api.download_file(&params).await.unwrap();
    let path2 = api.download_file(&params).await.unwrap();
    assert_eq!(path1, path2);
}

#[tokio::test]
async fn test_download_file_local_files_only_miss() {
    let cache_dir = tempfile::tempdir().unwrap();
    let api = HfApiBuilder::new()
        .cache_dir(cache_dir.path())
        .build()
        .unwrap();

    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("config.json")
        .local_files_only(true)
        .build();
    let result = api.download_file(&params).await;
    assert!(matches!(
        result,
        Err(huggingface_hub::HfError::LocalEntryNotFound { .. })
    ));
}

#[tokio::test]
async fn test_download_file_local_files_only_hit() {
    let Some(_) = api() else { return };
    let cache_dir = tempfile::tempdir().unwrap();
    let api = HfApiBuilder::new()
        .cache_dir(cache_dir.path())
        .build()
        .unwrap();

    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("config.json")
        .build();
    let path1 = api.download_file(&params).await.unwrap();

    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("config.json")
        .local_files_only(true)
        .build();
    let path2 = api.download_file(&params).await.unwrap();
    assert_eq!(path1, path2);
}

#[cfg(not(windows))]
#[tokio::test]
async fn test_download_file_cache_symlink_structure() {
    let Some(_) = api() else { return };
    let cache_dir = tempfile::tempdir().unwrap();
    let api = HfApiBuilder::new()
        .cache_dir(cache_dir.path())
        .build()
        .unwrap();

    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("config.json")
        .build();
    let path = api.download_file(&params).await.unwrap();

    let meta = std::fs::symlink_metadata(&path).unwrap();
    assert!(meta.file_type().is_symlink());
    let target = std::fs::read_link(&path).unwrap();
    assert!(target.to_string_lossy().contains("blobs"));
}

#[tokio::test]
async fn test_snapshot_download() {
    let Some(_) = api() else { return };
    let cache_dir = tempfile::tempdir().unwrap();
    let api = HfApiBuilder::new()
        .cache_dir(cache_dir.path())
        .build()
        .unwrap();

    let params = SnapshotDownloadParams::builder()
        .repo_id("gpt2")
        .allow_patterns(vec!["*.json".to_string()])
        .build();
    let snapshot_dir = api.snapshot_download(&params).await.unwrap();

    assert!(snapshot_dir.exists());
    assert!(snapshot_dir.to_string_lossy().contains("snapshots"));
    let config = snapshot_dir.join("config.json");
    assert!(config.exists());
}

#[tokio::test]
async fn test_scan_cache_after_download() {
    let Some(_) = api() else { return };
    let cache_dir = tempfile::tempdir().unwrap();
    let api = HfApiBuilder::new()
        .cache_dir(cache_dir.path())
        .build()
        .unwrap();

    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("config.json")
        .build();
    api.download_file(&params).await.unwrap();

    let info = api.scan_cache().await.unwrap();
    assert_eq!(info.repos.len(), 1);
    assert!(info.repos[0].repo_id.contains("gpt2"));
    assert_eq!(info.repos[0].revisions.len(), 1);
    assert!(!info.repos[0].revisions[0].files.is_empty());
    assert!(info.size_on_disk > 0);
}

#[tokio::test]
async fn test_delete_cache_revisions_integration() {
    let Some(_) = api() else { return };
    let cache_dir = tempfile::tempdir().unwrap();
    let api = HfApiBuilder::new()
        .cache_dir(cache_dir.path())
        .build()
        .unwrap();

    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("config.json")
        .build();
    api.download_file(&params).await.unwrap();

    let info = api.scan_cache().await.unwrap();
    let repo = &info.repos[0];
    let commit = repo.revisions[0].commit_hash.clone();

    api.delete_cache_revisions(&[DeleteCacheRevision {
        repo_id: repo.repo_id.clone(),
        repo_type: repo.repo_type,
        commit_hash: commit,
    }])
    .await
    .unwrap();

    let info = api.scan_cache().await.unwrap();
    if !info.repos.is_empty() {
        assert!(info.repos[0].revisions.is_empty());
    }
}

// =============================================================================
// Cross-library interoperability tests (Python huggingface_hub)
// =============================================================================

fn python_available() -> bool {
    std::process::Command::new("python3")
        .arg("--version")
        .output()
        .is_ok()
}

fn setup_python_venv(base_dir: &std::path::Path) -> Option<std::path::PathBuf> {
    if !python_available() {
        return None;
    }
    let venv_dir = base_dir.join("venv");
    let status = std::process::Command::new("python3")
        .args(["-m", "venv", &venv_dir.to_string_lossy()])
        .status()
        .ok()?;
    if !status.success() {
        return None;
    }

    let pip = venv_dir.join("bin").join("pip");
    let status = std::process::Command::new(&pip)
        .args(["install", "-q", "huggingface_hub"])
        .status()
        .ok()?;
    if !status.success() {
        return None;
    }

    Some(venv_dir)
}

fn python_bin(venv_dir: &std::path::Path) -> std::path::PathBuf {
    venv_dir.join("bin").join("python")
}

#[tokio::test]
async fn test_interop_python_downloads_first() {
    let Some(_) = api() else { return };
    let base_dir = tempfile::tempdir().unwrap();
    let cache_dir = base_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).unwrap();

    let Some(venv_dir) = setup_python_venv(base_dir.path()) else {
        return;
    };
    let python = python_bin(&venv_dir);
    let token = std::env::var("HF_TOKEN").unwrap();

    let script = format!(
        r#"
import os
os.environ["HF_HUB_CACHE"] = "{cache}"
os.environ["HF_TOKEN"] = "{token}"
from huggingface_hub import hf_hub_download
path = hf_hub_download("gpt2", "config.json")
print(path)
"#,
        cache = cache_dir.display(),
        token = token,
    );
    let output = std::process::Command::new(&python)
        .args(["-c", &script])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "Python failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let repo_folder = std::fs::read_dir(&cache_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.file_name().to_string_lossy().contains("gpt2"))
        .unwrap();
    let blob_count_before = std::fs::read_dir(repo_folder.path().join("blobs"))
        .unwrap()
        .count();

    let api = HfApiBuilder::new().cache_dir(&cache_dir).build().unwrap();
    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("config.json")
        .build();
    let path = api.download_file(&params).await.unwrap();
    assert!(path.exists());

    let blob_count_after = std::fs::read_dir(repo_folder.path().join("blobs"))
        .unwrap()
        .count();
    assert_eq!(blob_count_before, blob_count_after);
}

#[tokio::test]
async fn test_interop_rust_downloads_first() {
    let Some(_) = api() else { return };
    let base_dir = tempfile::tempdir().unwrap();
    let cache_dir = base_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).unwrap();

    let Some(venv_dir) = setup_python_venv(base_dir.path()) else {
        return;
    };
    let python = python_bin(&venv_dir);
    let token = std::env::var("HF_TOKEN").unwrap();

    let api = HfApiBuilder::new().cache_dir(&cache_dir).build().unwrap();
    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("config.json")
        .build();
    api.download_file(&params).await.unwrap();

    let script = format!(
        r#"
import os
os.environ["HF_HUB_CACHE"] = "{cache}"
os.environ["HF_TOKEN"] = "{token}"
from huggingface_hub import hf_hub_download
path = hf_hub_download("gpt2", "config.json", local_files_only=True)
print(path)
"#,
        cache = cache_dir.display(),
        token = token,
    );
    let output = std::process::Command::new(&python)
        .args(["-c", &script])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "Python local_files_only failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test]
async fn test_interop_mixed_partial_downloads() {
    let Some(_) = api() else { return };
    let base_dir = tempfile::tempdir().unwrap();
    let cache_dir = base_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).unwrap();

    let Some(venv_dir) = setup_python_venv(base_dir.path()) else {
        return;
    };
    let python = python_bin(&venv_dir);
    let token = std::env::var("HF_TOKEN").unwrap();

    let script = format!(
        r#"
import os
os.environ["HF_HUB_CACHE"] = "{cache}"
os.environ["HF_TOKEN"] = "{token}"
from huggingface_hub import hf_hub_download
hf_hub_download("gpt2", "README.md")
"#,
        cache = cache_dir.display(),
        token = token,
    );
    let output = std::process::Command::new(&python)
        .args(["-c", &script])
        .output()
        .unwrap();
    assert!(output.status.success());

    let api = HfApiBuilder::new().cache_dir(&cache_dir).build().unwrap();
    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("config.json")
        .build();
    api.download_file(&params).await.unwrap();

    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("README.md")
        .local_files_only(true)
        .build();
    let readme_path = api.download_file(&params).await.unwrap();
    assert!(readme_path.exists());

    let script = format!(
        r#"
import os
os.environ["HF_HUB_CACHE"] = "{cache}"
os.environ["HF_TOKEN"] = "{token}"
from huggingface_hub import hf_hub_download
path = hf_hub_download("gpt2", "config.json", local_files_only=True)
print(path)
"#,
        cache = cache_dir.display(),
        token = token,
    );
    let output = std::process::Command::new(&python)
        .args(["-c", &script])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "Python can't find Rust's file: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let info = api.scan_cache().await.unwrap();
    assert_eq!(info.repos.len(), 1);
    let total_files: usize = info.repos[0].revisions.iter().map(|r| r.files.len()).sum();
    assert!(total_files >= 2);
}

#[tokio::test]
async fn test_interop_python_snapshot_rust_snapshot() {
    let Some(_) = api() else { return };
    let base_dir = tempfile::tempdir().unwrap();
    let cache_dir = base_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).unwrap();

    let Some(venv_dir) = setup_python_venv(base_dir.path()) else {
        return;
    };
    let python = python_bin(&venv_dir);
    let token = std::env::var("HF_TOKEN").unwrap();

    let script = format!(
        r#"
import os
os.environ["HF_HUB_CACHE"] = "{cache}"
os.environ["HF_TOKEN"] = "{token}"
from huggingface_hub import snapshot_download
path = snapshot_download("gpt2", allow_patterns=["*.json"])
print(path)
"#,
        cache = cache_dir.display(),
        token = token,
    );
    let output = std::process::Command::new(&python)
        .args(["-c", &script])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "Python snapshot_download failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let repo_folder = std::fs::read_dir(&cache_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.file_name().to_string_lossy().contains("gpt2"))
        .unwrap();
    let blob_count_before = std::fs::read_dir(repo_folder.path().join("blobs"))
        .unwrap()
        .count();

    let api = HfApiBuilder::new().cache_dir(&cache_dir).build().unwrap();
    let params = SnapshotDownloadParams::builder()
        .repo_id("gpt2")
        .allow_patterns(vec!["*.json".to_string()])
        .build();
    let snapshot_dir = api.snapshot_download(&params).await.unwrap();
    assert!(snapshot_dir.exists());

    let blob_count_after = std::fs::read_dir(repo_folder.path().join("blobs"))
        .unwrap()
        .count();
    assert_eq!(blob_count_before, blob_count_after);
}

#[tokio::test]
async fn test_interop_rust_writes_python_validates_cache() {
    let Some(_) = api() else { return };
    let base_dir = tempfile::tempdir().unwrap();
    let cache_dir = base_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).unwrap();

    let Some(venv_dir) = setup_python_venv(base_dir.path()) else {
        return;
    };
    let python = python_bin(&venv_dir);
    let token = std::env::var("HF_TOKEN").unwrap();

    // Rust snapshot_download: multiple files into cache
    let api = HfApiBuilder::new().cache_dir(&cache_dir).build().unwrap();
    let params = SnapshotDownloadParams::builder()
        .repo_id("gpt2")
        .allow_patterns(vec!["*.json".to_string(), "*.md".to_string()])
        .build();
    let snapshot_dir = api.snapshot_download(&params).await.unwrap();
    assert!(snapshot_dir.exists());

    // Collect filenames Rust cached
    let mut rust_files: Vec<String> = Vec::new();
    let mut stack = vec![snapshot_dir.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let rel = path.strip_prefix(&snapshot_dir).unwrap();
                rust_files.push(rel.to_string_lossy().to_string());
            }
        }
    }
    assert!(
        rust_files.len() >= 2,
        "Expected at least 2 cached files, got: {rust_files:?}"
    );

    // Python validates the cache structure and reads every file
    let rust_files_json = serde_json::to_string(&rust_files).unwrap();
    let script = format!(
        r#"
import json
import os
import sys

os.environ["HF_HUB_CACHE"] = "{cache}"
os.environ["HF_TOKEN"] = "{token}"

from huggingface_hub import hf_hub_download, scan_cache_dir

# 1. scan_cache_dir must find the repo with correct structure
cache_info = scan_cache_dir("{cache}")
repos = [r for r in cache_info.repos if "gpt2" in r.repo_id]
assert len(repos) == 1, f"Expected 1 gpt2 repo, found {{len(repos)}}"
repo = repos[0]
assert len(repo.revisions) >= 1, f"Expected >=1 revision, found {{len(repo.revisions)}}"

revision = next(iter(repo.revisions))
snapshot_path = str(revision.snapshot_path)
cached_rel_paths = set()
for f in revision.files:
    rel = os.path.relpath(str(f.file_path), snapshot_path)
    cached_rel_paths.add(rel)
rust_files = set(json.loads('{rust_files_json}'))
assert rust_files.issubset(cached_rel_paths), (
    f"Rust files {{rust_files}} not all found in Python scan: {{cached_rel_paths}}"
)

# 2. Every file must be readable via hf_hub_download with local_files_only=True
for filename in rust_files:
    path = hf_hub_download("gpt2", filename, local_files_only=True)
    size = os.path.getsize(path)
    assert size > 0, f"File {{filename}} is empty at {{path}}"

# 3. Verify config.json content is valid JSON with expected field
config_path = hf_hub_download("gpt2", "config.json", local_files_only=True)
with open(config_path) as f:
    config = json.load(f)
assert "model_type" in config, f"config.json missing model_type: {{list(config.keys())}}"

print("ALL_CHECKS_PASSED")
"#,
        cache = cache_dir.display(),
        token = token,
        rust_files_json = rust_files_json,
    );
    let output = std::process::Command::new(&python)
        .args(["-c", &script])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "Python validation failed.\nstdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        stdout.contains("ALL_CHECKS_PASSED"),
        "Python did not complete all checks.\nstdout: {stdout}\nstderr: {stderr}"
    );
}

// =============================================================================
// Xet cache tests (require --features xet)
// =============================================================================

#[cfg(feature = "xet")]
#[tokio::test]
async fn test_xet_download_to_cache() {
    let Some(_) = api() else { return };
    let cache_dir = tempfile::tempdir().unwrap();
    let api = HfApiBuilder::new()
        .cache_dir(cache_dir.path())
        .build()
        .unwrap();

    let params = DownloadFileParams::builder()
        .repo_id("mcpotato/42-xet-test-repo")
        .filename("large_random.bin")
        .build();

    let path = match api.download_file(&params).await {
        Ok(p) => p,
        Err(e) => {
            let err_str = e.to_string();
            if err_str.contains("not found") || err_str.contains("Not Found") {
                eprintln!("Skipping xet cache test: repo not found");
                return;
            }
            panic!("Unexpected error: {err_str}");
        }
    };

    assert!(path.exists());
    assert!(path.to_string_lossy().contains("snapshots"));

    let file_size = std::fs::metadata(&path).unwrap().len();
    assert!(
        file_size > 1_000_000,
        "Expected large file, got {file_size} bytes"
    );

    // Blob should exist (LFS files use SHA-256 etag = 64 hex chars)
    let repo_folder = std::fs::read_dir(cache_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.file_name().to_string_lossy().contains("42-xet-test-repo"))
        .expect("repo folder not found");
    let blobs_dir = repo_folder.path().join("blobs");
    assert!(blobs_dir.exists());
    let blobs: Vec<_> = std::fs::read_dir(&blobs_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert!(!blobs.is_empty());

    // Symlink should point to blob
    #[cfg(not(windows))]
    {
        let meta = std::fs::symlink_metadata(&path).unwrap();
        assert!(meta.file_type().is_symlink());
        let target = std::fs::read_link(&path).unwrap();
        assert!(target.to_string_lossy().contains("blobs"));
    }

    // Ref should exist
    let refs_dir = repo_folder.path().join("refs");
    assert!(refs_dir.exists());
    let main_ref = refs_dir.join("main");
    assert!(main_ref.exists());

    // Second download should be a cache hit (same path returned)
    let path2 = api.download_file(&params).await.unwrap();
    assert_eq!(path, path2);

    // local_files_only should work
    let params_local = DownloadFileParams::builder()
        .repo_id("mcpotato/42-xet-test-repo")
        .filename("large_random.bin")
        .local_files_only(true)
        .build();
    let path3 = api.download_file(&params_local).await.unwrap();
    assert_eq!(path, path3);

    // scan_cache should report the file
    let info = api.scan_cache().await.unwrap();
    let repo = info
        .repos
        .iter()
        .find(|r| r.repo_id.contains("42-xet-test-repo"))
        .expect("repo not in scan_cache");
    assert!(!repo.revisions.is_empty());
    assert!(repo.size_on_disk >= file_size);
}
