//! Xet high-performance transfer support.
//!
//! This module is only compiled when the "xet" feature is enabled.
//! When xet headers are detected during download/upload but the feature
//! is not enabled, HfError::XetNotEnabled is returned at the call site.

use std::path::PathBuf;

use serde::Deserialize;
use xet::xet_session::{Sha256Policy, XetFileInfo, XetSessionBuilder};

use crate::client::{CachedXetSession, HfApi};
use crate::constants;
use crate::error::{HfError, Result};
use crate::types::{AddSource, GetXetTokenParams, RepoType};

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

struct XetOperationConfig {
    session: xet::xet_session::XetSession,
    token: String,
    expiry: u64,
    refresh_url: String,
    refresh_headers: reqwest::header::HeaderMap,
}

fn xet_token_url(api: &HfApi, token_type: &str, repo_id: &str, repo_type: Option<RepoType>, revision: &str) -> String {
    let segment = constants::repo_type_api_segment(repo_type);
    format!("{}/api/{}/{}/xet-{}-token/{}", api.inner.endpoint, segment, repo_id, token_type, revision)
}

fn xet_file_info(hash: impl Into<String>, file_size: Option<u64>) -> XetFileInfo {
    XetFileInfo {
        hash: hash.into(),
        file_size,
        sha256: None,
    }
}

async fn prepare_xet_operation(
    api: &HfApi,
    token_type: &'static str,
    repo_id: &str,
    repo_type: Option<RepoType>,
    revision: &str,
) -> Result<XetOperationConfig> {
    let conn = fetch_xet_connection_info(api, token_type, repo_id, repo_type, revision).await?;
    let session = api.get_or_init_xet_session(&conn.endpoint)?;
    let (token, expiry) = conn.token_info();

    Ok(XetOperationConfig {
        session,
        token,
        expiry,
        refresh_url: xet_token_url(api, token_type, repo_id, repo_type, revision),
        refresh_headers: api.auth_headers(),
    })
}

async fn build_xet_download_group(
    api: &HfApi,
    repo_id: &str,
    repo_type: Option<RepoType>,
    revision: &str,
) -> Result<xet::xet_session::XetFileDownloadGroup> {
    let config = prepare_xet_operation(api, "read", repo_id, repo_type, revision).await?;

    config
        .session
        .new_file_download_group()
        .map_err(|e| HfError::Other(format!("Xet download failed: {e}")))?
        .with_token_info(config.token, config.expiry)
        .with_token_refresh_url(config.refresh_url, config.refresh_headers)
        .build()
        .await
        .map_err(|e| HfError::Other(format!("Xet download failed: {e}")))
}

async fn build_xet_upload_commit(
    api: &HfApi,
    repo_id: &str,
    repo_type: Option<RepoType>,
    revision: &str,
) -> Result<xet::xet_session::XetUploadCommit> {
    let config = prepare_xet_operation(api, "write", repo_id, repo_type, revision).await?;

    config
        .session
        .new_upload_commit()
        .map_err(|e| HfError::Other(format!("Xet upload failed: {e}")))?
        .with_token_info(config.token, config.expiry)
        .with_token_refresh_url(config.refresh_url, config.refresh_headers)
        .build()
        .await
        .map_err(|e| HfError::Other(format!("Xet upload failed: {e}")))
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
    let url = xet_token_url(api, token_type, repo_id, repo_type, revision);

    let response = api.inner.client.get(&url).headers(api.auth_headers()).send().await?;

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

pub(crate) async fn xet_download_to_local_dir(
    api: &HfApi,
    repo_id: &str,
    repo_type: Option<RepoType>,
    revision: &str,
    filename: &str,
    local_dir: &std::path::Path,
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

    let group = build_xet_download_group(api, repo_id, repo_type, revision).await?;

    tokio::fs::create_dir_all(local_dir).await?;
    let dest_path = local_dir.join(filename);
    if let Some(parent) = dest_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    group
        .download_file_to_path(xet_file_info(file_hash, Some(file_size)), dest_path.clone())
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
    path: &std::path::Path,
) -> Result<()> {
    let group = build_xet_download_group(api, repo_id, repo_type, revision).await?;

    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let incomplete_path = PathBuf::from(format!("{}.incomplete", path.display()));

    group
        .download_file_to_path(xet_file_info(file_hash.to_string(), Some(file_size)), incomplete_path.clone())
        .await
        .map_err(|e| HfError::Other(format!("Xet download failed: {e}")))?;

    group
        .finish()
        .await
        .map_err(|e| HfError::Other(format!("Xet download failed: {e}")))?;

    tokio::fs::rename(&incomplete_path, path).await?;
    Ok(())
}

pub(crate) struct XetBatchFile {
    pub hash: String,
    pub file_size: u64,
    pub path: PathBuf,
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

    let group = build_xet_download_group(api, repo_id, repo_type, revision)
        .await
        .map_err(|e| HfError::Other(format!("Xet batch download failed: {e}")))?;

    let mut incomplete_paths = Vec::with_capacity(files.len());
    for file in files {
        if let Some(parent) = file.path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let incomplete = PathBuf::from(format!("{}.incomplete", file.path.display()));

        group
            .download_file_to_path(xet_file_info(file.hash.clone(), Some(file.file_size)), incomplete.clone())
            .await
            .map_err(|e| HfError::Other(format!("Xet batch download failed: {e}")))?;

        incomplete_paths.push((incomplete, file.path.clone()));
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
    let commit = build_xet_upload_commit(api, repo_id, repo_type, revision).await?;

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
        task_ids_in_order.push(handle.task_id());
    }

    let results = commit
        .commit()
        .await
        .map_err(|e| HfError::Other(format!("Xet upload failed: {e}")))?;

    let mut xet_file_infos = Vec::with_capacity(files.len());
    for task_id in &task_ids_in_order {
        let metadata = results
            .uploads
            .get(task_id)
            .ok_or_else(|| HfError::Other("Missing xet upload result for task".to_string()))?;
        xet_file_infos.push(metadata.xet_info.clone());
    }

    Ok(xet_file_infos)
}

impl HfApi {
    /// Return the cached XetSession for an endpoint, creating it on first use.
    ///
    /// The session is reused across operations targeting the same CAS endpoint.
    /// Read/write credentials are configured on the per-operation builders.
    fn get_or_init_xet_session(&self, endpoint: &str) -> Result<xet::xet_session::XetSession> {
        {
            let guard = self
                .inner
                .xet_session
                .lock()
                .map_err(|e| HfError::Other(format!("xet session lock poisoned: {e}")))?;
            if let Some(cached) = guard.as_ref() {
                if cached.endpoint == endpoint {
                    return Ok(cached.session.clone());
                }
            }
        }

        let session = XetSessionBuilder::new()
            .with_endpoint(endpoint.to_string())
            .build()
            .map_err(|e| HfError::Other(format!("Failed to build xet session: {e}")))?;

        let mut guard = self
            .inner
            .xet_session
            .lock()
            .map_err(|e| HfError::Other(format!("xet session lock poisoned: {e}")))?;
        if let Some(existing) = guard.as_ref() {
            if existing.endpoint == endpoint {
                Ok(existing.session.clone())
            } else {
                *guard = Some(CachedXetSession {
                    endpoint: endpoint.to_string(),
                    session: session.clone(),
                });
                Ok(session)
            }
        } else {
            *guard = Some(CachedXetSession {
                endpoint: endpoint.to_string(),
                session: session.clone(),
            });
            Ok(session)
        }
    }

    /// Fetch a Xet connection token (read or write) for a repository.
    /// Endpoint: GET /api/{repo_type}s/{repo_id}/xet-{read|write}-token/{revision}
    pub async fn get_xet_token(&self, params: &GetXetTokenParams) -> Result<XetConnectionInfo> {
        let revision = params.revision.as_deref().unwrap_or(constants::DEFAULT_REVISION);
        fetch_xet_connection_info(self, params.token_type.as_str(), &params.repo_id, params.repo_type, revision).await
    }
}
