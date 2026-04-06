use std::collections::HashMap;
use std::path::{Path, PathBuf};

use futures::stream::{Stream, StreamExt};
use futures::TryStreamExt;
use globset::Glob;
use reqwest::header::IF_NONE_MATCH;
#[cfg(feature = "xet")]
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;
use url::Url;

use crate::error::{HFError, Result};
use crate::repository::{
    HFRepository, RepoCreateCommitParams, RepoDeleteFileParams, RepoDeleteFolderParams, RepoDownloadFileParams,
    RepoDownloadFileStreamParams, RepoDownloadFileToBytesParams, RepoGetPathsInfoParams, RepoListFilesParams,
    RepoListTreeParams, RepoSnapshotDownloadParams, RepoUploadFileParams, RepoUploadFolderParams,
};
use crate::types::{AddSource, CommitInfo, CommitOperation, RepoTreeEntry, RepoType};
use crate::{cache, constants};

impl HFRepository {
    /// Return a flat list of all file paths in the repository at the given revision.
    pub async fn list_files(&self, params: &RepoListFilesParams) -> Result<Vec<String>> {
        let revision = self.resolve_revision(params.revision.as_deref());
        let stream = self.list_tree(&crate::repository::RepoListTreeParams {
            revision,
            recursive: true,
            expand: false,
            limit: None,
        })?;
        futures::pin_mut!(stream);

        let mut files = Vec::new();
        while let Some(entry) = stream.next().await {
            let entry = entry?;
            if let RepoTreeEntry::File { path, .. } = entry {
                files.push(path);
            }
        }
        Ok(files)
    }

    /// Stream file and directory entries in the repository tree.
    ///
    /// Returns `Result<impl Stream<Item = Result<RepoTreeEntry>>>`. Set `recursive` to traverse
    /// subdirectories. Use `limit` to cap the total number of entries yielded.
    pub fn list_tree(&self, params: &RepoListTreeParams) -> Result<impl Stream<Item = Result<RepoTreeEntry>> + '_> {
        let revision = self.effective_revision(params.revision.as_deref());
        let url_str = format!("{}/tree/{}", self.client.api_url(Some(self.repo_type), &self.repo_path()), revision);
        let url = Url::parse(&url_str)?;

        let mut query: Vec<(String, String)> = Vec::new();
        if params.recursive {
            query.push(("recursive".into(), "true".into()));
        }
        if params.expand {
            query.push(("expand".into(), "true".into()));
        }

        Ok(self.client.paginate(url, query, params.limit))
    }

    /// Get info about specific paths in a repository.
    /// Endpoint: POST /api/{repo_type}s/{repo_id}/paths-info/{revision}
    pub async fn get_paths_info(&self, params: &RepoGetPathsInfoParams) -> Result<Vec<RepoTreeEntry>> {
        let revision = self.effective_revision(params.revision.as_deref());
        let url = format!("{}/paths-info/{}", self.client.api_url(Some(self.repo_type), &self.repo_path()), revision);

        let body = serde_json::json!({
            "paths": params.paths,
        });

        let response = self
            .client
            .inner
            .client
            .post(&url)
            .headers(self.client.auth_headers())
            .json(&body)
            .send()
            .await?;

        let repo_path = self.repo_path();
        let response = self
            .client
            .check_response(
                response,
                Some(&repo_path),
                crate::error::NotFoundContext::Entry {
                    path: params.paths.join(", "),
                },
            )
            .await?;
        Ok(response.json().await?)
    }
}

impl HFRepository {
    /// Download a single file from a repository.
    ///
    /// When `local_dir` is `Some`, the file is downloaded directly to that directory
    /// (no caching). When `local_dir` is `None`, the HF cache system is used:
    /// blobs are stored by etag and symlinked from snapshots/{commit}/{filename}.
    ///
    /// Endpoint: GET {endpoint}/{prefix}{repo_id}/resolve/{revision}/{filename}
    pub async fn download_file(&self, params: &RepoDownloadFileParams) -> Result<PathBuf> {
        if params.local_dir.is_some() {
            self.download_file_to_local_dir(params).await
        } else {
            if !self.client.inner.cache_enabled {
                return Err(HFError::CacheNotEnabled);
            }
            self.download_file_to_cache(params).await
        }
    }

    /// Download a file and return a byte stream instead of writing to disk.
    ///
    /// Returns a `(content_length, stream)` tuple. `content_length` is `Some`
    /// when the server provides a `Content-Length` header.
    ///
    /// When `range` is set, only the specified byte range is fetched. This works
    /// for both regular files (via HTTP Range header) and xet-backed files (via
    /// the xet streaming download API). For non-xet files the server must support
    /// range requests; the returned `content_length` reflects the range size.
    ///
    /// Endpoint: GET {endpoint}/{prefix}{repo_id}/resolve/{revision}/{filename}
    pub async fn download_file_stream(
        &self,
        params: &RepoDownloadFileStreamParams,
    ) -> Result<(Option<u64>, Box<dyn Stream<Item = std::result::Result<bytes::Bytes, HFError>> + Send + Unpin>)> {
        let revision = self.effective_revision(params.revision.as_deref());
        let repo_path = self.repo_path();
        let url = self
            .client
            .download_url(Some(self.repo_type), &repo_path, revision, &params.filename);

        #[cfg(feature = "xet")]
        {
            let head_response = self
                .client
                .inner
                .client
                .head(&url)
                .headers(self.client.auth_headers())
                .send()
                .await?;

            let head_response = self
                .client
                .check_response(
                    head_response,
                    Some(&repo_path),
                    crate::error::NotFoundContext::Entry {
                        path: params.filename.clone(),
                    },
                )
                .await?;

            let headers = head_response.headers();
            if let Some(xet_hash) = headers.get(constants::HEADER_X_XET_HASH).and_then(|v| v.to_str().ok()) {
                let file_size: u64 = headers
                    .get(reqwest::header::CONTENT_LENGTH)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0);

                let content_length = params.range.as_ref().map(|r| r.end.saturating_sub(r.start)).or(Some(file_size));

                let stream = crate::xet::xet_download_stream(
                    &self.client,
                    &repo_path,
                    Some(self.repo_type),
                    revision,
                    xet_hash,
                    file_size,
                    params.range.clone(),
                )
                .await?;

                return Ok((content_length, Box::new(Box::pin(stream))));
            }
        }

        let mut request = self.client.inner.client.get(&url).headers(self.client.auth_headers());

        if let Some(ref range) = params.range {
            request = request
                .header(reqwest::header::RANGE, format!("bytes={}-{}", range.start, range.end.saturating_sub(1)));
        }

        let response = request.send().await?;
        let response = self
            .client
            .check_response(
                response,
                Some(&repo_path),
                crate::error::NotFoundContext::Entry {
                    path: params.filename.clone(),
                },
            )
            .await?;

        let content_length = response.content_length();
        let stream = response.bytes_stream().map(|r| r.map_err(HFError::from));
        Ok((content_length, Box::new(Box::pin(stream))))
    }

    /// Download a file (or byte range) into memory and return the contents as [`bytes::Bytes`].
    ///
    /// This is a convenience wrapper around [`download_file_stream`](Self::download_file_stream)
    /// that collects the entire stream into a single buffer. When `range` is set,
    /// only the specified byte range is fetched.
    pub async fn download_file_to_bytes(&self, params: &RepoDownloadFileToBytesParams) -> Result<bytes::Bytes> {
        let (content_length, stream) = self.download_file_stream(params).await?;
        futures::pin_mut!(stream);

        let capacity = content_length.unwrap_or(0) as usize;
        let mut buf = bytes::BytesMut::with_capacity(capacity);
        while let Some(chunk) = stream.next().await {
            buf.extend_from_slice(&chunk?);
        }
        Ok(buf.freeze())
    }

    async fn download_file_to_local_dir(&self, params: &RepoDownloadFileParams) -> Result<PathBuf> {
        let revision = self.effective_revision(params.revision.as_deref());
        let repo_path = self.repo_path();
        let url = self
            .client
            .download_url(Some(self.repo_type), &repo_path, revision, &params.filename);

        #[cfg(feature = "xet")]
        {
            let head_response = self
                .client
                .inner
                .client
                .head(&url)
                .headers(self.client.auth_headers())
                .send()
                .await?;

            let head_response = self
                .client
                .check_response(
                    head_response,
                    Some(&repo_path),
                    crate::error::NotFoundContext::Entry {
                        path: params.filename.clone(),
                    },
                )
                .await?;

            let has_xet_hash = head_response.headers().get(constants::HEADER_X_XET_HASH).is_some();

            if has_xet_hash {
                let local_dir = params.local_dir.as_ref().unwrap();
                return crate::xet::xet_download_to_local_dir(
                    &self.client,
                    &repo_path,
                    Some(self.repo_type),
                    revision,
                    &params.filename,
                    local_dir,
                    &head_response,
                )
                .await;
            }
        }

        let response = self
            .client
            .inner
            .client
            .get(&url)
            .headers(self.client.auth_headers())
            .send()
            .await?;
        let response = self
            .client
            .check_response(
                response,
                Some(&repo_path),
                crate::error::NotFoundContext::Entry {
                    path: params.filename.clone(),
                },
            )
            .await?;

        let local_dir = params.local_dir.as_ref().unwrap();
        tokio::fs::create_dir_all(local_dir).await?;

        let dest_path = local_dir.join(&params.filename);
        if let Some(parent) = dest_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        stream_response_to_file(response, &dest_path).await?;

        Ok(dest_path)
    }

    /// Resolve a file from the local cache without making network requests.
    /// Matches Python's `try_to_load_from_cache`: checks the snapshot pointer
    /// first, then consults `.no_exist` markers for negative cache hits.
    fn resolve_from_cache_only(&self, repo_folder: &str, revision: &str, filename: &str) -> Result<PathBuf> {
        let cache_dir = &self.client.inner.cache_dir;

        let commit_hash = if cache::is_commit_hash(revision) {
            Some(revision.to_string())
        } else {
            let ref_path = cache::ref_path(cache_dir, repo_folder, revision);
            std::fs::read_to_string(&ref_path).ok().map(|s| s.trim().to_string())
        };

        if let Some(ref hash) = commit_hash {
            let snap = cache::snapshot_path(cache_dir, repo_folder, hash, filename);
            if snap.exists() {
                return Ok(snap);
            }
            if cache::no_exist_path(cache_dir, repo_folder, hash, filename).exists() {
                return Err(HFError::EntryNotFound {
                    path: filename.to_string(),
                    repo_id: String::new(),
                });
            }
        }

        Err(HFError::LocalEntryNotFound {
            path: filename.to_string(),
        })
    }

    /// Resolve the cached etag for a file by reading the symlink target in snapshots/.
    /// On Windows, where copies are used instead of symlinks, `read_link` will fail
    /// and this returns `None`, disabling conditional-request (If-None-Match) optimization.
    fn find_cached_etag(&self, repo_folder: &str, revision: &str, filename: &str) -> Option<String> {
        let cache_dir = &self.client.inner.cache_dir;

        let commit_hash = if cache::is_commit_hash(revision) {
            Some(revision.to_string())
        } else {
            let ref_path = cache::ref_path(cache_dir, repo_folder, revision);
            std::fs::read_to_string(&ref_path).ok().map(|s| s.trim().to_string())
        };

        let hash = commit_hash?;
        let snap = cache::snapshot_path(cache_dir, repo_folder, &hash, filename);
        let target = std::fs::read_link(&snap).ok()?;
        target.file_name()?.to_str().map(|s| s.to_string())
    }

    async fn download_file_to_cache(&self, params: &RepoDownloadFileParams) -> Result<PathBuf> {
        let revision = self.effective_revision(params.revision.as_deref());
        let cache_dir = &self.client.inner.cache_dir;
        let repo_folder = cache::repo_folder_name(&self.repo_path(), Some(self.repo_type));
        let force_download = params.force_download.unwrap_or(false);

        if cache::is_commit_hash(revision) && !force_download {
            let snap = cache::snapshot_path(cache_dir, &repo_folder, revision, &params.filename);
            if snap.exists() {
                return Ok(snap);
            }
        }

        if params.local_files_only.unwrap_or(false) {
            return self.resolve_from_cache_only(&repo_folder, revision, &params.filename);
        }

        let result = self
            .download_file_to_cache_network(params, revision, cache_dir, &repo_folder, force_download)
            .await;

        match &result {
            Err(e) if e.is_transient() && !force_download => self
                .resolve_from_cache_only(&repo_folder, revision, &params.filename)
                .or(result),
            _ => result,
        }
    }

    async fn download_file_to_cache_network(
        &self,
        params: &RepoDownloadFileParams,
        revision: &str,
        cache_dir: &Path,
        repo_folder: &str,
        force_download: bool,
    ) -> Result<PathBuf> {
        let repo_path = self.repo_path();
        let url = self
            .client
            .download_url(Some(self.repo_type), &repo_path, revision, &params.filename);

        let cached_etag = if !force_download {
            self.find_cached_etag(repo_folder, revision, &params.filename)
        } else {
            None
        };

        let mut head_headers = self.client.auth_headers();
        if let Some(ref etag_val) = cached_etag {
            if let Ok(hv) = reqwest::header::HeaderValue::from_str(&format!("\"{etag_val}\"")) {
                head_headers.insert(IF_NONE_MATCH, hv);
            }
        }

        let head_response = self
            .client
            .inner
            .no_redirect_client
            .head(&url)
            .headers(head_headers)
            .send()
            .await?;

        let status = head_response.status();

        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(mark_no_exist_and_return_error(
                cache_dir,
                repo_folder,
                revision,
                &head_response,
                &repo_path,
                &params.filename,
            )
            .await);
        }

        if status == reqwest::StatusCode::NOT_MODIFIED {
            let etag =
                cached_etag.ok_or_else(|| HFError::Other("Received 304 but no cached etag available".to_string()))?;
            let commit_hash = if cache::is_commit_hash(revision) {
                revision.to_string()
            } else {
                cache::read_ref(cache_dir, repo_folder, revision)
                    .await?
                    .ok_or_else(|| HFError::Other("Received 304 but no cached commit hash".to_string()))?
            };
            return finalize_cached_file(cache_dir, repo_folder, revision, &commit_hash, &params.filename, &etag).await;
        }

        let etag =
            extract_etag(&head_response).ok_or_else(|| HFError::Other("Missing ETag header in response".to_string()));
        let commit_hash = extract_commit_hash(&head_response);
        let has_xet_hash = head_response.headers().get(constants::HEADER_X_XET_HASH).is_some();
        #[cfg(feature = "xet")]
        let xet_hash = head_response
            .headers()
            .get(constants::HEADER_X_XET_HASH)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        #[cfg(feature = "xet")]
        let file_size: u64 = head_response
            .headers()
            .get(constants::HEADER_X_LINKED_SIZE)
            .or_else(|| head_response.headers().get(reqwest::header::CONTENT_LENGTH))
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        if !status.is_success() && !status.is_redirection() {
            self.client
                .check_response(
                    head_response,
                    Some(&repo_path),
                    crate::error::NotFoundContext::Entry {
                        path: params.filename.clone(),
                    },
                )
                .await?;
        }

        let etag = etag?;
        let commit_hash = commit_hash.ok_or_else(|| HFError::Other("Missing X-Repo-Commit header".to_string()))?;

        #[cfg(feature = "xet")]
        if has_xet_hash {
            let xet_hash = xet_hash.ok_or_else(|| HFError::Other("Missing X-Xet-Hash header".to_string()))?;
            let blob = cache::blob_path(cache_dir, repo_folder, &etag);
            if !blob.exists() || force_download {
                if let Some(parent) = blob.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
                let _lock = cache::acquire_lock(cache_dir, repo_folder, &etag).await?;

                crate::xet::xet_download_to_blob(
                    &self.client,
                    &repo_path,
                    Some(self.repo_type),
                    revision,
                    &xet_hash,
                    file_size,
                    &blob,
                )
                .await?;
            }

            return finalize_cached_file(cache_dir, repo_folder, revision, &commit_hash, &params.filename, &etag).await;
        }

        #[cfg(not(feature = "xet"))]
        if has_xet_hash {
            return Err(HFError::Other(
                "File requires xet transfer but the 'xet' feature is not enabled. \
                 Rebuild with --features xet to download this file."
                    .to_string(),
            ));
        }

        let blob = cache::blob_path(cache_dir, repo_folder, &etag);

        if blob.exists() && !force_download {
            return finalize_cached_file(cache_dir, repo_folder, revision, &commit_hash, &params.filename, &etag).await;
        }

        let _lock = cache::acquire_lock(cache_dir, repo_folder, &etag).await?;
        let incomplete_path = PathBuf::from(format!("{}.incomplete", blob.display()));
        if let Some(parent) = incomplete_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let response = self
            .client
            .inner
            .client
            .get(&url)
            .headers(self.client.auth_headers())
            .send()
            .await?;
        stream_response_to_file(response, &incomplete_path).await?;
        tokio::fs::rename(&incomplete_path, &blob).await?;

        finalize_cached_file(cache_dir, repo_folder, revision, &commit_hash, &params.filename, &etag).await
    }
}

impl HFRepository {
    async fn resolve_commit_hash(&self, revision: &str) -> Result<String> {
        if cache::is_commit_hash(revision) {
            return Ok(revision.to_string());
        }
        let repo_path = self.repo_path();
        let sha = match self.repo_type {
            RepoType::Dataset => self.dataset_info(Some(revision.to_string())).await?.sha,
            RepoType::Space => self.space_info(Some(revision.to_string())).await?.sha,
            _ => self.model_info(Some(revision.to_string())).await?.sha,
        };
        sha.ok_or_else(|| HFError::Other(format!("No commit hash returned for {}/{}", repo_path, revision)))
    }

    async fn list_filtered_files(
        &self,
        revision: &str,
        allow_patterns: Option<&Vec<String>>,
        ignore_patterns: Option<&Vec<String>>,
    ) -> Result<Vec<String>> {
        let stream = self.list_tree(&crate::repository::RepoListTreeParams {
            revision: Some(revision.to_string()),
            recursive: true,
            expand: false,
            limit: None,
        })?;
        futures::pin_mut!(stream);

        let mut filenames: Vec<String> = Vec::new();
        while let Some(entry) = stream.next().await {
            let entry = entry?;
            if let RepoTreeEntry::File { path, .. } = entry {
                filenames.push(path);
            }
        }

        if let Some(allow) = allow_patterns {
            filenames.retain(|f| matches_any_glob(allow, f));
        }
        if let Some(ignore) = ignore_patterns {
            filenames.retain(|f| !matches_any_glob(ignore, f));
        }

        Ok(filenames)
    }

    pub async fn snapshot_download(&self, params: &RepoSnapshotDownloadParams) -> Result<PathBuf> {
        if params.local_dir.is_none() && !self.client.inner.cache_enabled {
            return Err(HFError::CacheNotEnabled);
        }
        let revision = self.effective_revision(params.revision.as_deref());
        let max_workers = params.max_workers.unwrap_or(8);
        let repo_folder = crate::cache::repo_folder_name(&self.repo_path(), Some(self.repo_type));
        let cache_dir = &self.client.inner.cache_dir;

        if params.local_files_only == Some(true) {
            let commit_hash = if crate::cache::is_commit_hash(revision) {
                revision.to_string()
            } else {
                crate::cache::read_ref(cache_dir, &repo_folder, revision)
                    .await?
                    .ok_or_else(|| HFError::LocalEntryNotFound {
                        path: format!("{}/{}", repo_folder, revision),
                    })?
            };
            let snapshot_dir = cache_dir.join(&repo_folder).join("snapshots").join(&commit_hash);
            if snapshot_dir.exists() {
                return Ok(snapshot_dir);
            }
            return Err(HFError::LocalEntryNotFound {
                path: format!("{}/{}", repo_folder, commit_hash),
            });
        }

        let commit_hash = self.resolve_commit_hash(revision).await?;

        let mut filenames = self
            .list_filtered_files(&commit_hash, params.allow_patterns.as_ref(), params.ignore_patterns.as_ref())
            .await?;

        let force = params.force_download == Some(true);

        if !force && params.local_dir.is_none() {
            filenames.retain(|f| !crate::cache::snapshot_path(cache_dir, &repo_folder, &commit_hash, f).exists());
        }

        #[cfg(feature = "xet")]
        {
            struct FileMetadataInfo {
                filename: String,
                etag: String,
                commit_hash: String,
                xet_hash: Option<String>,
                file_size: u64,
            }

            let repo_path = self.repo_path();
            let commit_hash_ref = &commit_hash;
            let head_futs = filenames.iter().map(|filename| {
                let url = self
                    .client
                    .download_url(Some(self.repo_type), &repo_path, commit_hash_ref, filename);
                let client = &self.client.inner.no_redirect_client;
                let auth = self.client.auth_headers();
                let filename = filename.clone();
                let repo_folder_ref = &repo_folder;
                async move {
                    let resp = client.head(&url).headers(auth).send().await?;
                    // Per-file 404 resilience: write a .no_exist marker and skip
                    // the file rather than aborting the entire snapshot download.
                    // This matches the Python huggingface_hub library behavior.
                    // Alternative: since the file list comes from list_repo_tree
                    // on a pinned commit, a 404 here is unexpected and could be
                    // treated as an error instead.
                    if resp.status() == reqwest::StatusCode::NOT_FOUND {
                        if let Some(commit) = extract_commit_hash(&resp) {
                            let no_exist = cache::no_exist_path(cache_dir, repo_folder_ref, &commit, &filename);
                            if let Some(parent) = no_exist.parent() {
                                let _ = tokio::fs::create_dir_all(parent).await;
                            }
                            let _ = tokio::fs::write(&no_exist, b"").await;
                        }
                        return Ok::<_, HFError>(None);
                    } else if !resp.status().is_success() && !resp.status().is_redirection() {
                        return Err(HFError::Http {
                            status: resp.status(),
                            url,
                            body: String::new(),
                        });
                    }
                    let etag =
                        extract_etag(&resp).ok_or_else(|| HFError::Other(format!("Missing ETag for {filename}")))?;
                    let commit = extract_commit_hash(&resp).unwrap_or_else(|| commit_hash_ref.clone());
                    let xet_hash = resp
                        .headers()
                        .get(constants::HEADER_X_XET_HASH)
                        .and_then(|v| v.to_str().ok())
                        .map(|s| s.to_string());
                    let file_size: u64 = resp
                        .headers()
                        .get(constants::HEADER_X_LINKED_SIZE)
                        .or_else(|| resp.headers().get(reqwest::header::CONTENT_LENGTH))
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0);
                    Ok::<_, HFError>(Some(FileMetadataInfo {
                        filename,
                        etag,
                        commit_hash: commit,
                        xet_hash,
                        file_size,
                    }))
                }
            });

            let file_metas: Vec<FileMetadataInfo> = futures::stream::iter(head_futs)
                .buffer_unordered(max_workers)
                .try_collect::<Vec<Option<FileMetadataInfo>>>()
                .await?
                .into_iter()
                .flatten()
                .collect();

            let mut xet_metas = Vec::new();
            let mut non_xet_filenames = Vec::new();

            if let Some(ref local_dir) = params.local_dir {
                for meta in file_metas {
                    let dest = local_dir.join(&meta.filename);
                    if dest.exists() && !force {
                        continue;
                    }
                    if meta.xet_hash.is_some() {
                        xet_metas.push(meta);
                    } else {
                        non_xet_filenames.push(meta.filename);
                    }
                }

                let xet_batch_fut = async {
                    if xet_metas.is_empty() {
                        return Ok::<_, HFError>(());
                    }
                    let batch_files: Vec<crate::xet::XetBatchFile> = xet_metas
                        .iter()
                        .map(|m| crate::xet::XetBatchFile {
                            hash: m.xet_hash.as_ref().unwrap().clone(),
                            file_size: m.file_size,
                            path: local_dir.join(&m.filename),
                        })
                        .collect();
                    crate::xet::xet_download_batch(
                        &self.client,
                        &repo_path,
                        Some(self.repo_type),
                        &commit_hash,
                        &batch_files,
                    )
                    .await
                };

                let non_xet_dl_params = build_download_params(
                    &repo_path,
                    &non_xet_filenames,
                    Some(self.repo_type),
                    &commit_hash,
                    params.force_download,
                    Some(local_dir.clone()),
                );
                let non_xet_fut = async {
                    download_concurrently(self, &non_xet_dl_params, max_workers).await?;
                    Ok::<_, HFError>(())
                };

                tokio::try_join!(xet_batch_fut, non_xet_fut)?;
                return Ok(local_dir.clone());
            }

            // Cache mode
            for meta in file_metas {
                let blob = cache::blob_path(cache_dir, &repo_folder, &meta.etag);
                if blob.exists() && !force {
                    cache::create_pointer_symlink(
                        cache_dir,
                        &repo_folder,
                        &meta.commit_hash,
                        &meta.filename,
                        &meta.etag,
                    )
                    .await?;
                    continue;
                }
                if meta.xet_hash.is_some() {
                    xet_metas.push(meta);
                } else {
                    non_xet_filenames.push(meta.filename);
                }
            }

            let xet_batch_fut = async {
                if xet_metas.is_empty() {
                    return Ok::<_, HFError>(());
                }
                let mut locks = Vec::with_capacity(xet_metas.len());
                for m in &xet_metas {
                    locks.push(cache::acquire_lock(cache_dir, &repo_folder, &m.etag).await?);
                }
                let batch_files: Vec<crate::xet::XetBatchFile> = xet_metas
                    .iter()
                    .map(|m| crate::xet::XetBatchFile {
                        hash: m.xet_hash.as_ref().unwrap().clone(),
                        file_size: m.file_size,
                        path: cache::blob_path(cache_dir, &repo_folder, &m.etag),
                    })
                    .collect();
                crate::xet::xet_download_batch(
                    &self.client,
                    &repo_path,
                    Some(self.repo_type),
                    &commit_hash,
                    &batch_files,
                )
                .await?;
                for m in &xet_metas {
                    cache::create_pointer_symlink(cache_dir, &repo_folder, &m.commit_hash, &m.filename, &m.etag)
                        .await?;
                }
                drop(locks);
                Ok(())
            };

            let non_xet_dl_params = build_download_params(
                &repo_path,
                &non_xet_filenames,
                Some(self.repo_type),
                &commit_hash,
                params.force_download,
                None,
            );
            let non_xet_fut = async {
                download_concurrently(self, &non_xet_dl_params, max_workers).await?;
                Ok::<_, HFError>(())
            };

            tokio::try_join!(xet_batch_fut, non_xet_fut)?;
        }

        #[cfg(not(feature = "xet"))]
        {
            let repo_path = self.repo_path();
            if let Some(ref local_dir) = params.local_dir {
                let dl_params = build_download_params(
                    &repo_path,
                    &filenames,
                    Some(self.repo_type),
                    &commit_hash,
                    params.force_download,
                    Some(local_dir.clone()),
                );
                download_concurrently(self, &dl_params, max_workers).await?;
                return Ok(local_dir.clone());
            }

            let dl_params = build_download_params(
                &repo_path,
                &filenames,
                Some(self.repo_type),
                &commit_hash,
                params.force_download,
                None,
            );
            download_concurrently(self, &dl_params, max_workers).await?;
        }

        if !crate::cache::is_commit_hash(revision) {
            crate::cache::write_ref(cache_dir, &repo_folder, revision, &commit_hash).await?;
        }

        Ok(cache_dir.join(&repo_folder).join("snapshots").join(&commit_hash))
    }
}

fn extract_etag(response: &reqwest::Response) -> Option<String> {
    let headers = response.headers();
    let raw = headers
        .get(constants::HEADER_X_LINKED_ETAG)
        .or_else(|| headers.get(reqwest::header::ETAG))
        .and_then(|v| v.to_str().ok())?;
    let normalized = raw.strip_prefix("W/").unwrap_or(raw);
    Some(normalized.trim_matches('"').to_string())
}

fn extract_commit_hash(response: &reqwest::Response) -> Option<String> {
    response
        .headers()
        .get(constants::HEADER_X_REPO_COMMIT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

async fn mark_no_exist_and_return_error(
    cache_dir: &Path,
    repo_folder: &str,
    revision: &str,
    response: &reqwest::Response,
    repo_id: &str,
    filename: &str,
) -> HFError {
    if let Some(commit_hash) = extract_commit_hash(response) {
        let no_exist = cache::no_exist_path(cache_dir, repo_folder, &commit_hash, filename);
        if let Some(parent) = no_exist.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        let _ = tokio::fs::write(&no_exist, b"").await;
        if !cache::is_commit_hash(revision) {
            let _ = cache::write_ref(cache_dir, repo_folder, revision, &commit_hash).await;
        }
    }
    HFError::EntryNotFound {
        path: filename.to_string(),
        repo_id: repo_id.to_string(),
    }
}

async fn finalize_cached_file(
    cache_dir: &Path,
    repo_folder: &str,
    revision: &str,
    commit_hash: &str,
    filename: &str,
    etag: &str,
) -> Result<PathBuf> {
    if !cache::is_commit_hash(revision) {
        cache::write_ref(cache_dir, repo_folder, revision, commit_hash).await?;
    }
    cache::create_pointer_symlink(cache_dir, repo_folder, commit_hash, filename, etag).await?;
    Ok(cache::snapshot_path(cache_dir, repo_folder, commit_hash, filename))
}

fn build_download_params(
    _repo_id: &str,
    filenames: &[String],
    _repo_type: Option<RepoType>,
    commit_hash: &str,
    force_download: Option<bool>,
    local_dir: Option<PathBuf>,
) -> Vec<RepoDownloadFileParams> {
    filenames
        .iter()
        .map(|filename| RepoDownloadFileParams {
            filename: filename.clone(),
            local_dir: local_dir.clone(),
            revision: Some(commit_hash.to_string()),
            force_download,
            local_files_only: None,
        })
        .collect()
}

async fn download_concurrently(
    api: &HFRepository,
    params: &[RepoDownloadFileParams],
    max_workers: usize,
) -> Result<Vec<PathBuf>> {
    futures::stream::iter(params.iter().map(|p| api.download_file(p)))
        .buffer_unordered(max_workers)
        .try_collect()
        .await
}

async fn stream_response_to_file(response: reqwest::Response, dest: &std::path::Path) -> Result<()> {
    let mut file = tokio::fs::File::create(dest).await?;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
    }
    file.flush().await?;
    Ok(())
}

impl HFRepository {
    /// Create a commit with multiple operations.
    ///
    /// Files are checked against the Hub's preupload endpoint to determine
    /// upload mode. Files marked as "lfs" are uploaded via the xet protocol
    /// (requires the "xet" feature) and referenced by SHA256 OID in the commit.
    /// Files marked as "regular" are sent inline as base64.
    ///
    /// Returns `HFError::XetNotEnabled` if any files require LFS upload but
    /// the "xet" feature is not enabled.
    ///
    /// Endpoint: POST /api/{repo_type}s/{repo_id}/commit/{revision}
    pub async fn create_commit(&self, params: &RepoCreateCommitParams) -> Result<CommitInfo> {
        let revision = self.effective_revision(params.revision.as_deref());
        let url = format!("{}/commit/{}", self.client.api_url(Some(self.repo_type), &self.repo_path()), revision);

        // Determine which files should be uploaded via xet (LFS) vs inline
        // (regular). Files uploaded via xet are referenced by their SHA256 OID
        // in the commit NDJSON.
        let lfs_uploaded: HashMap<String, (String, u64)> =
            self.preupload_and_upload_lfs_files(params, revision).await?;

        let mut ndjson_lines: Vec<Vec<u8>> = Vec::new();

        let mut header_value = serde_json::json!({
            "summary": params.commit_message,
            "description": params.commit_description.as_deref().unwrap_or(""),
        });
        if let Some(ref parent) = params.parent_commit {
            header_value["parentCommit"] = serde_json::Value::String(parent.clone());
        }
        let header_line = serde_json::json!({"key": "header", "value": header_value});
        ndjson_lines.push(serde_json::to_vec(&header_line)?);

        for op in &params.operations {
            let line = match op {
                CommitOperation::Add { path_in_repo, source } => {
                    if let Some((oid, size)) = lfs_uploaded.get(path_in_repo) {
                        serde_json::json!({
                            "key": "lfsFile",
                            "value": {
                                "path": path_in_repo,
                                "algo": "sha256",
                                "oid": oid,
                                "size": size,
                            }
                        })
                    } else {
                        Self::inline_base64_entry(path_in_repo, source).await?
                    }
                },
                CommitOperation::Delete { path_in_repo } => {
                    serde_json::json!({
                        "key": "deletedFile",
                        "value": {"path": path_in_repo}
                    })
                },
            };
            ndjson_lines.push(serde_json::to_vec(&line)?);
        }

        let body: Vec<u8> = ndjson_lines
            .into_iter()
            .flat_map(|mut line| {
                line.push(b'\n');
                line
            })
            .collect();

        let mut headers = self.client.auth_headers();
        headers.insert(reqwest::header::CONTENT_TYPE, "application/x-ndjson".parse().unwrap());

        let mut request = self.client.inner.client.post(&url).headers(headers).body(body);

        if params.create_pr == Some(true) {
            request = request.query(&[("create_pr", "1")]);
        }

        let response = request.send().await?;
        let repo_path = self.repo_path();
        let response = self
            .client
            .check_response(response, Some(&repo_path), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(response.json().await?)
    }

    async fn inline_base64_entry(path_in_repo: &str, source: &AddSource) -> Result<serde_json::Value> {
        use base64::Engine;
        let content = match source {
            AddSource::File(path) => tokio::fs::read(path).await?,
            AddSource::Bytes(bytes) => bytes.clone(),
        };
        let b64 = base64::engine::general_purpose::STANDARD.encode(&content);
        Ok(serde_json::json!({
            "key": "file",
            "value": {
                "content": b64,
                "path": path_in_repo,
                "encoding": "base64",
            }
        }))
    }

    /// Upload a single file to a repository. Convenience wrapper around create_commit.
    pub async fn upload_file(&self, params: &RepoUploadFileParams) -> Result<CommitInfo> {
        let commit_message = params
            .commit_message
            .clone()
            .unwrap_or_else(|| format!("Upload {}", params.path_in_repo));

        self.create_commit(&RepoCreateCommitParams {
            operations: vec![CommitOperation::Add {
                path_in_repo: params.path_in_repo.clone(),
                source: params.source.clone(),
            }],
            commit_message,
            commit_description: params.commit_description.clone(),
            revision: self.resolve_revision(params.revision.as_deref()),
            create_pr: params.create_pr,
            parent_commit: params.parent_commit.clone(),
        })
        .await
    }

    /// Upload a folder to a repository. Walks the directory and creates add operations.
    pub async fn upload_folder(&self, params: &RepoUploadFolderParams) -> Result<CommitInfo> {
        let mut operations = Vec::new();

        let folder = &params.folder_path;
        let base_repo_path = params.path_in_repo.as_deref().unwrap_or("");

        collect_files_recursive(
            folder,
            folder,
            base_repo_path,
            &params.allow_patterns,
            &params.ignore_patterns,
            &mut operations,
        )
        .await?;

        if let Some(ref delete_patterns) = params.delete_patterns {
            let revision = self.effective_revision(params.revision.as_deref());
            let stream = self.list_tree(&crate::repository::RepoListTreeParams {
                revision: Some(revision.to_string()),
                recursive: true,
                expand: false,
                limit: None,
            })?;
            futures::pin_mut!(stream);
            while let Some(entry) = stream.next().await {
                let entry = entry?;
                if let RepoTreeEntry::File { path, .. } = entry {
                    if matches_any_glob(delete_patterns, &path) {
                        operations.push(CommitOperation::Delete { path_in_repo: path });
                    }
                }
            }
        }

        let commit_message = params.commit_message.clone().unwrap_or_else(|| "Upload folder".to_string());

        self.create_commit(&RepoCreateCommitParams {
            operations,
            commit_message,
            commit_description: params.commit_description.clone(),
            revision: self.resolve_revision(params.revision.as_deref()),
            create_pr: params.create_pr,
            parent_commit: None,
        })
        .await
    }

    /// Delete a file from a repository. Convenience wrapper around create_commit.
    pub async fn delete_file(&self, params: &RepoDeleteFileParams) -> Result<CommitInfo> {
        let commit_message = params
            .commit_message
            .clone()
            .unwrap_or_else(|| format!("Delete {}", params.path_in_repo));

        self.create_commit(&RepoCreateCommitParams {
            operations: vec![CommitOperation::Delete {
                path_in_repo: params.path_in_repo.clone(),
            }],
            commit_message,
            commit_description: None,
            revision: self.resolve_revision(params.revision.as_deref()),
            create_pr: params.create_pr,
            parent_commit: None,
        })
        .await
    }

    /// Delete a folder from a repository. Lists files under the path and deletes them.
    pub async fn delete_folder(&self, params: &RepoDeleteFolderParams) -> Result<CommitInfo> {
        let revision = self.effective_revision(params.revision.as_deref());

        let stream = self.list_tree(&crate::repository::RepoListTreeParams {
            revision: Some(revision.to_string()),
            recursive: true,
            expand: false,
            limit: None,
        })?;
        futures::pin_mut!(stream);

        let mut operations = Vec::new();
        let prefix = if params.path_in_repo.ends_with('/') {
            params.path_in_repo.clone()
        } else {
            format!("{}/", params.path_in_repo)
        };

        while let Some(entry) = stream.next().await {
            let entry = entry?;
            if let RepoTreeEntry::File { path, .. } = entry {
                if path.starts_with(&prefix) || path == params.path_in_repo {
                    operations.push(CommitOperation::Delete { path_in_repo: path });
                }
            }
        }

        let commit_message = params
            .commit_message
            .clone()
            .unwrap_or_else(|| format!("Delete {}", params.path_in_repo));

        self.create_commit(&RepoCreateCommitParams {
            operations,
            commit_message,
            commit_description: None,
            revision: Some(revision.to_string()),
            create_pr: params.create_pr,
            parent_commit: None,
        })
        .await
    }
}

// --- Preupload and LFS upload integration ---

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreuploadFileInfo {
    path: String,
    upload_mode: String,
}

#[derive(Debug, serde::Deserialize)]
struct PreuploadResponse {
    files: Vec<PreuploadFileInfo>,
}

#[cfg(feature = "xet")]
#[derive(Debug, serde::Deserialize)]
struct LfsBatchResponse {
    transfer: Option<String>,
}

impl HFRepository {
    /// Check upload modes for all files and upload LFS files via xet.
    ///
    /// Always calls the preupload endpoint to determine upload mode per file.
    /// If any files require LFS and the "xet" feature is not enabled, returns
    /// `HFError::XetNotEnabled`.
    ///
    /// Returns a map of path_in_repo -> (sha256_oid, size) for files that were
    /// uploaded via xet and should be referenced as lfsFile in the commit.
    async fn preupload_and_upload_lfs_files(
        &self,
        params: &RepoCreateCommitParams,
        revision: &str,
    ) -> Result<HashMap<String, (String, u64)>> {
        let add_ops: Vec<(&String, &AddSource)> = params
            .operations
            .iter()
            .filter_map(|op| match op {
                CommitOperation::Add { path_in_repo, source } => Some((path_in_repo, source)),
                _ => None,
            })
            .collect();

        if add_ops.is_empty() {
            return Ok(HashMap::new());
        }

        // Step 1: Gather file info (path, size, sample) for preupload check
        let mut file_infos: Vec<(String, u64, Vec<u8>, &AddSource)> = Vec::new();
        for (path_in_repo, source) in &add_ops {
            let (size, sample) = read_size_and_sample(source).await?;
            file_infos.push(((*path_in_repo).clone(), size, sample, source));
        }

        // Step 2: Call preupload endpoint to classify files as "lfs" or "regular"
        let upload_modes = self
            .fetch_upload_modes(
                &self.repo_path(),
                Some(self.repo_type),
                revision,
                &file_infos
                    .iter()
                    .map(|(path, size, sample, _)| (path.as_str(), *size, sample.as_slice()))
                    .collect::<Vec<_>>(),
            )
            .await?;

        // Step 3: Identify LFS files (empty files are always regular)
        let lfs_files: Vec<&(String, u64, Vec<u8>, &AddSource)> = file_infos
            .iter()
            .filter(|(path, size, _, _)| {
                *size > 0 && upload_modes.get(path.as_str()).map(|m| m == "lfs").unwrap_or(false)
            })
            .collect();

        if lfs_files.is_empty() {
            return Ok(HashMap::new());
        }

        // LFS files require xet upload — fail if the feature is not enabled
        #[cfg(not(feature = "xet"))]
        {
            let _ = lfs_files;
            Err(HFError::XetNotEnabled)
        }

        #[cfg(feature = "xet")]
        self.upload_lfs_files_via_xet(params, revision, &lfs_files).await
    }

    /// Call the Hub preupload endpoint to determine upload mode per file.
    /// Returns a map of path -> upload_mode ("lfs" or "regular").
    async fn fetch_upload_modes(
        &self,
        repo_id: &str,
        repo_type: Option<crate::types::RepoType>,
        revision: &str,
        files: &[(&str, u64, &[u8])],
    ) -> Result<HashMap<String, String>> {
        use base64::Engine;

        let url = format!("{}/preupload/{}", self.client.api_url(repo_type, repo_id), revision);

        let files_payload: Vec<serde_json::Value> = files
            .iter()
            .map(|(path, size, sample)| {
                serde_json::json!({
                    "path": path,
                    "size": size,
                    "sample": base64::engine::general_purpose::STANDARD.encode(sample),
                })
            })
            .collect();

        let body = serde_json::json!({ "files": files_payload });

        let response = self
            .client
            .inner
            .client
            .post(&url)
            .headers(self.client.auth_headers())
            .json(&body)
            .send()
            .await?;

        let response = self
            .client
            .check_response(response, Some(repo_id), crate::error::NotFoundContext::Repo)
            .await?;

        let preupload: PreuploadResponse = response.json().await?;

        Ok(preupload.files.into_iter().map(|f| (f.path, f.upload_mode)).collect())
    }
}

#[cfg(feature = "xet")]
impl HFRepository {
    /// Compute SHA256, negotiate LFS batch transfer, and upload via xet.
    async fn upload_lfs_files_via_xet(
        &self,
        _params: &RepoCreateCommitParams,
        revision: &str,
        lfs_files: &[&(String, u64, Vec<u8>, &AddSource)],
    ) -> Result<HashMap<String, (String, u64)>> {
        // Step 4: Compute SHA256 for LFS files
        let mut lfs_with_sha: Vec<(String, u64, String, &AddSource)> = Vec::new();
        for (path, size, _, source) in lfs_files {
            let sha256_oid = sha256_of_source(source).await?;
            lfs_with_sha.push(((*path).clone(), *size, sha256_oid, source));
        }

        // Step 5: Call LFS batch endpoint to negotiate transfer method
        let objects: Vec<(&str, u64)> = lfs_with_sha.iter().map(|(_, size, oid, _)| (oid.as_str(), *size)).collect();

        let repo_path = self.repo_path();
        let chosen_transfer = self
            .post_lfs_batch_info(&repo_path, Some(self.repo_type), revision, &objects)
            .await?;

        // Step 6: If server chose xet, upload via xet
        if chosen_transfer.as_deref() != Some("xet") {
            return Ok(HashMap::new());
        }

        let xet_files: Vec<(String, AddSource)> = lfs_with_sha
            .iter()
            .map(|(path, _, _, source)| (path.clone(), (*source).clone()))
            .collect();

        crate::xet::xet_upload(&self.client, &xet_files, &repo_path, Some(self.repo_type), revision).await?;

        let result: HashMap<String, (String, u64)> = lfs_with_sha
            .into_iter()
            .map(|(path, size, oid, _)| (path, (oid, size)))
            .collect();

        Ok(result)
    }

    /// Call the LFS batch endpoint to negotiate transfer method.
    /// Returns the chosen transfer (e.g. "xet", "basic", "multipart").
    async fn post_lfs_batch_info(
        &self,
        repo_id: &str,
        repo_type: Option<crate::types::RepoType>,
        revision: &str,
        objects: &[(&str, u64)],
    ) -> Result<Option<String>> {
        let prefix = constants::repo_type_url_prefix(repo_type);
        let url = format!("{}/{}{}.git/info/lfs/objects/batch", self.client.inner.endpoint, prefix, repo_id);

        let objects_payload: Vec<serde_json::Value> = objects
            .iter()
            .map(|(oid, size)| {
                serde_json::json!({
                    "oid": oid,
                    "size": size,
                })
            })
            .collect();

        let body = serde_json::json!({
            "operation": "upload",
            "transfers": ["basic", "multipart", "xet"],
            "objects": objects_payload,
            "hash_algo": "sha256",
            "ref": { "name": revision },
        });

        let mut headers = self.client.auth_headers();
        headers.insert(reqwest::header::ACCEPT, "application/vnd.git-lfs+json".parse().unwrap());
        headers.insert(reqwest::header::CONTENT_TYPE, "application/vnd.git-lfs+json".parse().unwrap());

        let response = self.client.inner.client.post(&url).headers(headers).json(&body).send().await?;

        let response = self
            .client
            .check_response(response, Some(repo_id), crate::error::NotFoundContext::Repo)
            .await?;

        let batch: LfsBatchResponse = response.json().await?;
        Ok(batch.transfer)
    }
}

#[cfg(feature = "xet")]
async fn sha256_of_source(source: &AddSource) -> Result<String> {
    match source {
        AddSource::Bytes(bytes) => {
            let hash = Sha256::digest(bytes);
            Ok(format!("{:x}", hash))
        },
        AddSource::File(path) => {
            use tokio::io::AsyncReadExt;
            let mut file = tokio::fs::File::open(path).await?;
            let mut hasher = Sha256::new();
            let mut buf = vec![0u8; 64 * 1024];
            loop {
                let n = file.read(&mut buf).await?;
                if n == 0 {
                    break;
                }
                hasher.update(&buf[..n]);
            }
            Ok(format!("{:x}", hasher.finalize()))
        },
    }
}

async fn read_size_and_sample(source: &AddSource) -> Result<(u64, Vec<u8>)> {
    match source {
        AddSource::Bytes(bytes) => {
            let size = bytes.len() as u64;
            let sample = bytes[..std::cmp::min(bytes.len(), 512)].to_vec();
            Ok((size, sample))
        },
        AddSource::File(path) => {
            use tokio::io::AsyncReadExt;
            let mut file = tokio::fs::File::open(path).await?;
            let metadata = file.metadata().await?;
            let size = metadata.len();
            let mut sample = vec![0u8; 512];
            let n = file.read(&mut sample).await?;
            sample.truncate(n);
            Ok((size, sample))
        },
    }
}

/// Recursively collect files from a directory into CommitOperation::Add entries.
/// Respects allow_patterns and ignore_patterns (glob-style).
async fn collect_files_recursive(
    root: &std::path::Path,
    current: &std::path::Path,
    base_repo_path: &str,
    allow_patterns: &Option<Vec<String>>,
    ignore_patterns: &Option<Vec<String>>,
    operations: &mut Vec<CommitOperation>,
) -> Result<()> {
    let mut entries = tokio::fs::read_dir(current).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let metadata = entry.metadata().await?;

        if metadata.is_dir() {
            Box::pin(collect_files_recursive(root, &path, base_repo_path, allow_patterns, ignore_patterns, operations))
                .await?;
        } else if metadata.is_file() {
            let relative = path.strip_prefix(root).map_err(|e| HFError::Other(e.to_string()))?;
            let relative_str = relative.to_string_lossy();

            if let Some(ref allow) = allow_patterns {
                if !matches_any_glob(allow, &relative_str) {
                    continue;
                }
            }
            if let Some(ref ignore) = ignore_patterns {
                if matches_any_glob(ignore, &relative_str) {
                    continue;
                }
            }

            let repo_path = if base_repo_path.is_empty() {
                relative_str.to_string()
            } else {
                format!("{}/{}", base_repo_path.trim_end_matches('/'), relative_str)
            };

            operations.push(CommitOperation::Add {
                path_in_repo: repo_path,
                source: AddSource::File(path),
            });
        }
    }

    Ok(())
}

/// Check if a path matches any of the given glob patterns using the `globset` crate.
fn matches_any_glob(patterns: &[String], path: &str) -> bool {
    patterns
        .iter()
        .any(|p| Glob::new(p).ok().map(|g| g.compile_matcher().is_match(path)).unwrap_or(false))
}

sync_api! {
    impl HFRepository -> HFRepositorySync {
        fn list_files(&self, params: &RepoListFilesParams) -> crate::error::Result<Vec<String>>;
        fn get_paths_info(&self, params: &RepoGetPathsInfoParams) -> crate::error::Result<Vec<RepoTreeEntry>>;
        fn download_file(&self, params: &RepoDownloadFileParams) -> crate::error::Result<std::path::PathBuf>;
        fn download_file_to_bytes(&self, params: &RepoDownloadFileToBytesParams) -> crate::error::Result<bytes::Bytes>;
        fn snapshot_download(&self, params: &RepoSnapshotDownloadParams) -> crate::error::Result<std::path::PathBuf>;
        fn create_commit(&self, params: &RepoCreateCommitParams) -> crate::error::Result<CommitInfo>;
        fn upload_file(&self, params: &RepoUploadFileParams) -> crate::error::Result<CommitInfo>;
        fn upload_folder(&self, params: &RepoUploadFolderParams) -> crate::error::Result<CommitInfo>;
        fn delete_file(&self, params: &RepoDeleteFileParams) -> crate::error::Result<CommitInfo>;
        fn delete_folder(&self, params: &RepoDeleteFolderParams) -> crate::error::Result<CommitInfo>;
    }
}

sync_api_stream! {
    impl HFRepository -> HFRepositorySync {
        fn list_tree(&self, params: &RepoListTreeParams) -> RepoTreeEntry;
    }
}
