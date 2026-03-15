//! Xet high-performance transfer support.
//!
//! This module is only compiled when the "xet" feature is enabled.
//! When xet headers are detected during download/upload but the feature
//! is not enabled, HfError::XetNotEnabled is returned at the call site.

use std::path::PathBuf;
use std::sync::Arc;

use serde::Deserialize;
use xet_client::cas_client::auth::{AuthError, TokenRefresher};
use xet_data::processing::data_client;
use xet_data::processing::{Sha256Policy, XetFileInfo};

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
/// fetches a read token, and calls xet_data's download_async.
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

    let conn =
        fetch_xet_connection_info(api, "read", &params.repo_id, params.repo_type, revision).await?;

    let token_refresher: Arc<dyn TokenRefresher> = Arc::new(HubTokenRefresher {
        api: api.clone(),
        repo_id: params.repo_id.clone(),
        repo_type: params.repo_type,
        revision: revision.to_string(),
        token_type: "read",
    });

    tokio::fs::create_dir_all(&params.local_dir).await?;
    let dest_path = params.local_dir.join(&params.filename);
    if let Some(parent) = dest_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let file_info = XetFileInfo::new(file_hash, file_size);
    let dest_str = dest_path.to_string_lossy().to_string();

    let token_info = conn.token_info();
    data_client::download_async(
        vec![(file_info, dest_str)],
        Some(conn.endpoint),
        Some(token_info),
        Some(token_refresher),
        None,
        None,
    )
    .await
    .map_err(|e| HfError::Other(format!("Xet download failed: {e}")))?;

    Ok(dest_path)
}

/// Upload files using the xet protocol.
/// Fetches a write token and calls xet_data's upload functions.
/// Returns the XetFileInfo (hash + size) for each uploaded file.
#[allow(dead_code)]
pub(crate) async fn xet_upload(
    api: &HfApi,
    files: &[(String, AddSource)],
    repo_id: &str,
    repo_type: Option<RepoType>,
    revision: &str,
) -> Result<Vec<XetFileInfo>> {
    let conn = fetch_xet_connection_info(api, "write", repo_id, repo_type, revision).await?;

    let token_refresher: Arc<dyn TokenRefresher> = Arc::new(HubTokenRefresher {
        api: api.clone(),
        repo_id: repo_id.to_string(),
        repo_type,
        revision: revision.to_string(),
        token_type: "write",
    });

    let mut path_files: Vec<String> = Vec::new();
    let mut byte_files: Vec<Vec<u8>> = Vec::new();
    let mut path_indices: Vec<usize> = Vec::new();
    let mut byte_indices: Vec<usize> = Vec::new();

    for (i, (_path_in_repo, source)) in files.iter().enumerate() {
        match source {
            AddSource::File(path) => {
                path_files.push(path.to_string_lossy().to_string());
                path_indices.push(i);
            }
            AddSource::Bytes(bytes) => {
                byte_files.push(bytes.clone());
                byte_indices.push(i);
            }
        }
    }

    let mut results: Vec<Option<XetFileInfo>> = vec![None; files.len()];

    if !path_files.is_empty() {
        let sha256_policies = vec![Sha256Policy::Compute; path_files.len()];
        let infos = data_client::upload_async(
            path_files,
            sha256_policies,
            Some(conn.endpoint.clone()),
            Some(conn.token_info()),
            Some(token_refresher.clone()),
            None,
            None,
        )
        .await
        .map_err(|e| HfError::Other(format!("Xet upload (files) failed: {e}")))?;

        for (idx, info) in path_indices.into_iter().zip(infos) {
            results[idx] = Some(info);
        }
    }

    if !byte_files.is_empty() {
        let sha256_policies = vec![Sha256Policy::Compute; byte_files.len()];
        let infos = data_client::upload_bytes_async(
            byte_files,
            sha256_policies,
            Some(conn.endpoint.clone()),
            Some(conn.token_info()),
            Some(token_refresher),
            None,
            None,
        )
        .await
        .map_err(|e| HfError::Other(format!("Xet upload (bytes) failed: {e}")))?;

        for (idx, info) in byte_indices.into_iter().zip(infos) {
            results[idx] = Some(info);
        }
    }

    results
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| HfError::Other("Unexpected missing xet upload result".to_string()))
}

impl HfApi {
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
