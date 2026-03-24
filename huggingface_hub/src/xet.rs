//! Xet high-performance transfer support.
//!
//! This module is only compiled when the "xet" feature is enabled.
//! When xet headers are detected during download/upload but the feature
//! is not enabled, HfError::XetNotEnabled is returned at the call site.

use std::path::PathBuf;
use std::sync::Arc;

use serde::Deserialize;
use xet::xet_session::{FileMetadata, Sha256Policy, XetFileInfo, XetSessionBuilder};
use xet_client::cas_client::auth::{AuthError, TokenRefresher};

use crate::client::HfApi;
use crate::constants;
use crate::error::{HfError, Result};
use crate::types::{AddSource, DownloadFileParams, GetXetTokenParams, RepoType};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct XetTokenResponse {
    access_token: String,
    exp: u64,
    cas_url: String,
}

pub struct XetConnectionInfo {
    pub endpoint: String,
    pub access_token: String,
    pub expiration_unix_epoch: u64,
}

impl XetConnectionInfo {
    fn token_info(&self) -> (String, u64) {
        (self.access_token.clone(), self.expiration_unix_epoch)
    }
}

/// Implements token refresh by calling the Hub API's xet token endpoint.
struct HubTokenRefresher {
    api: HfApi,
    repo_id: String,
    repo_type: Option<RepoType>,
    revision: String,
    token_type: &'static str,
}

#[async_trait::async_trait]
impl TokenRefresher for HubTokenRefresher {
    async fn refresh(&self) -> std::result::Result<(String, u64), AuthError> {
        let conn = fetch_xet_connection_info(
            &self.api,
            self.token_type,
            &self.repo_id,
            self.repo_type,
            &self.revision,
        )
        .await
        .map_err(|e| AuthError::token_refresh_failure(e.to_string()))?;
        Ok(conn.token_info())
    }
}

/// Fetch xet connection info (read or write token) from the Hub API.
/// Endpoint: GET /api/{repo_type}s/{repo_id}/xet-{read|write}-token/{revision}
async fn fetch_xet_connection_info(
    api: &HfApi,
    token_type: &str,
    repo_id: &str,
    repo_type: Option<RepoType>,
    revision: &str,
) -> Result<XetConnectionInfo> {
    let segment = constants::repo_type_api_segment(repo_type);
    let url = format!(
        "{}/api/{}/{}/xet-{}-token/{}",
        api.inner.endpoint, segment, repo_id, token_type, revision
    );

    let response = api
        .inner
        .client
        .get(&url)
        .headers(api.auth_headers())
        .send()
        .await?;

    let response = api
        .check_response(response, Some(repo_id), crate::error::NotFoundContext::Repo)
        .await?;

    let token_resp: XetTokenResponse = response.json().await?;
    Ok(XetConnectionInfo {
        endpoint: token_resp.cas_url,
        access_token: token_resp.access_token,
        expiration_unix_epoch: token_resp.exp,
    })
}

/// Download a file using the xet protocol.
/// Extracts the file hash and size from the HEAD response headers,
/// fetches a read token, and uses xet-session's DownloadGroup.
pub(crate) async fn xet_download(
    api: &HfApi,
    params: &DownloadFileParams,
    head_response: &reqwest::Response,
) -> Result<PathBuf> {
    let headers = head_response.headers();

    let file_hash = headers
        .get(constants::HEADER_X_XET_HASH)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| HfError::Other("Missing X-Xet-Hash header".to_string()))?
        .to_string();

    let file_size: u64 = headers
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    let revision = params
        .revision
        .as_deref()
        .unwrap_or(constants::DEFAULT_REVISION);

    let session = api
        .get_or_init_xet_session("read", &params.repo_id, params.repo_type, revision)
        .await?;

    let local_dir = params
        .local_dir
        .as_ref()
        .ok_or_else(|| HfError::Other("xet_download requires local_dir".to_string()))?;
    tokio::fs::create_dir_all(local_dir).await?;
    let dest_path = local_dir.join(&params.filename);
    if let Some(parent) = dest_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let group = session
        .new_download_group()
        .await
        .map_err(|e| HfError::Other(format!("Xet download failed: {e}")))?;

    let file_info = XetFileInfo {
        hash: file_hash,
        file_size,
        sha256: None,
    };

    group
        .download_file_to_path(file_info, dest_path.clone())
        .await
        .map_err(|e| HfError::Other(format!("Xet download failed: {e}")))?;

    group
        .finish()
        .await
        .map_err(|e| HfError::Other(format!("Xet download failed: {e}")))?;

    Ok(dest_path)
}

pub(crate) async fn xet_download_to_blob(
    api: &HfApi,
    repo_id: &str,
    repo_type: Option<RepoType>,
    revision: &str,
    file_hash: &str,
    file_size: u64,
    blob_path: &std::path::Path,
) -> Result<()> {
    let session = api
        .get_or_init_xet_session("read", repo_id, repo_type, revision)
        .await?;

    if let Some(parent) = blob_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let incomplete_path = PathBuf::from(format!("{}.incomplete", blob_path.display()));

    let group = session
        .new_download_group()
        .await
        .map_err(|e| HfError::Other(format!("Xet download failed: {e}")))?;

    let file_info = XetFileInfo {
        hash: file_hash.to_string(),
        file_size,
        sha256: None,
    };

    group
        .download_file_to_path(file_info, incomplete_path.clone())
        .await
        .map_err(|e| HfError::Other(format!("Xet download failed: {e}")))?;

    group
        .finish()
        .await
        .map_err(|e| HfError::Other(format!("Xet download failed: {e}")))?;

    tokio::fs::rename(&incomplete_path, blob_path).await?;
    Ok(())
}

pub(crate) struct XetBatchFile {
    pub hash: String,
    pub file_size: u64,
    pub blob_path: PathBuf,
}

pub(crate) async fn xet_download_batch(
    api: &HfApi,
    repo_id: &str,
    repo_type: Option<RepoType>,
    revision: &str,
    files: &[XetBatchFile],
) -> Result<()> {
    if files.is_empty() {
        return Ok(());
    }

    let session = api
        .get_or_init_xet_session("read", repo_id, repo_type, revision)
        .await?;

    let group = session
        .new_download_group()
        .await
        .map_err(|e| HfError::Other(format!("Xet batch download failed: {e}")))?;

    let mut incomplete_paths = Vec::with_capacity(files.len());
    for file in files {
        if let Some(parent) = file.blob_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let incomplete = PathBuf::from(format!("{}.incomplete", file.blob_path.display()));

        let file_info = XetFileInfo {
            hash: file.hash.clone(),
            file_size: file.file_size,
            sha256: None,
        };

        group
            .download_file_to_path(file_info, incomplete.clone())
            .await
            .map_err(|e| HfError::Other(format!("Xet batch download failed: {e}")))?;

        incomplete_paths.push((incomplete, file.blob_path.clone()));
    }

    group
        .finish()
        .await
        .map_err(|e| HfError::Other(format!("Xet batch download failed: {e}")))?;

    for (incomplete, final_path) in &incomplete_paths {
        tokio::fs::rename(incomplete, final_path).await?;
    }

    Ok(())
}

/// Upload files using the xet protocol.
/// Fetches a write token and uses xet-session's UploadCommit.
/// Returns the XetFileInfo (hash + size) for each uploaded file.
pub(crate) async fn xet_upload(
    api: &HfApi,
    files: &[(String, AddSource)],
    repo_id: &str,
    repo_type: Option<RepoType>,
    revision: &str,
) -> Result<Vec<XetFileInfo>> {
    let session = api
        .get_or_init_xet_session("write", repo_id, repo_type, revision)
        .await?;

    let commit = session
        .new_upload_commit()
        .await
        .map_err(|e| HfError::Other(format!("Xet upload failed: {e}")))?;

    let mut task_ids_in_order = Vec::with_capacity(files.len());

    for (_path_in_repo, source) in files {
        let handle = match source {
            AddSource::File(path) => commit
                .upload_from_path(path.clone(), Sha256Policy::Compute)
                .await
                .map_err(|e| HfError::Other(format!("Xet upload failed: {e}")))?,
            AddSource::Bytes(bytes) => commit
                .upload_bytes(bytes.clone(), Sha256Policy::Compute, None)
                .await
                .map_err(|e| HfError::Other(format!("Xet upload failed: {e}")))?,
        };
        task_ids_in_order.push(handle.task_id);
    }

    let results = commit
        .commit()
        .await
        .map_err(|e| HfError::Other(format!("Xet upload failed: {e}")))?;

    let mut xet_file_infos = Vec::with_capacity(files.len());
    for task_id in &task_ids_in_order {
        let result = results
            .get(task_id)
            .ok_or_else(|| HfError::Other("Missing xet upload result for task".to_string()))?;
        let metadata: &FileMetadata = result
            .as_ref()
            .as_ref()
            .map_err(|e| HfError::Other(format!("Xet upload task failed: {e}")))?;
        xet_file_infos.push(XetFileInfo {
            hash: metadata.hash.clone(),
            file_size: metadata.file_size,
            sha256: metadata.sha256.clone(),
        });
    }

    Ok(xet_file_infos)
}

impl HfApi {
    /// Return the cached XetSession, creating it on first use.
    ///
    /// The session is built once per HfApi lifetime and reused for all
    /// subsequent xet operations. A token refresher is installed so the
    /// session can renew its CAS credentials when they expire.
    async fn get_or_init_xet_session(
        &self,
        token_type: &'static str,
        repo_id: &str,
        repo_type: Option<RepoType>,
        revision: &str,
    ) -> Result<xet::xet_session::XetSession> {
        {
            let guard = self
                .inner
                .xet_session
                .lock()
                .map_err(|e| HfError::Other(format!("xet session lock poisoned: {e}")))?;
            if let Some(session) = guard.as_ref() {
                return Ok(session.clone());
            }
        }

        let conn =
            fetch_xet_connection_info(self, token_type, repo_id, repo_type, revision).await?;

        let token_refresher: Arc<dyn TokenRefresher> = Arc::new(HubTokenRefresher {
            api: self.clone(),
            repo_id: repo_id.to_string(),
            repo_type,
            revision: revision.to_string(),
            token_type,
        });

        let (token, expiry) = conn.token_info();
        let session = XetSessionBuilder::new()
            .with_endpoint(conn.endpoint.clone())
            .with_token_info(token, expiry)
            .with_token_refresher(token_refresher)
            .build_async()
            .await
            .map_err(|e| HfError::Other(format!("Failed to build xet session: {e}")))?;

        let mut guard = self
            .inner
            .xet_session
            .lock()
            .map_err(|e| HfError::Other(format!("xet session lock poisoned: {e}")))?;
        if let Some(existing) = guard.as_ref() {
            Ok(existing.clone())
        } else {
            *guard = Some(session.clone());
            Ok(session)
        }
    }

    /// Fetch a Xet connection token (read or write) for a repository.
    /// Endpoint: GET /api/{repo_type}s/{repo_id}/xet-{read|write}-token/{revision}
    pub async fn get_xet_token(&self, params: &GetXetTokenParams) -> Result<XetConnectionInfo> {
        let revision = params
            .revision
            .as_deref()
            .unwrap_or(constants::DEFAULT_REVISION);
        fetch_xet_connection_info(
            self,
            params.token_type.as_str(),
            &params.repo_id,
            params.repo_type,
            revision,
        )
        .await
    }
}
