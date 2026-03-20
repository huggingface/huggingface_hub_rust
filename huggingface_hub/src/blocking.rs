use crate::client::HfApi;
use crate::error::{HfError, Result};

pub struct HfApiSync {
    pub(crate) inner: HfApi,
    pub(crate) runtime: tokio::runtime::Runtime,
}

impl HfApiSync {
    pub fn new() -> Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| HfError::Other(format!("Failed to create tokio runtime: {e}")))?;
        let inner = HfApi::new()?;
        Ok(Self { inner, runtime })
    }

    pub fn from_api(api: HfApi) -> Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| HfError::Other(format!("Failed to create tokio runtime: {e}")))?;
        Ok(Self {
            inner: api,
            runtime,
        })
    }

    pub fn api(&self) -> &HfApi {
        &self.inner
    }
}

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
        let api = HfApi::new().unwrap();
        let sync_api = HfApiSync::from_api(api);
        assert!(sync_api.is_ok());
    }
}
