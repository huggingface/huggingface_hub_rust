use std::sync::Arc;

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};
use reqwest_middleware::ClientWithMiddleware;
use reqwest_retry::policies::ExponentialBackoff;
use reqwest_retry::RetryTransientMiddleware;

use crate::constants;
use crate::error::{HfError, Result};

pub struct HFClient {
    pub(crate) inner: Arc<HFClientInner>,
}

impl Clone for HFClient {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

pub(crate) struct HFClientInner {
    pub(crate) client: ClientWithMiddleware,
    pub(crate) endpoint: String,
    pub(crate) token: Option<String>,
    pub(crate) cache_dir: std::path::PathBuf,
    pub(crate) cache_enabled: bool,
}

pub struct HFClientBuilder {
    endpoint: Option<String>,
    token: Option<String>,
    user_agent: Option<String>,
    headers: Option<HeaderMap>,
    client: Option<reqwest::Client>,
    cache_dir: Option<std::path::PathBuf>,
    cache_enabled: Option<bool>,
}

impl HFClientBuilder {
    pub fn new() -> Self {
        Self {
            endpoint: None,
            token: None,
            user_agent: None,
            headers: None,
            client: None,
            cache_dir: None,
            cache_enabled: None,
        }
    }

    pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    pub fn headers(mut self, headers: HeaderMap) -> Self {
        self.headers = Some(headers);
        self
    }

    /// Provide a pre-configured reqwest::Client. The retry middleware will
    /// still be applied on top. Caller is responsible for setting User-Agent
    /// and other default headers on this client.
    pub fn client(mut self, client: reqwest::Client) -> Self {
        self.client = Some(client);
        self
    }

    pub fn cache_dir(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.cache_dir = Some(path.into());
        self
    }

    pub fn cache_enabled(mut self, enabled: bool) -> Self {
        self.cache_enabled = Some(enabled);
        self
    }

    pub fn build(self) -> Result<HFClient> {
        let endpoint = self
            .endpoint
            .or_else(|| std::env::var(constants::HF_ENDPOINT).ok())
            .unwrap_or_else(|| constants::DEFAULT_HF_ENDPOINT.to_string());

        let _ = url::Url::parse(&endpoint)?;

        let token = self.token.or_else(resolve_token);

        let cache_dir = self.cache_dir.unwrap_or_else(resolve_cache_dir);

        let mut default_headers = self.headers.unwrap_or_default();

        let user_agent = self.user_agent.unwrap_or_else(|| {
            let ua_origin = std::env::var(constants::HF_HUB_USER_AGENT_ORIGIN).ok();
            match ua_origin {
                Some(origin) => format!("huggingface-hub-rust/0.1.0; {origin}"),
                None => "huggingface-hub-rust/0.1.0".to_string(),
            }
        });
        default_headers.insert(
            USER_AGENT,
            HeaderValue::from_str(&user_agent).map_err(|e| HfError::Other(format!("Invalid user agent: {e}")))?,
        );

        let raw_client = match self.client {
            Some(c) => c,
            None => reqwest::Client::builder().default_headers(default_headers).build()?,
        };

        let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
        let client = reqwest_middleware::ClientBuilder::new(raw_client)
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build();

        Ok(HFClient {
            inner: Arc::new(HFClientInner {
                client,
                endpoint: endpoint.trim_end_matches('/').to_string(),
                token,
                cache_dir,
                cache_enabled: self.cache_enabled.unwrap_or(true),
            }),
        })
    }
}

impl Default for HFClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl HFClient {
    pub fn new() -> Result<Self> {
        HFClientBuilder::new().build()
    }

    pub fn builder() -> HFClientBuilder {
        HFClientBuilder::new()
    }

    /// Build authorization headers for requests
    pub(crate) fn auth_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Some(ref token) = self.inner.token {
            if let Ok(val) = HeaderValue::from_str(&format!("Bearer {token}")) {
                headers.insert(AUTHORIZATION, val);
            }
        }
        headers
    }

    /// Build a URL for the API: {endpoint}/api/{segment}/{repo_id}
    pub(crate) fn api_url(&self, repo_type: Option<crate::types::RepoType>, repo_id: &str) -> String {
        let segment = constants::repo_type_api_segment(repo_type);
        format!("{}/api/{}/{}", self.inner.endpoint, segment, repo_id)
    }

    /// Build a download URL: {endpoint}/{prefix}{repo_id}/resolve/{revision}/{filename}
    pub(crate) fn download_url(
        &self,
        repo_type: Option<crate::types::RepoType>,
        repo_id: &str,
        revision: &str,
        filename: &str,
    ) -> String {
        let prefix = constants::repo_type_url_prefix(repo_type);
        format!("{}/{}{}/resolve/{}/{}", self.inner.endpoint, prefix, repo_id, revision, filename)
    }

    /// Check an HTTP response and map error status codes to HfError variants.
    /// Returns the response on success (2xx).
    ///
    /// `repo_id` and `not_found_ctx` control how 404s are mapped:
    /// - `NotFoundContext::Repo` → `HfError::RepoNotFound`
    /// - `NotFoundContext::Entry { path }` → `HfError::EntryNotFound`
    /// - `NotFoundContext::Revision { revision }` → `HfError::RevisionNotFound`
    /// - `NotFoundContext::Generic` → `HfError::Http`
    pub(crate) async fn check_response(
        &self,
        response: reqwest::Response,
        repo_id: Option<&str>,
        not_found_ctx: crate::error::NotFoundContext,
    ) -> Result<reqwest::Response> {
        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }

        let url = response.url().to_string();
        let body = response.text().await.unwrap_or_default();
        let repo_id_str = repo_id.unwrap_or("").to_string();

        match status.as_u16() {
            401 => Err(HfError::AuthRequired),
            404 => match not_found_ctx {
                crate::error::NotFoundContext::Repo => Err(HfError::RepoNotFound { repo_id: repo_id_str }),
                crate::error::NotFoundContext::Entry { path } => Err(HfError::EntryNotFound {
                    path,
                    repo_id: repo_id_str,
                }),
                crate::error::NotFoundContext::Revision { revision } => Err(HfError::RevisionNotFound {
                    revision,
                    repo_id: repo_id_str,
                }),
                crate::error::NotFoundContext::Generic => Err(HfError::Http { status, url, body }),
            },
            _ => Err(HfError::Http { status, url, body }),
        }
    }
}

pub type HfApi = HFClient;
pub type HfApiBuilder = HFClientBuilder;
pub type HfClient = HFClient;
pub type HfClientBuilder = HFClientBuilder;

/// Resolve token from environment or token file.
/// Priority: HF_TOKEN env → HF_TOKEN_PATH file → $HF_HOME/token file.
fn resolve_token() -> Option<String> {
    if let Ok(val) = std::env::var(constants::HF_HUB_DISABLE_IMPLICIT_TOKEN) {
        if !val.is_empty() {
            return None;
        }
    }

    if let Ok(token) = std::env::var(constants::HF_TOKEN) {
        if !token.is_empty() {
            return Some(token);
        }
    }

    if let Ok(path) = std::env::var(constants::HF_TOKEN_PATH) {
        if let Ok(token) = std::fs::read_to_string(&path) {
            let token = token.trim().to_string();
            if !token.is_empty() {
                return Some(token);
            }
        }
    }

    let hf_home = std::env::var(constants::HF_HOME).unwrap_or_else(|_| {
        let home = dirs_or_home();
        format!("{home}/.cache/huggingface")
    });
    let token_path = format!("{hf_home}/{}", constants::TOKEN_FILENAME);
    if let Ok(token) = std::fs::read_to_string(&token_path) {
        let token = token.trim().to_string();
        if !token.is_empty() {
            return Some(token);
        }
    }

    None
}

fn dirs_or_home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string())
}

/// Resolve cache directory from environment.
/// Priority: HF_HUB_CACHE env → $HF_HOME/hub → ~/.cache/huggingface/hub.
fn resolve_cache_dir() -> std::path::PathBuf {
    if let Ok(cache) = std::env::var(constants::HF_HUB_CACHE) {
        return std::path::PathBuf::from(cache);
    }
    if let Ok(hf_home) = std::env::var(constants::HF_HOME) {
        return std::path::PathBuf::from(hf_home).join("hub");
    }
    if let Ok(xdg) = std::env::var(constants::XDG_CACHE_HOME) {
        return std::path::PathBuf::from(xdg).join("huggingface").join("hub");
    }
    let home = dirs_or_home();
    std::path::PathBuf::from(format!("{home}/.cache/huggingface")).join("hub")
}

#[cfg(test)]
mod tests {
    use super::HFClientBuilder;

    #[test]
    fn test_builder_cache_dir_explicit() {
        let api = HFClientBuilder::new().cache_dir("/tmp/my-cache").build().unwrap();
        assert_eq!(api.inner.cache_dir, std::path::PathBuf::from("/tmp/my-cache"));
    }

    #[test]
    fn test_builder_cache_dir_default() {
        let api = HFClientBuilder::new().build().unwrap();
        let path_str = api.inner.cache_dir.to_string_lossy();
        assert!(path_str.contains("huggingface") && path_str.ends_with("hub"));
    }
}
