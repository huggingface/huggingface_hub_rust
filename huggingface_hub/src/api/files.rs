use crate::client::HfApi;
use crate::constants;
use crate::error::{HfError, Result};
use crate::types::{
    AddSource, CommitInfo, CommitOperation, CreateCommitParams, DeleteFileParams,
    DeleteFolderParams, DownloadFileParams, GetPathsInfoParams, ListRepoFilesParams,
    ListRepoTreeParams, RepoTreeEntry, UploadFileParams, UploadFolderParams,
};
use futures::stream::{Stream, StreamExt};
use std::path::PathBuf;
use url::Url;

impl HfApi {
    /// List file paths in a repository (convenience wrapper over list_repo_tree).
    /// Returns all file paths recursively.
    pub async fn list_repo_files(&self, params: &ListRepoFilesParams) -> Result<Vec<String>> {
        let tree_params = ListRepoTreeParams::builder()
            .repo_id(&params.repo_id)
            .recursive(true)
            .build();
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
    pub fn list_repo_tree(
        &self,
        params: &ListRepoTreeParams,
    ) -> impl Stream<Item = Result<RepoTreeEntry>> + '_ {
        let revision = params
            .revision
            .as_deref()
            .unwrap_or(constants::DEFAULT_REVISION);
        let url_str = format!(
            "{}/tree/{}",
            self.api_url(params.repo_type, &params.repo_id),
            revision
        );
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
        let revision = params
            .revision
            .as_deref()
            .unwrap_or(constants::DEFAULT_REVISION);
        let url = format!(
            "{}/paths-info/{}",
            self.api_url(params.repo_type, &params.repo_id),
            revision
        );

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
    /// Sends a HEAD request first to check for xet headers.
    /// If xet headers are present and the "xet" feature is not enabled,
    /// returns HfError::XetNotEnabled.
    /// Otherwise, streams the file content to `local_dir/filename`.
    ///
    /// Endpoint: GET {endpoint}/{prefix}{repo_id}/resolve/{revision}/{filename}
    pub async fn download_file(&self, params: &DownloadFileParams) -> Result<PathBuf> {
        let revision = params
            .revision
            .as_deref()
            .unwrap_or(constants::DEFAULT_REVISION);
        let url = self.download_url(
            params.repo_type,
            &params.repo_id,
            revision,
            &params.filename,
        );

        let head_response = self
            .inner
            .client
            .head(&url)
            .headers(self.auth_headers())
            .send()
            .await?;

        let head_response = self
            .check_response(
                head_response,
                Some(&params.repo_id),
                crate::error::NotFoundContext::Entry {
                    path: params.filename.clone(),
                },
            )
            .await?;

        if head_response
            .headers()
            .get(constants::HEADER_X_XET_HASH)
            .is_some()
        {
            #[cfg(feature = "xet")]
            {
                return crate::xet::xet_download(self, params, &head_response).await;
            }
            #[cfg(not(feature = "xet"))]
            {
                return Err(HfError::XetNotEnabled);
            }
        }

        let response = self
            .inner
            .client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await?;
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
    /// For add operations, this uploads files via multipart form to
    /// POST /api/{repo_type}s/{repo_id}/commit/{revision}
    ///
    /// **IMPLEMENTATION NOTE:** The multipart protocol below is based on the
    /// Python huggingface_hub library's implementation. The exact format
    /// (header JSON structure, part naming) MUST be validated against the
    /// live Hub API during integration testing. The Python library's
    /// `_commit_api.py` is the reference implementation. If the format
    /// doesn't match, refer to the Python source for the correct protocol.
    ///
    /// For xet-enabled repos, if the server negotiates xet transfer,
    /// the xet feature must be enabled or HfError::XetNotEnabled is returned.
    pub async fn create_commit(&self, params: &CreateCommitParams) -> Result<CommitInfo> {
        use reqwest::multipart;

        let revision = params
            .revision
            .as_deref()
            .unwrap_or(constants::DEFAULT_REVISION);
        let url = format!(
            "{}/commit/{}",
            self.api_url(params.repo_type, &params.repo_id),
            revision
        );

        let mut form = multipart::Form::new();

        let mut header = serde_json::json!({
            "summary": params.commit_message,
        });
        if let Some(ref desc) = params.commit_description {
            header["description"] = serde_json::Value::String(desc.clone());
        }
        if let Some(ref parent) = params.parent_commit {
            header["parentCommit"] = serde_json::Value::String(parent.clone());
        }

        let mut operations_json = Vec::new();

        for op in &params.operations {
            match op {
                CommitOperation::Add {
                    path_in_repo,
                    source,
                } => {
                    let content = match source {
                        AddSource::File(path) => tokio::fs::read(path).await?,
                        AddSource::Bytes(bytes) => bytes.clone(),
                    };

                    operations_json.push(serde_json::json!({
                        "key": "file",
                        "path": path_in_repo,
                    }));

                    let part = multipart::Part::bytes(content).file_name(path_in_repo.clone());
                    form = form.part(format!("file:{}", path_in_repo), part);
                }
                CommitOperation::Delete { path_in_repo } => {
                    operations_json.push(serde_json::json!({
                        "key": "deletedFile",
                        "path": path_in_repo,
                    }));
                }
            }
        }

        header["lfsFiles"] = serde_json::json!([]);
        header["files"] = serde_json::json!(operations_json);

        let header_part =
            multipart::Part::text(serde_json::to_string(&header)?).mime_str("application/json")?;
        form = form.part("header", header_part);

        let mut request = self
            .inner
            .client
            .post(&url)
            .headers(self.auth_headers())
            .multipart(form);

        if let Some(create_pr) = params.create_pr {
            if create_pr {
                request = request.query(&[("create_pr", "1")]);
            }
        }

        let response = request.send().await?;
        let response = self
            .check_response(
                response,
                Some(&params.repo_id),
                crate::error::NotFoundContext::Repo,
            )
            .await?;
        Ok(response.json().await?)
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
            let revision = params
                .revision
                .as_deref()
                .unwrap_or(constants::DEFAULT_REVISION);
            let tree_params = ListRepoTreeParams::builder()
                .repo_id(&params.repo_id)
                .recursive(true)
                .build();
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

        let commit_message = params
            .commit_message
            .clone()
            .unwrap_or_else(|| "Upload folder".to_string());

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
        let revision = params
            .revision
            .as_deref()
            .unwrap_or(constants::DEFAULT_REVISION);

        let tree_params = ListRepoTreeParams::builder()
            .repo_id(&params.repo_id)
            .recursive(true)
            .build();
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
            Box::pin(collect_files_recursive(
                root,
                &path,
                base_repo_path,
                allow_patterns,
                ignore_patterns,
                operations,
            ))
            .await?;
        } else if metadata.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(|e| HfError::Other(e.to_string()))?;
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
    patterns.iter().any(|p| {
        Glob::new(p)
            .ok()
            .map(|g| g.compile_matcher().is_match(path))
            .unwrap_or(false)
    })
}
