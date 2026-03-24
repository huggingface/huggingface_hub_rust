#![allow(dead_code)]

use std::fs::File;
use std::path::{Path, PathBuf};

use fs4::fs_std::FileExt;

use crate::types::RepoType;

pub(crate) struct CacheLock {
    _file: File,
}

pub(crate) async fn acquire_lock(
    cache_dir: &Path,
    repo_folder: &str,
    etag: &str,
) -> crate::error::Result<CacheLock> {
    let path = lock_path(cache_dir, repo_folder, etag);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let lock_path_clone = path.clone();
    let lock = tokio::time::timeout(
        std::time::Duration::from_secs(crate::constants::CACHE_LOCK_TIMEOUT_SECS),
        tokio::task::spawn_blocking(move || {
            let file = File::create(&lock_path_clone)?;
            file.lock_exclusive()?;
            Ok::<_, std::io::Error>(file)
        }),
    )
    .await
    .map_err(|_| crate::error::HfError::CacheLockTimeout { path: path.clone() })?
    .map_err(|e| crate::error::HfError::Other(format!("Lock task failed: {e}")))?
    .map_err(crate::error::HfError::Io)?;
    Ok(CacheLock { _file: lock })
}

pub(crate) async fn write_ref(
    cache_dir: &Path,
    repo_folder: &str,
    revision: &str,
    commit_hash: &str,
) -> crate::error::Result<()> {
    let path = ref_path(cache_dir, repo_folder, revision);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(&path, commit_hash).await?;
    Ok(())
}

pub(crate) async fn read_ref(
    cache_dir: &Path,
    repo_folder: &str,
    revision: &str,
) -> crate::error::Result<Option<String>> {
    let path = ref_path(cache_dir, repo_folder, revision);
    match tokio::fs::read_to_string(&path).await {
        Ok(content) => Ok(Some(content.trim().to_string())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub(crate) async fn create_pointer_symlink(
    cache_dir: &Path,
    repo_folder: &str,
    commit_hash: &str,
    filename: &str,
    etag: &str,
) -> crate::error::Result<()> {
    let pointer = snapshot_path(cache_dir, repo_folder, commit_hash, filename);
    if let Some(parent) = pointer.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let blob = blob_path(cache_dir, repo_folder, etag);
    let pointer_parent = pointer.parent().unwrap();
    let relative = pathdiff::diff_paths(&blob, pointer_parent).unwrap_or(blob);
    let _ = tokio::fs::remove_file(&pointer).await;

    #[cfg(not(windows))]
    {
        tokio::fs::symlink(&relative, &pointer).await?;
    }
    #[cfg(windows)]
    {
        tokio::fs::copy(&blob_path(cache_dir, repo_folder, etag), &pointer).await?;
    }
    Ok(())
}

pub(crate) fn is_commit_hash(revision: &str) -> bool {
    revision.len() == 40 && revision.chars().all(|c| c.is_ascii_hexdigit())
}

pub(crate) fn repo_folder_name(repo_id: &str, repo_type: Option<RepoType>) -> String {
    let type_str = match repo_type {
        None | Some(RepoType::Model) => "models",
        Some(RepoType::Dataset) => "datasets",
        Some(RepoType::Space) => "spaces",
    };
    let parts: Vec<&str> = std::iter::once(type_str)
        .chain(repo_id.split('/'))
        .collect();
    parts.join("--")
}

pub(crate) fn blob_path(cache_dir: &Path, repo_folder: &str, etag: &str) -> PathBuf {
    cache_dir.join(repo_folder).join("blobs").join(etag)
}

pub(crate) fn snapshot_path(
    cache_dir: &Path,
    repo_folder: &str,
    commit_hash: &str,
    filename: &str,
) -> PathBuf {
    cache_dir
        .join(repo_folder)
        .join("snapshots")
        .join(commit_hash)
        .join(filename)
}

pub(crate) fn ref_path(cache_dir: &Path, repo_folder: &str, revision: &str) -> PathBuf {
    cache_dir.join(repo_folder).join("refs").join(revision)
}

pub(crate) fn lock_path(cache_dir: &Path, repo_folder: &str, etag: &str) -> PathBuf {
    cache_dir
        .join(".locks")
        .join(repo_folder)
        .join(format!("{etag}.lock"))
}

pub(crate) fn no_exist_path(
    cache_dir: &Path,
    repo_folder: &str,
    commit_hash: &str,
    filename: &str,
) -> PathBuf {
    cache_dir
        .join(repo_folder)
        .join(".no_exist")
        .join(commit_hash)
        .join(filename)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repo_folder_name_model_with_org() {
        assert_eq!(
            repo_folder_name("google/bert-base-uncased", Some(RepoType::Model)),
            "models--google--bert-base-uncased"
        );
    }

    #[test]
    fn test_repo_folder_name_model_no_org() {
        assert_eq!(
            repo_folder_name("gpt2", Some(RepoType::Model)),
            "models--gpt2"
        );
    }

    #[test]
    fn test_repo_folder_name_model_none_type() {
        assert_eq!(repo_folder_name("gpt2", None), "models--gpt2");
    }

    #[test]
    fn test_repo_folder_name_dataset() {
        assert_eq!(
            repo_folder_name("rajpurkar/squad", Some(RepoType::Dataset)),
            "datasets--rajpurkar--squad"
        );
    }

    #[test]
    fn test_repo_folder_name_space() {
        assert_eq!(
            repo_folder_name("dalle-mini/dalle-mini", Some(RepoType::Space)),
            "spaces--dalle-mini--dalle-mini"
        );
    }

    #[test]
    fn test_blob_path() {
        let cache = Path::new("/home/user/.cache/huggingface/hub");
        assert_eq!(
            blob_path(cache, "models--gpt2", "abc123"),
            PathBuf::from("/home/user/.cache/huggingface/hub/models--gpt2/blobs/abc123")
        );
    }

    #[test]
    fn test_snapshot_path() {
        assert_eq!(
            snapshot_path(Path::new("/cache"), "models--gpt2", "aaa111", "config.json"),
            PathBuf::from("/cache/models--gpt2/snapshots/aaa111/config.json")
        );
    }

    #[test]
    fn test_snapshot_path_nested_file() {
        assert_eq!(
            snapshot_path(
                Path::new("/cache"),
                "models--gpt2",
                "aaa111",
                "subdir/model.bin"
            ),
            PathBuf::from("/cache/models--gpt2/snapshots/aaa111/subdir/model.bin")
        );
    }

    #[test]
    fn test_ref_path() {
        assert_eq!(
            ref_path(Path::new("/cache"), "models--gpt2", "main"),
            PathBuf::from("/cache/models--gpt2/refs/main")
        );
    }

    #[test]
    fn test_ref_path_pr() {
        assert_eq!(
            ref_path(Path::new("/cache"), "models--gpt2", "refs/pr/1"),
            PathBuf::from("/cache/models--gpt2/refs/refs/pr/1")
        );
    }

    #[test]
    fn test_lock_path() {
        assert_eq!(
            lock_path(Path::new("/cache"), "models--gpt2", "abc123"),
            PathBuf::from("/cache/.locks/models--gpt2/abc123.lock")
        );
    }

    #[test]
    fn test_no_exist_path() {
        assert_eq!(
            no_exist_path(
                Path::new("/cache"),
                "models--gpt2",
                "aaa111",
                "missing.json"
            ),
            PathBuf::from("/cache/models--gpt2/.no_exist/aaa111/missing.json")
        );
    }

    #[tokio::test]
    async fn test_write_and_read_ref() {
        let dir = tempfile::tempdir().unwrap();
        write_ref(
            dir.path(),
            "models--gpt2",
            "main",
            "abc123def456abc123def456abc123def456abcd",
        )
        .await
        .unwrap();
        let hash = read_ref(dir.path(), "models--gpt2", "main").await.unwrap();
        assert_eq!(
            hash,
            Some("abc123def456abc123def456abc123def456abcd".to_string())
        );
    }

    #[tokio::test]
    async fn test_read_ref_missing() {
        let dir = tempfile::tempdir().unwrap();
        let hash = read_ref(dir.path(), "models--gpt2", "main").await.unwrap();
        assert_eq!(hash, None);
    }

    #[cfg(not(windows))]
    #[tokio::test]
    async fn test_create_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path();
        let blob = blob_path(cache, "models--gpt2", "abc123");
        tokio::fs::create_dir_all(blob.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&blob, b"file content").await.unwrap();
        create_pointer_symlink(cache, "models--gpt2", "def456", "config.json", "abc123")
            .await
            .unwrap();
        let pointer = snapshot_path(cache, "models--gpt2", "def456", "config.json");
        assert!(pointer.exists());
        assert!(pointer.symlink_metadata().unwrap().file_type().is_symlink());
        let content = tokio::fs::read_to_string(&pointer).await.unwrap();
        assert_eq!(content, "file content");
    }

    #[cfg(not(windows))]
    #[tokio::test]
    async fn test_create_symlink_nested_file() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path();
        let blob = blob_path(cache, "models--gpt2", "abc123");
        tokio::fs::create_dir_all(blob.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&blob, b"nested content").await.unwrap();
        create_pointer_symlink(
            cache,
            "models--gpt2",
            "def456",
            "subdir/model.bin",
            "abc123",
        )
        .await
        .unwrap();
        let pointer = snapshot_path(cache, "models--gpt2", "def456", "subdir/model.bin");
        assert!(pointer.exists());
        assert!(pointer.symlink_metadata().unwrap().file_type().is_symlink());
        let target = std::fs::read_link(&pointer).unwrap();
        assert!(target.to_string_lossy().contains("blobs"));
        let content = tokio::fs::read_to_string(&pointer).await.unwrap();
        assert_eq!(content, "nested content");
    }

    #[test]
    fn test_is_commit_hash() {
        assert!(is_commit_hash("abc123def456abc123def456abc123def456abcd"));
        assert!(!is_commit_hash("main"));
        assert!(!is_commit_hash("abc123"));
        assert!(!is_commit_hash("xyz123def456abc123def456abc123def456abcd"));
    }

    #[tokio::test]
    async fn test_acquire_and_release_lock() {
        let dir = tempfile::tempdir().unwrap();
        let lock = acquire_lock(dir.path(), "models--gpt2", "abc123")
            .await
            .unwrap();
        let lock_file_path = lock_path(dir.path(), "models--gpt2", "abc123");
        assert!(lock_file_path.exists());
        drop(lock);
    }
}
