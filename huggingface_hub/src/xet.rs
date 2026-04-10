//! Xet high-performance transfer support.
//!
//! This module is only compiled when the "xet" feature is enabled.
//! When xet headers are detected during download/upload but the feature
//! is not enabled, HFError::XetNotEnabled is returned at the call site.

use std::path::PathBuf;

use serde::Deserialize;
use xet::xet_session::{Sha256Policy, XetFileInfo, XetFileMetadata};

use crate::client::HFClient;
use crate::constants;
use crate::error::{HFError, Result};
use crate::repository::HFRepository;
use crate::types::{AddSource, GetXetTokenParams, RepoType};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct XetTokenResponse {
    access_token: String,
    exp: u64,
    cas_url: String,
}

#[derive(Default)]
pub(crate) struct XetState {
    pub(crate) session: Option<xet::xet_session::XetSession>,
    pub(crate) generation: u64,
}

pub struct XetConnectionInfo {
    pub endpoint: String,
    pub access_token: String,
    pub expiration_unix_epoch: u64,
}

async fn fetch_xet_connection_info(api: &HFClient, token_url: &str) -> Result<XetConnectionInfo> {
    let response = api.http_client().get(token_url).headers(api.auth_headers()).send().await?;

    let response = api
        .check_response(response, None, crate::error::NotFoundContext::Generic)
        .await?;

    let token_resp: XetTokenResponse = response.json().await?;
    Ok(XetConnectionInfo {
        endpoint: token_resp.cas_url,
        access_token: token_resp.access_token,
        expiration_unix_epoch: token_resp.exp,
    })
}

fn repo_xet_token_url(
    api: &HFClient,
    token_type: &str,
    repo_id: &str,
    repo_type: Option<RepoType>,
    revision: &str,
) -> String {
    let segment = constants::repo_type_api_segment(repo_type);
    format!("{}/api/{}/{}/xet-{}-token/{}", api.endpoint(), segment, repo_id, token_type, revision)
}

#[allow(dead_code)]
pub(crate) fn bucket_xet_token_url(api: &HFClient, token_type: &str, bucket_id: &str) -> String {
    format!("{}/api/buckets/{}/xet-{}-token", api.endpoint(), bucket_id, token_type)
}

/// Returns `true` if the error indicates the XetSession is permanently
/// poisoned and must be replaced before retrying.
#[cfg(test)]
fn is_session_poisoned(err: &xet::error::XetError) -> bool {
    use xet::error::XetError;
    matches!(
        err,
        XetError::UserCancelled(_)
            | XetError::AlreadyCompleted
            | XetError::PreviousTaskError(_)
            | XetError::KeyboardInterrupt
    )
}

pub(crate) struct XetBatchFile {
    pub hash: String,
    pub file_size: u64,
    pub path: PathBuf,
}

impl HFRepository {
    pub(crate) async fn xet_download_to_local_dir(
        &self,
        revision: &str,
        filename: &str,
        local_dir: &std::path::Path,
        head_response: &reqwest::Response,
    ) -> Result<PathBuf> {
        let repo_path = self.repo_path();
        let repo_type = Some(self.repo_type);
        let file_hash = crate::api::files::extract_xet_hash(head_response)
            .ok_or_else(|| HFError::Other("Missing X-Xet-Hash header".to_string()))?;

        let file_size: u64 = crate::api::files::extract_file_size(head_response).unwrap_or(0);

        let token_url = repo_xet_token_url(&self.hf_client, "read", &repo_path, repo_type, revision);
        let conn = fetch_xet_connection_info(&self.hf_client, &token_url).await?;

        tokio::fs::create_dir_all(local_dir).await?;
        let dest_path = local_dir.join(filename);
        if let Some(parent) = dest_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let (session, generation) = self.hf_client.xet_session()?;
        let group = match session.new_file_download_group() {
            Ok(b) => b,
            Err(e) => {
                self.hf_client.replace_xet_session(generation, &e);
                self.hf_client
                    .xet_session()?
                    .0
                    .new_file_download_group()
                    .map_err(|e| HFError::Other(format!("Xet download failed: {e}")))?
            },
        }
        .with_endpoint(conn.endpoint.clone())
        .with_token_info(conn.access_token.clone(), conn.expiration_unix_epoch)
        .with_token_refresh_url(token_url, self.hf_client.auth_headers())
        .build()
        .await
        .map_err(|e| HFError::Other(format!("Xet download failed: {e}")))?;

        let file_info = XetFileInfo::new(file_hash, file_size);

        group
            .download_file_to_path(file_info, dest_path.clone())
            .await
            .map_err(|e| HFError::Other(format!("Xet download failed: {e}")))?;

        group
            .finish()
            .await
            .map_err(|e| HFError::Other(format!("Xet download failed: {e}")))?;

        Ok(dest_path)
    }

    pub(crate) async fn xet_download_to_blob(
        &self,
        revision: &str,
        file_hash: &str,
        file_size: u64,
        path: &std::path::Path,
    ) -> Result<()> {
        let repo_path = self.repo_path();
        let repo_type = Some(self.repo_type);
        let token_url = repo_xet_token_url(&self.hf_client, "read", &repo_path, repo_type, revision);
        let conn = fetch_xet_connection_info(&self.hf_client, &token_url).await?;

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let incomplete_path = PathBuf::from(format!("{}.incomplete", path.display()));

        let (session, generation) = self.hf_client.xet_session()?;
        let group = match session.new_file_download_group() {
            Ok(b) => b,
            Err(e) => {
                self.hf_client.replace_xet_session(generation, &e);
                self.hf_client
                    .xet_session()?
                    .0
                    .new_file_download_group()
                    .map_err(|e| HFError::Other(format!("Xet download failed: {e}")))?
            },
        }
        .with_endpoint(conn.endpoint.clone())
        .with_token_info(conn.access_token.clone(), conn.expiration_unix_epoch)
        .with_token_refresh_url(token_url, self.hf_client.auth_headers())
        .build()
        .await
        .map_err(|e| HFError::Other(format!("Xet download failed: {e}")))?;

        let file_info = XetFileInfo::new(file_hash.to_string(), file_size);

        group
            .download_file_to_path(file_info, incomplete_path.clone())
            .await
            .map_err(|e| HFError::Other(format!("Xet download failed: {e}")))?;

        group
            .finish()
            .await
            .map_err(|e| HFError::Other(format!("Xet download failed: {e}")))?;

        tokio::fs::rename(&incomplete_path, path).await?;
        Ok(())
    }

    pub(crate) async fn xet_download_batch(&self, revision: &str, files: &[XetBatchFile]) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }

        let repo_path = self.repo_path();
        let repo_type = Some(self.repo_type);
        let token_url = repo_xet_token_url(&self.hf_client, "read", &repo_path, repo_type, revision);
        let conn = fetch_xet_connection_info(&self.hf_client, &token_url).await?;

        let (session, generation) = self.hf_client.xet_session()?;
        let group = match session.new_file_download_group() {
            Ok(b) => b,
            Err(e) => {
                self.hf_client.replace_xet_session(generation, &e);
                self.hf_client
                    .xet_session()?
                    .0
                    .new_file_download_group()
                    .map_err(|e| HFError::Other(format!("Xet batch download failed: {e}")))?
            },
        }
        .with_endpoint(conn.endpoint.clone())
        .with_token_info(conn.access_token.clone(), conn.expiration_unix_epoch)
        .with_token_refresh_url(token_url, self.hf_client.auth_headers())
        .build()
        .await
        .map_err(|e| HFError::Other(format!("Xet batch download failed: {e}")))?;

        let mut incomplete_paths = Vec::with_capacity(files.len());
        for file in files {
            if let Some(parent) = file.path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }

            let incomplete = PathBuf::from(format!("{}.incomplete", file.path.display()));

            let file_info = XetFileInfo::new(file.hash.clone(), file.file_size);

            group
                .download_file_to_path(file_info, incomplete.clone())
                .await
                .map_err(|e| HFError::Other(format!("Xet batch download failed: {e}")))?;

            incomplete_paths.push((incomplete, file.path.clone()));
        }

        group
            .finish()
            .await
            .map_err(|e| HFError::Other(format!("Xet batch download failed: {e}")))?;

        for (incomplete, final_path) in &incomplete_paths {
            tokio::fs::rename(incomplete, final_path).await?;
        }

        Ok(())
    }

    /// Download a file (or byte range) via xet and return a byte stream.
    ///
    /// Uses `XetDownloadStreamGroup` which supports `Option<Range<u64>>` for partial downloads.
    pub(crate) async fn xet_download_stream(
        &self,
        revision: &str,
        file_hash: &str,
        file_size: u64,
        range: Option<std::ops::Range<u64>>,
    ) -> Result<impl futures::Stream<Item = Result<bytes::Bytes>> + use<>> {
        let repo_path = self.repo_path();
        let repo_type = Some(self.repo_type);
        let token_url = repo_xet_token_url(&self.hf_client, "read", &repo_path, repo_type, revision);
        let conn = fetch_xet_connection_info(&self.hf_client, &token_url).await?;

        let (session, generation) = self.hf_client.xet_session()?;
        let group = match session.new_download_stream_group() {
            Ok(b) => b,
            Err(e) => {
                self.hf_client.replace_xet_session(generation, &e);
                self.hf_client
                    .xet_session()?
                    .0
                    .new_download_stream_group()
                    .map_err(|e| HFError::Other(format!("Xet stream download failed: {e}")))?
            },
        }
        .with_endpoint(conn.endpoint.clone())
        .with_token_info(conn.access_token.clone(), conn.expiration_unix_epoch)
        .with_token_refresh_url(token_url, self.hf_client.auth_headers())
        .build()
        .await
        .map_err(|e| HFError::Other(format!("Xet stream download failed: {e}")))?;

        let file_info = XetFileInfo::new(file_hash.to_string(), file_size);

        let mut stream = group
            .download_stream(file_info, range)
            .await
            .map_err(|e| HFError::Other(format!("Xet stream download failed: {e}")))?;

        stream.start();

        Ok(futures::stream::unfold(stream, |mut stream| async move {
            match stream.next().await {
                Ok(Some(bytes)) => Some((Ok(bytes), stream)),
                Ok(None) => None,
                Err(e) => Some((Err(HFError::Other(format!("Xet stream read failed: {e}"))), stream)),
            }
        }))
    }

    /// Upload files using the xet protocol.
    /// Fetches a write token and uses xet-session's UploadCommit.
    /// Returns the XetFileInfo (hash + size) for each uploaded file.
    pub(crate) async fn xet_upload(&self, files: &[(String, AddSource)], revision: &str) -> Result<Vec<XetFileInfo>> {
        let repo_path = self.repo_path();
        let repo_type = Some(self.repo_type);
        tracing::info!(repo = repo_path.as_str(), "fetching xet write token");
        let token_url = repo_xet_token_url(&self.hf_client, "write", &repo_path, repo_type, revision);
        let conn = fetch_xet_connection_info(&self.hf_client, &token_url).await?;
        tracing::info!(endpoint = conn.endpoint.as_str(), "xet write token obtained, building session");

        tracing::info!("building xet upload commit");
        let (session, generation) = self.hf_client.xet_session()?;
        let commit = match session.new_upload_commit() {
            Ok(b) => b,
            Err(e) => {
                self.hf_client.replace_xet_session(generation, &e);
                self.hf_client
                    .xet_session()?
                    .0
                    .new_upload_commit()
                    .map_err(|e| HFError::Other(format!("Xet upload failed: {e}")))?
            },
        }
        .with_endpoint(conn.endpoint.clone())
        .with_token_info(conn.access_token.clone(), conn.expiration_unix_epoch)
        .with_token_refresh_url(token_url, self.hf_client.auth_headers())
        .build()
        .await
        .map_err(|e| HFError::Other(format!("Xet upload failed: {e}")))?;
        tracing::info!("xet upload commit built, queuing file uploads");

        let mut task_ids_in_order = Vec::with_capacity(files.len());

        for (path_in_repo, source) in files {
            tracing::info!(path = path_in_repo.as_str(), "queuing xet upload");
            let handle = match source {
                AddSource::File(path) => commit
                    .upload_from_path(path.clone(), Sha256Policy::Compute)
                    .await
                    .map_err(|e| HFError::Other(format!("Xet upload failed: {e}")))?,
                AddSource::Bytes(bytes) => commit
                    .upload_bytes(bytes.clone(), Sha256Policy::Compute, None)
                    .await
                    .map_err(|e| HFError::Other(format!("Xet upload failed: {e}")))?,
            };
            task_ids_in_order.push(handle.task_id());
        }

        tracing::info!(file_count = files.len(), "committing xet uploads");
        let results = commit
            .commit()
            .await
            .map_err(|e| HFError::Other(format!("Xet upload failed: {e}")))?;
        tracing::info!("xet upload commit complete");

        let mut xet_file_infos = Vec::with_capacity(files.len());
        for task_id in &task_ids_in_order {
            let metadata: &XetFileMetadata = results
                .uploads
                .get(task_id)
                .ok_or_else(|| HFError::Other("Missing xet upload result for task".to_string()))?;
            xet_file_infos.push(metadata.xet_info.clone());
        }

        Ok(xet_file_infos)
    }
}

impl HFClient {
    /// Fetch a Xet connection token (read or write) for a repository.
    /// Endpoint: GET /api/{repo_type}s/{repo_id}/xet-{read|write}-token/{revision}
    pub async fn get_xet_token(&self, params: &GetXetTokenParams) -> Result<XetConnectionInfo> {
        let revision = params.revision.as_deref().unwrap_or(constants::DEFAULT_REVISION);
        let token_url =
            repo_xet_token_url(self, params.token_type.as_str(), &params.repo_id, params.repo_type, revision);
        fetch_xet_connection_info(self, &token_url).await
    }
}

#[cfg(test)]
mod tests {
    use xet::error::XetError;

    use super::*;

    #[test]
    fn test_session_poisoned_positive() {
        assert!(is_session_poisoned(&XetError::UserCancelled("test".into())));
        assert!(is_session_poisoned(&XetError::AlreadyCompleted));
        assert!(is_session_poisoned(&XetError::PreviousTaskError("err".into())));
        assert!(is_session_poisoned(&XetError::KeyboardInterrupt));
    }

    #[test]
    fn test_session_poisoned_negative() {
        let non_poisoned = [
            XetError::Network("timeout".into()),
            XetError::Authentication("bad token".into()),
            XetError::Io("disk full".into()),
            XetError::Internal("bug".into()),
            XetError::Timeout("slow".into()),
            XetError::NotFound("missing".into()),
            XetError::DataIntegrity("corrupt".into()),
            XetError::Configuration("bad config".into()),
            XetError::Cancelled("cancelled".into()),
            XetError::WrongRuntimeMode("wrong mode".into()),
            XetError::TaskError("task failed".into()),
        ];
        for err in &non_poisoned {
            assert!(!is_session_poisoned(err), "{err:?} should NOT be classified as poisoned");
        }
    }

    #[test]
    fn test_xet_error_message_preserved_in_hferror() {
        let xet_err = XetError::Network("connection reset by peer".into());
        let hf_err = HFError::Other(format!("Xet download failed: {xet_err}"));
        let msg = hf_err.to_string();
        assert!(msg.contains("Xet download failed"), "missing prefix: {msg}");
        assert!(msg.contains("connection reset by peer"), "missing original message: {msg}");
    }
}
