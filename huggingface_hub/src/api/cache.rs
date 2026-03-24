use crate::client::HfApi;
use crate::error::Result;
use crate::types::cache::{DeleteCacheRevision, HfCacheInfo};

impl HfApi {
    pub async fn scan_cache(&self) -> Result<HfCacheInfo> {
        crate::cache::scan_cache_dir(&self.inner.cache_dir).await
    }

    pub async fn delete_cache_revisions(&self, revisions: &[DeleteCacheRevision]) -> Result<()> {
        let refs: Vec<(&str, crate::types::RepoType, &str)> = revisions
            .iter()
            .map(|r| (r.repo_id.as_str(), r.repo_type, r.commit_hash.as_str()))
            .collect();
        crate::cache::delete_revisions(&self.inner.cache_dir, &refs).await
    }
}
