//! Xet high-performance transfer support.
//!
//! This module is only active when the "xet" feature is enabled.
//! When xet headers are detected during download/upload but the feature
//! is not enabled, HfError::XetNotEnabled is returned.

#[cfg(feature = "xet")]
pub(crate) struct XetConnectionInfo {
    pub endpoint: String,
    pub access_token: String,
    pub expiration_unix_epoch: u64,
}

#[cfg(feature = "xet")]
pub(crate) async fn xet_download(
    _api: &crate::client::HfApi,
    _params: &crate::types::DownloadFileParams,
    _head_response: &reqwest::Response,
) -> crate::error::Result<std::path::PathBuf> {
    // TODO: Implement xet download using hf-xet crate
    Err(crate::error::HfError::Other(
        "Xet download not yet implemented".to_string(),
    ))
}

#[cfg(feature = "xet")]
pub(crate) async fn xet_upload(
    _api: &crate::client::HfApi,
    _files: &[(String, crate::types::AddSource)],
    _repo_id: &str,
    _repo_type: Option<crate::types::RepoType>,
    _revision: &str,
) -> crate::error::Result<()> {
    // TODO: Implement xet upload using hf-xet crate
    Err(crate::error::HfError::Other(
        "Xet upload not yet implemented".to_string(),
    ))
}
