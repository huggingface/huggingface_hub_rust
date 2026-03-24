#![allow(dead_code)]

use std::path::{Path, PathBuf};

use crate::types::RepoType;

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
}
