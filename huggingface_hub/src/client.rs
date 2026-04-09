use std::sync::Arc;

use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT};
use reqwest_middleware::ClientWithMiddleware;
use reqwest_retry::RetryTransientMiddleware;
use reqwest_retry::policies::ExponentialBackoff;
use tracing::debug;

use crate::constants;
use crate::error::{HFError, Result};

/// Async client for the Hugging Face Hub API.
///
/// `HFClient` wraps an `Arc<HFClientInner>` so it is cheap to clone — all clones
/// share the same underlying HTTP connection pool, token, and configuration.
///
/// # Creating a client
///
/// ```rust,no_run
/// use huggingface_hub::HFClient;
///
/// // Reads token and settings from the environment (HF_TOKEN, HF_ENDPOINT, …).
/// let client = HFClient::new()?;
///
/// // Or configure explicitly:
/// let client = HFClient::builder().token("hf_…").endpoint("https://huggingface.co").build()?;
/// # Ok::<(), huggingface_hub::HFError>(())
/// ```
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
    pub(crate) no_redirect_client: ClientWithMiddleware,
    pub(crate) endpoint: String,
    pub(crate) token: Option<String>,
    pub(crate) cache_dir: std::path::PathBuf,
    pub(crate) cache_enabled: bool,
    #[cfg(feature = "xet")]
    pub(crate) xet_session: std::sync::Mutex<Option<xet::xet_session::XetSession>>,
}

/// Builder for [`HFClient`].
///
/// Obtain one via [`HFClient::builder()`] or [`HFClientBuilder::new()`].
/// Call [`build()`](HFClientBuilder::build) when all options are set.
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
    /// Creates a builder with all options unset; defaults are applied at [`build`](Self::build) time.
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

    /// Overrides the Hub base URL (default: `https://huggingface.co`, or `HF_ENDPOINT` env var).
    pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    /// Sets the authentication token. Without this, the client falls back to the `HF_TOKEN` env
    /// var and the cached token file written by `huggingface-cli login`.
    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Overrides the `User-Agent` header sent with every request.
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    /// Merges additional default headers into every request. Ignored when a custom
    /// [`client`](Self::client) is supplied (configure headers on that client directly).
    pub fn headers(mut self, headers: HeaderMap) -> Self {
        self.headers = Some(headers);
        self
    }

    /// Supplies a pre-configured `reqwest::Client`. Retry middleware is still applied on top.
    /// The caller is responsible for any default headers (including `User-Agent`) on this client;
    /// the [`headers`](Self::headers) and [`user_agent`](Self::user_agent) options are ignored.
    pub fn client(mut self, client: reqwest::Client) -> Self {
        self.client = Some(client);
        self
    }

    /// Sets the local cache directory. Defaults to `HF_HUB_CACHE` → `$HF_HOME/hub` →
    /// `~/.cache/huggingface/hub`.
    pub fn cache_dir(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.cache_dir = Some(path.into());
        self
    }

    /// Enables or disables the local file cache. Caching is on by default.
    pub fn cache_enabled(mut self, enabled: bool) -> Self {
        self.cache_enabled = Some(enabled);
        self
    }

    /// Builds the [`HFClient`].
    ///
    /// # Errors
    ///
    /// Returns an error if the endpoint URL is not a valid URL or if the `reqwest` client
    /// cannot be constructed (e.g. an invalid `User-Agent` string was provided).
    pub fn build(self) -> Result<HFClient> {
        let endpoint = self
            .endpoint
            .or_else(|| std::env::var(constants::HF_ENDPOINT).ok())
            .unwrap_or_else(|| constants::DEFAULT_HF_ENDPOINT.to_string());

        let _ = url::Url::parse(&endpoint)?;

        let token = self.token.or_else(resolve_token);

        let cache_dir = self.cache_dir.unwrap_or_else(constants::resolve_cache_dir);

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
            HeaderValue::from_str(&user_agent).map_err(|e| HFError::Other(format!("Invalid user agent: {e}")))?,
        );

        let raw_client = match self.client {
            Some(c) => c,
            None => reqwest::Client::builder().default_headers(default_headers.clone()).build()?,
        };

        let no_redirect_raw = reqwest::Client::builder()
            .default_headers(default_headers)
            .redirect(reqwest::redirect::Policy::none())
            .build()?;

        let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
        let client = reqwest_middleware::ClientBuilder::new(raw_client)
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build();

        let no_redirect_retry = ExponentialBackoff::builder().build_with_max_retries(3);
        let no_redirect_client = reqwest_middleware::ClientBuilder::new(no_redirect_raw)
            .with(RetryTransientMiddleware::new_with_policy(no_redirect_retry))
            .build();

        Ok(HFClient {
            inner: Arc::new(HFClientInner {
                client,
                no_redirect_client,
                endpoint: endpoint.trim_end_matches('/').to_string(),
                token,
                cache_dir,
                cache_enabled: self.cache_enabled.unwrap_or(true),
                #[cfg(feature = "xet")]
                xet_session: std::sync::Mutex::new(None),
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
    /// Creates a client with default settings, reading the token and endpoint from the
    /// environment. Equivalent to `HFClient::builder().build()`.
    ///
    /// # Errors
    ///
    /// Fails if the resolved endpoint URL is invalid or the HTTP client cannot be built.
    pub fn new() -> Result<Self> {
        HFClientBuilder::new().build()
    }

    /// Returns an [`HFClientBuilder`] for fine-grained configuration.
    pub fn builder() -> HFClientBuilder {
        HFClientBuilder::new()
    }

    pub(crate) fn http_client(&self) -> &ClientWithMiddleware {
        &self.inner.client
    }

    pub(crate) fn no_redirect_client(&self) -> &ClientWithMiddleware {
        &self.inner.no_redirect_client
    }

    pub(crate) fn endpoint(&self) -> &str {
        &self.inner.endpoint
    }

    pub(crate) fn cache_dir(&self) -> &std::path::Path {
        &self.inner.cache_dir
    }

    pub(crate) fn cache_enabled(&self) -> bool {
        self.inner.cache_enabled
    }

    /// Build authorization headers for requests
    pub(crate) fn auth_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Some(ref token) = self.inner.token
            && let Ok(val) = HeaderValue::from_str(&format!("Bearer {token}"))
        {
            headers.insert(AUTHORIZATION, val);
        }
        headers
    }

    /// Build a URL for the API: {endpoint}/api/{segment}/{repo_id}
    pub(crate) fn api_url(&self, repo_type: Option<crate::types::RepoType>, repo_id: &str) -> String {
        let segment = constants::repo_type_api_segment(repo_type);
        format!("{}/api/{}/{}", self.endpoint(), segment, repo_id)
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
        format!("{}/{}{}/resolve/{}/{}", self.endpoint(), prefix, repo_id, revision, filename)
    }

    /// Check an HTTP response and map error status codes to HFError variants.
    /// Returns the response on success (2xx).
    ///
    /// `repo_id` and `not_found_ctx` control how 404s are mapped:
    /// - `NotFoundContext::Repo` → `HFError::RepoNotFound`
    /// - `NotFoundContext::Entry { path }` → `HFError::EntryNotFound`
    /// - `NotFoundContext::Revision { revision }` → `HFError::RevisionNotFound`
    /// - `NotFoundContext::Generic` → `HFError::Http`
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
            401 => Err(HFError::AuthRequired),
            404 => match not_found_ctx {
                crate::error::NotFoundContext::Repo => Err(HFError::RepoNotFound { repo_id: repo_id_str }),
                crate::error::NotFoundContext::Entry { path } => Err(HFError::EntryNotFound {
                    path,
                    repo_id: repo_id_str,
                }),
                crate::error::NotFoundContext::Revision { revision } => Err(HFError::RevisionNotFound {
                    revision,
                    repo_id: repo_id_str,
                }),
                crate::error::NotFoundContext::Generic => Err(HFError::Http { status, url, body }),
            },
            _ => Err(HFError::Http { status, url, body }),
        }
    }
}

#[cfg(feature = "xet")]
impl HFClient {
    /// Get or lazily create the cached XetSession.
    ///
    /// On first call, builds a new session and caches it. On subsequent calls,
    /// probes the cached session's health via a lightweight factory call. If
    /// the session is permanently poisoned (e.g. after abort), it is replaced
    /// with a fresh one transparently.
    ///
    /// Returns a cheap clone (Arc increment) of the shared session.
    pub(crate) fn xet_session(&self) -> Result<xet::xet_session::XetSession> {
        let mut guard = self
            .inner
            .xet_session
            .lock()
            .map_err(|e| HFError::Other(format!("xet session mutex poisoned: {e}")))?;

        if let Some(ref session) = *guard {
            match session.new_file_download_group() {
                Ok(_probe) => return Ok(session.clone()),
                Err(e) if crate::xet::is_session_poisoned(&e) => {
                    tracing::warn!(error = %e, "cached XetSession poisoned, replacing");
                },
                Err(e) => {
                    return Err(HFError::Other(format!("XetSession health check failed: {e}")));
                },
            }
        }

        let session = xet::xet_session::XetSessionBuilder::new()
            .build()
            .map_err(|e| HFError::Other(format!("Failed to build xet session: {e}")))?;
        *guard = Some(session.clone());
        Ok(session)
    }
}

/// Resolve token from environment or token file.
/// Priority: HF_TOKEN env → HF_TOKEN_PATH file → $HF_HOME/token file.
fn resolve_token() -> Option<String> {
    if let Ok(val) = std::env::var(constants::HF_HUB_DISABLE_IMPLICIT_TOKEN)
        && !val.is_empty()
    {
        debug!("implicit token disabled via HF_HUB_DISABLE_IMPLICIT_TOKEN");
        return None;
    }

    if let Ok(token) = std::env::var(constants::HF_TOKEN)
        && !token.is_empty()
    {
        debug!("resolved token from HF_TOKEN env var");
        return Some(token);
    }

    if let Ok(path) = std::env::var(constants::HF_TOKEN_PATH)
        && let Ok(token) = std::fs::read_to_string(&path)
    {
        let token = token.trim().to_string();
        if !token.is_empty() {
            debug!("resolved token from HF_TOKEN_PATH file");
            return Some(token);
        }
    }

    let hf_home = constants::hf_home();
    let token_path = hf_home.join(constants::TOKEN_FILENAME);
    if let Ok(token) = std::fs::read_to_string(&token_path) {
        let token = token.trim().to_string();
        if !token.is_empty() {
            debug!("resolved token from stored token file");
            return Some(token);
        }
    }

    debug!("no token found");
    None
}

#[cfg(test)]
mod tests {
    use super::HFClientBuilder;

    #[test]
    fn test_builder_cache_dir_explicit() {
        let api = HFClientBuilder::new().cache_dir("/tmp/my-cache").build().unwrap();
        assert_eq!(api.cache_dir(), std::path::Path::new("/tmp/my-cache"));
    }

    #[test]
    fn test_builder_cache_dir_default() {
        let api = HFClientBuilder::new().build().unwrap();
        let path_str = api.cache_dir().to_string_lossy();
        assert!(path_str.contains("huggingface") && path_str.ends_with("hub"));
    }

    #[cfg(feature = "xet")]
    #[test]
    fn test_xet_session_lazy_creation() {
        let client = HFClientBuilder::new().build().unwrap();
        assert!(client.inner.xet_session.lock().unwrap().is_none());
        let _s1 = client.xet_session().unwrap();
        assert!(client.inner.xet_session.lock().unwrap().is_some());
    }

    #[cfg(feature = "xet")]
    #[test]
    fn test_xet_session_shared_across_clones() {
        let client = HFClientBuilder::new().build().unwrap();
        let clone = client.clone();
        let _s1 = client.xet_session().unwrap();
        assert!(clone.inner.xet_session.lock().unwrap().is_some());
    }
}
