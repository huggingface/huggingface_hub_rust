use std::collections::HashMap;
use std::path::PathBuf;

use futures::stream::{Stream, StreamExt};
use url::Url;

use crate::client::HfApi;
use crate::constants;
use crate::error::{HfError, Result};
use crate::types::{
    AddSource, CommitInfo, CommitOperation, CreateCommitParams, DeleteFileParams, DeleteFolderParams,
    DownloadFileParams, GetPathsInfoParams, ListRepoFilesParams, ListRepoTreeParams, RepoTreeEntry, UploadFileParams,
    UploadFolderParams,
};

impl HfApi {
    /// List file paths in a repository (convenience wrapper over list_repo_tree).
    /// Returns all file paths recursively.
    pub async fn list_repo_files(&self, params: &ListRepoFilesParams) -> Result<Vec<String>> {
        let tree_params = ListRepoTreeParams::builder().repo_id(&params.repo_id).recursive(true).build();
        let tree_params = ListRepoTreeParams {
            revision: params.revision.clone(),
            repo_type: params.repo_type,
            ..tree_params
        };

        let stream = self.list_repo_tree(&tree_params);
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

    /// List files and directories in a repository tree.
    /// Endpoint: GET /api/{repo_type}s/{repo_id}/tree/{revision}
    pub fn list_repo_tree(&self, params: &ListRepoTreeParams) -> impl Stream<Item = Result<RepoTreeEntry>> + '_ {
        let revision = params.revision.as_deref().unwrap_or(constants::DEFAULT_REVISION);
        let url_str = format!("{}/tree/{}", self.api_url(params.repo_type, &params.repo_id), revision);
        let url = Url::parse(&url_str).unwrap();

        let mut query: Vec<(String, String)> = Vec::new();
        if params.recursive {
            query.push(("recursive".into(), "true".into()));
        }
        if params.expand {
            query.push(("expand".into(), "true".into()));
        }

        self.paginate(url, query)
    }

    /// Get info about specific paths in a repository.
    /// Endpoint: POST /api/{repo_type}s/{repo_id}/paths-info/{revision}
    pub async fn get_paths_info(&self, params: &GetPathsInfoParams) -> Result<Vec<RepoTreeEntry>> {
        let revision = params.revision.as_deref().unwrap_or(constants::DEFAULT_REVISION);
        let url = format!("{}/paths-info/{}", self.api_url(params.repo_type, &params.repo_id), revision);

        let body = serde_json::json!({
            "paths": params.paths,
        });

        let response = self
            .inner
            .client
            .post(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;

        let response = self
            .check_response(
                response,
                Some(&params.repo_id),
                crate::error::NotFoundContext::Entry {
                    path: params.paths.join(", "),
                },
            )
            .await?;
        Ok(response.json().await?)
    }
}

impl HfApi {
    /// Download a single file from a repository to a local directory.
    ///
    /// All repositories are xet-enabled. When the "xet" feature is active,
    /// downloads use the xet protocol. Otherwise, falls back to a standard
    /// HTTP GET and streams the response to disk.
    ///
    /// Endpoint: GET {endpoint}/{prefix}{repo_id}/resolve/{revision}/{filename}
    pub async fn download_file(&self, params: &DownloadFileParams) -> Result<PathBuf> {
        let revision = params.revision.as_deref().unwrap_or(constants::DEFAULT_REVISION);
        let url = self.download_url(params.repo_type, &params.repo_id, revision, &params.filename);

        #[cfg(feature = "xet")]
        {
            let head_response = self.inner.client.head(&url).headers(self.auth_headers()).send().await?;

            let head_response = self
                .check_response(
                    head_response,
                    Some(&params.repo_id),
                    crate::error::NotFoundContext::Entry {
                        path: params.filename.clone(),
                    },
                )
                .await?;

            let has_xet_hash = head_response.headers().get(constants::HEADER_X_XET_HASH).is_some();

            if has_xet_hash {
                return crate::xet::xet_download(self, params, &head_response).await;
            }
        }

        let response = self.inner.client.get(&url).headers(self.auth_headers()).send().await?;
        let response = self
            .check_response(
                response,
                Some(&params.repo_id),
                crate::error::NotFoundContext::Entry {
                    path: params.filename.clone(),
                },
            )
            .await?;

        tokio::fs::create_dir_all(&params.local_dir).await?;

        let dest_path = params.local_dir.join(&params.filename);
        if let Some(parent) = dest_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let mut file = tokio::fs::File::create(&dest_path).await?;
        let mut stream = response.bytes_stream();
        use tokio::io::AsyncWriteExt;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
        }
        file.flush().await?;

        Ok(dest_path)
    }
}

impl HfApi {
    /// Create a commit with multiple operations.
    ///
    /// Files are checked against the Hub's preupload endpoint to determine
    /// upload mode. Files marked as "lfs" are uploaded via the xet protocol
    /// (requires the "xet" feature) and referenced by SHA256 OID in the commit.
    /// Files marked as "regular" are sent inline as base64.
    ///
    /// Returns `HfError::XetNotEnabled` if any files require LFS upload but
    /// the "xet" feature is not enabled.
    ///
    /// Endpoint: POST /api/{repo_type}s/{repo_id}/commit/{revision}
    pub async fn create_commit(&self, params: &CreateCommitParams) -> Result<CommitInfo> {
        let revision = params.revision.as_deref().unwrap_or(constants::DEFAULT_REVISION);
        let url = format!("{}/commit/{}", self.api_url(params.repo_type, &params.repo_id), revision);

        // Determine which files should be uploaded via xet (LFS) vs inline
        // (regular). Files uploaded via xet are referenced by their SHA256 OID
        // in the commit NDJSON.
        let lfs_uploaded: HashMap<String, (String, u64)> = self
            .preupload_and_upload_lfs_files(params, revision, params.progress_callback.as_ref())
            .await?;

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
            let path_in_repo = match op {
                CommitOperation::Add { path_in_repo, .. } => path_in_repo,
                CommitOperation::Delete { path_in_repo } => path_in_repo,
            };
            let is_lfs = lfs_uploaded.contains_key(path_in_repo);
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

            // Call progress callback for non-LFS files (LFS files already triggered callback during upload)
            if !is_lfs {
                if let Some(ref callback) = params.progress_callback {
                    callback(path_in_repo);
                }
            }
        }

        let body: Vec<u8> = ndjson_lines
            .into_iter()
            .flat_map(|mut line| {
                line.push(b'\n');
                line
            })
            .collect();

        let mut headers = self.auth_headers();
        headers.insert(reqwest::header::CONTENT_TYPE, "application/x-ndjson".parse().unwrap());

        let mut request = self.inner.client.post(&url).headers(headers).body(body);

        if params.create_pr == Some(true) {
            request = request.query(&[("create_pr", "1")]);
        }

        let response = request.send().await?;
        let response = self
            .check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Repo)
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
    pub async fn upload_file(&self, params: &UploadFileParams) -> Result<CommitInfo> {
        let commit_message = params
            .commit_message
            .clone()
            .unwrap_or_else(|| format!("Upload {}", params.path_in_repo));

        let commit_params = CreateCommitParams::builder()
            .repo_id(&params.repo_id)
            .operations(vec![CommitOperation::Add {
                path_in_repo: params.path_in_repo.clone(),
                source: params.source.clone(),
            }])
            .commit_message(commit_message)
            .build();

        let commit_params = CreateCommitParams {
            commit_description: params.commit_description.clone(),
            repo_type: params.repo_type,
            revision: params.revision.clone(),
            create_pr: params.create_pr,
            parent_commit: params.parent_commit.clone(),
            ..commit_params
        };

        self.create_commit(&commit_params).await
    }

    /// Upload a folder to a repository. Walks the directory and creates add operations.
    pub async fn upload_folder(&self, params: &UploadFolderParams) -> Result<CommitInfo> {
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
            let revision = params.revision.as_deref().unwrap_or(constants::DEFAULT_REVISION);
            let tree_params = ListRepoTreeParams::builder().repo_id(&params.repo_id).recursive(true).build();
            let tree_params = ListRepoTreeParams {
                revision: Some(revision.to_string()),
                repo_type: params.repo_type,
                ..tree_params
            };
            let stream = self.list_repo_tree(&tree_params);
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

        let commit_params = CreateCommitParams::builder()
            .repo_id(&params.repo_id)
            .operations(operations)
            .commit_message(commit_message)
            .build();

        let commit_params = CreateCommitParams {
            commit_description: params.commit_description.clone(),
            repo_type: params.repo_type,
            revision: params.revision.clone(),
            create_pr: params.create_pr,
            ..commit_params
        };

        self.create_commit(&commit_params).await
    }

    /// Delete a file from a repository. Convenience wrapper around create_commit.
    pub async fn delete_file(&self, params: &DeleteFileParams) -> Result<CommitInfo> {
        let commit_message = params
            .commit_message
            .clone()
            .unwrap_or_else(|| format!("Delete {}", params.path_in_repo));

        let commit_params = CreateCommitParams::builder()
            .repo_id(&params.repo_id)
            .operations(vec![CommitOperation::Delete {
                path_in_repo: params.path_in_repo.clone(),
            }])
            .commit_message(commit_message)
            .build();

        let commit_params = CreateCommitParams {
            repo_type: params.repo_type,
            revision: params.revision.clone(),
            create_pr: params.create_pr,
            ..commit_params
        };

        self.create_commit(&commit_params).await
    }

    /// Delete a folder from a repository. Lists files under the path and deletes them.
    pub async fn delete_folder(&self, params: &DeleteFolderParams) -> Result<CommitInfo> {
        let revision = params.revision.as_deref().unwrap_or(constants::DEFAULT_REVISION);

        let tree_params = ListRepoTreeParams::builder().repo_id(&params.repo_id).recursive(true).build();
        let tree_params = ListRepoTreeParams {
            revision: Some(revision.to_string()),
            repo_type: params.repo_type,
            ..tree_params
        };

        let stream = self.list_repo_tree(&tree_params);
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

        let commit_params = CreateCommitParams::builder()
            .repo_id(&params.repo_id)
            .operations(operations)
            .commit_message(commit_message)
            .build();

        let commit_params = CreateCommitParams {
            repo_type: params.repo_type,
            revision: Some(revision.to_string()),
            create_pr: params.create_pr,
            ..commit_params
        };

        self.create_commit(&commit_params).await
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

impl HfApi {
    /// Check upload modes for all files and upload LFS files via xet.
    ///
    /// Always calls the preupload endpoint to determine upload mode per file.
    /// If any files require LFS and the "xet" feature is not enabled, returns
    /// `HfError::XetNotEnabled`.
    ///
    /// Returns a map of path_in_repo -> (sha256_oid, size) for files that were
    /// uploaded via xet and should be referenced as lfsFile in the commit.
    async fn preupload_and_upload_lfs_files(
        &self,
        params: &CreateCommitParams,
        revision: &str,
        progress_callback: Option<&crate::types::CommitProgressCallback>,
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
                &params.repo_id,
                params.repo_type,
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
            let _ = (lfs_files, progress_callback);
            Err(HfError::XetNotEnabled)
        }

        #[cfg(feature = "xet")]
        self.upload_lfs_files_via_xet(params, revision, &lfs_files, progress_callback)
            .await
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

        let url = format!("{}/preupload/{}", self.api_url(repo_type, repo_id), revision);

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
            .inner
            .client
            .post(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;

        let response = self
            .check_response(response, Some(repo_id), crate::error::NotFoundContext::Repo)
            .await?;

        let preupload: PreuploadResponse = response.json().await?;

        Ok(preupload.files.into_iter().map(|f| (f.path, f.upload_mode)).collect())
    }
}

#[cfg(feature = "xet")]
impl HfApi {
    /// Compute SHA256, negotiate LFS batch transfer, and upload via xet.
    async fn upload_lfs_files_via_xet(
        &self,
        params: &CreateCommitParams,
        revision: &str,
        lfs_files: &[&(String, u64, Vec<u8>, &AddSource)],
        progress_callback: Option<&crate::types::CommitProgressCallback>,
    ) -> Result<HashMap<String, (String, u64)>> {
        // Step 4: Compute SHA256 for LFS files
        let mut lfs_with_sha: Vec<(String, u64, String, &AddSource)> = Vec::new();
        for (path, size, _, source) in lfs_files {
            let sha256_oid = sha256_of_source(source).await?;
            lfs_with_sha.push(((*path).clone(), *size, sha256_oid, source));
        }

        // Step 5: Call LFS batch endpoint to negotiate transfer method
        let objects: Vec<(&str, u64)> = lfs_with_sha.iter().map(|(_, size, oid, _)| (oid.as_str(), *size)).collect();

        let chosen_transfer = self
            .post_lfs_batch_info(&params.repo_id, params.repo_type, revision, &objects)
            .await?;

        // Step 6: If server chose xet, upload via xet
        if chosen_transfer.as_deref() != Some("xet") {
            return Ok(HashMap::new());
        }

        let xet_files: Vec<(String, AddSource)> = lfs_with_sha
            .iter()
            .map(|(path, _, _, source)| (path.clone(), (*source).clone()))
            .collect();

        crate::xet::xet_upload(self, &xet_files, &params.repo_id, params.repo_type, revision, progress_callback)
            .await?;

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
        let url = format!("{}/{}{}.git/info/lfs/objects/batch", self.inner.endpoint, prefix, repo_id);

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

        let mut headers = self.auth_headers();
        headers.insert(reqwest::header::ACCEPT, "application/vnd.git-lfs+json".parse().unwrap());
        headers.insert(reqwest::header::CONTENT_TYPE, "application/vnd.git-lfs+json".parse().unwrap());

        let response = self.inner.client.post(&url).headers(headers).json(&body).send().await?;

        let response = self
            .check_response(response, Some(repo_id), crate::error::NotFoundContext::Repo)
            .await?;

        let batch: LfsBatchResponse = response.json().await?;
        Ok(batch.transfer)
    }
}

#[cfg(feature = "xet")]
async fn sha256_of_source(source: &AddSource) -> Result<String> {
    use sha2::{Digest, Sha256};
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
            let relative = path.strip_prefix(root).map_err(|e| HfError::Other(e.to_string()))?;
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
    use globset::Glob;
    patterns
        .iter()
        .any(|p| Glob::new(p).ok().map(|g| g.compile_matcher().is_match(path)).unwrap_or(false))
}

sync_api! {
    impl HfApiSync {
        fn list_repo_files(&self, params: &ListRepoFilesParams) -> Result<Vec<String>>;
        fn get_paths_info(&self, params: &GetPathsInfoParams) -> Result<Vec<RepoTreeEntry>>;
        fn download_file(&self, params: &DownloadFileParams) -> Result<PathBuf>;
        fn create_commit(&self, params: &CreateCommitParams) -> Result<CommitInfo>;
        fn upload_file(&self, params: &UploadFileParams) -> Result<CommitInfo>;
        fn upload_folder(&self, params: &UploadFolderParams) -> Result<CommitInfo>;
        fn delete_file(&self, params: &DeleteFileParams) -> Result<CommitInfo>;
        fn delete_folder(&self, params: &DeleteFolderParams) -> Result<CommitInfo>;
    }
}

sync_api_stream! {
    impl HfApiSync {
        fn list_repo_tree(&self, params: &ListRepoTreeParams) -> RepoTreeEntry;
    }
}
