use crate::client::HFClient;
use crate::error::{HfError, Result};

pub struct HfApiSync {
    pub(crate) inner: HFClient,
    pub(crate) runtime: tokio::runtime::Runtime,
}

impl HfApiSync {
    pub fn new() -> Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| HfError::Other(format!("Failed to create tokio runtime: {e}")))?;
        let inner = HFClient::new()?;
        Ok(Self { inner, runtime })
    }

    pub fn from_api(api: HFClient) -> Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| HfError::Other(format!("Failed to create tokio runtime: {e}")))?;
        Ok(Self { inner: api, runtime })
    }

    pub fn api(&self) -> &HFClient {
        &self.inner
    }
}

pub type HFClientSync = HfApiSync;
pub type HfClientSync = HFClientSync;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hfapisync_creation() {
        let sync_api = HfApiSync::new();
        assert!(sync_api.is_ok());
    }

    #[test]
    fn test_hfapisync_from_api() {
        let api = HFClient::new().unwrap();
        let sync_api = HfApiSync::from_api(api);
        assert!(sync_api.is_ok());
    }
}
