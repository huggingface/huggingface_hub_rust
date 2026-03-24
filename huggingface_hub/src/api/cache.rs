use crate::client::HfApi;
use crate::error::Result;
use crate::types::cache::HfCacheInfo;

impl HfApi {
    pub async fn scan_cache(&self) -> Result<HfCacheInfo> {
        crate::cache::scan_cache_dir(&self.inner.cache_dir).await
    }
}
