use std::fmt;
use std::ops::Deref;
use std::sync::Arc;

use crate::client::HFClient;
use crate::error::{HFError, Result};
use crate::{repository as repo, types};

fn build_runtime() -> Result<Arc<tokio::runtime::Runtime>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map(Arc::new)
        .map_err(|e| HFError::Other(format!("Failed to create tokio runtime: {e}")))
}

/// Synchronous/blocking counterpart to [`HFClient`].
///
/// Wraps an `HFClient` together with a dedicated single-threaded tokio runtime so
/// that every async API method can be called from synchronous code. The runtime is
/// shared with all repo/space handles derived from this client.
#[derive(Clone)]
pub struct HFClientSync {
    pub(crate) inner: HFClient,
    pub(crate) runtime: Arc<tokio::runtime::Runtime>,
}

/// Synchronous/blocking counterpart to [`repo::HFRepository`].
///
/// Holds a reference to the underlying async handle and the shared tokio runtime.
/// Derefs to [`repo::HFRepository`], so all accessor methods (owner, name, repo_path,
/// etc.) are available directly. Blocking API methods are defined via the `sync_api!`
/// macro in the corresponding `api/` modules.
#[derive(Clone)]
pub struct HFRepositorySync {
    pub(crate) inner: repo::HFRepository,
    pub(crate) runtime: Arc<tokio::runtime::Runtime>,
}

/// Synchronous/blocking counterpart to [`repo::HFSpace`].
///
/// Derefs to [`HFRepositorySync`] so all blocking repository methods and accessors
/// are available directly. Space-specific blocking methods are defined via the
/// `sync_api!` macro.
#[derive(Clone)]
pub struct HFSpaceSync {
    repo: HFRepositorySync,
    pub(crate) inner: repo::HFSpace,
}

impl fmt::Debug for HFClientSync {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HFClientSync").finish()
    }
}

impl fmt::Debug for HFRepositorySync {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HFRepositorySync").field("inner", &self.inner).finish()
    }
}

impl fmt::Debug for HFSpaceSync {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HFSpaceSync").field("inner", &self.inner).finish()
    }
}

impl HFClientSync {
    /// Creates an `HFClientSync` using default configuration from the environment.
    ///
    /// Reads `HF_TOKEN`, `HF_ENDPOINT`, and other the standard environment variables.
    ///
    /// # Errors
    ///
    /// Returns an error if the tokio runtime cannot be created or if `HFClient::new` fails.
    pub fn new() -> Result<Self> {
        Ok(Self {
            inner: HFClient::new()?,
            runtime: build_runtime()?,
        })
    }

    /// Creates an `HFClientSync` wrapping an already-configured [`HFClient`].
    ///
    /// # Errors
    ///
    /// Returns an error if the tokio runtime cannot be created.
    pub fn from_api(api: HFClient) -> Result<Self> {
        Ok(Self {
            inner: api,
            runtime: build_runtime()?,
        })
    }

    /// Creates a blocking repository handle for the given repo type, owner, and name.
    pub fn repo(
        &self,
        repo_type: types::RepoType,
        owner: impl Into<String>,
        name: impl Into<String>,
    ) -> HFRepositorySync {
        HFRepositorySync::new(self.clone(), repo_type, owner, name)
    }

    /// Creates a blocking handle for a model repository.
    pub fn model(&self, owner: impl Into<String>, name: impl Into<String>) -> HFRepositorySync {
        self.repo(types::RepoType::Model, owner, name)
    }

    /// Creates a blocking handle for a dataset repository.
    pub fn dataset(&self, owner: impl Into<String>, name: impl Into<String>) -> HFRepositorySync {
        self.repo(types::RepoType::Dataset, owner, name)
    }

    /// Creates a blocking handle for a space repository.
    pub fn space(&self, owner: impl Into<String>, name: impl Into<String>) -> HFSpaceSync {
        HFSpaceSync::new(self.clone(), owner, name)
    }
}

impl Deref for HFClientSync {
    type Target = HFClient;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl HFRepositorySync {
    /// Creates a blocking repository handle from a client, repo type, owner, and name.
    pub fn new(
        client: HFClientSync,
        repo_type: types::RepoType,
        owner: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        Self {
            inner: repo::HFRepository::new(client.inner.clone(), repo_type, owner, name),
            runtime: client.runtime.clone(),
        }
    }

    pub(crate) fn from_inner(inner: repo::HFRepository, runtime: Arc<tokio::runtime::Runtime>) -> Self {
        Self { inner, runtime }
    }

    /// Returns a new handle pinned to the given revision (branch, tag, or commit SHA).
    pub fn with_revision(&self, revision: impl Into<String>) -> Self {
        Self::from_inner(self.inner.with_revision(revision), self.runtime.clone())
    }

    /// Returns a new handle with the pinned revision cleared, using the repo's default branch.
    pub fn without_revision(&self) -> Self {
        Self::from_inner(self.inner.without_revision(), self.runtime.clone())
    }
}

impl Deref for HFRepositorySync {
    type Target = repo::HFRepository;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl HFSpaceSync {
    /// Creates a blocking space handle for the given owner and name.
    pub fn new(client: HFClientSync, owner: impl Into<String>, name: impl Into<String>) -> Self {
        let owner = owner.into();
        let name = name.into();
        let inner = repo::HFSpace::new(client.inner.clone(), &owner, &name);
        let repo = HFRepositorySync::new(client.clone(), types::RepoType::Space, owner, name);
        Self { repo, inner }
    }

    /// Returns a new handle pinned to the given revision (branch, tag, or commit SHA).
    pub fn with_revision(&self, revision: impl Into<String>) -> Self {
        let rev = revision.into();
        Self {
            inner: self.inner.with_revision(&rev),
            repo: self.repo.with_revision(rev),
        }
    }

    /// Returns a new handle with the pinned revision cleared, using the space's default branch.
    pub fn without_revision(&self) -> Self {
        Self {
            inner: self.inner.without_revision(),
            repo: self.repo.without_revision(),
        }
    }

    /// Converts this space handle into a plain [`HFRepositorySync`], discarding space-specific state.
    pub fn into_repo(self) -> HFRepositorySync {
        self.repo
    }
}

impl Deref for HFSpaceSync {
    type Target = HFRepoSync;

    fn deref(&self) -> &Self::Target {
        &self.repo
    }
}

impl TryFrom<HFRepositorySync> for HFSpaceSync {
    type Error = HFError;

    fn try_from(repo: HFRepositorySync) -> Result<Self> {
        let inner = repo::HFSpace::try_from(repo.inner.clone())?;
        Ok(Self { repo, inner })
    }
}

impl From<HFSpaceSync> for HFRepositorySync {
    fn from(space: HFSpaceSync) -> Self {
        space.repo
    }
}

/// Alias for [`HFRepositorySync`].
pub type HFRepoSync = HFRepositorySync;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hfapisync_creation() {
        let sync_api = HFClientSync::new();
        assert!(sync_api.is_ok());
    }

    #[test]
    fn test_hfapisync_from_api() {
        let api = HFClient::builder().build().unwrap();
        let sync_api = HFClientSync::from_api(api);
        assert!(sync_api.is_ok());
    }

    #[test]
    fn test_sync_repo_constructors() {
        let api = HFClientSync::from_api(HFClient::builder().build().unwrap()).unwrap();
        let repo = api.model("openai-community", "gpt2").with_revision("main");
        let space = api.space("huggingface", "transformers-benchmarks");

        assert_eq!(repo.owner(), "openai-community");
        assert_eq!(repo.name(), "gpt2");
        assert_eq!(repo.default_revision(), Some("main"));
        assert_eq!(repo.repo_type(), types::RepoType::Model);
        assert_eq!(space.repo_type(), types::RepoType::Space);
    }

    #[test]
    fn test_sync_space_try_from_repo() {
        let api = HFClientSync::from_api(HFClient::builder().build().unwrap()).unwrap();
        let space_repo = api.repo(types::RepoType::Space, "owner", "space");
        assert!(HFSpaceSync::try_from(space_repo).is_ok());

        let model_repo = api.repo(types::RepoType::Model, "owner", "model");
        let error = HFSpaceSync::try_from(model_repo).unwrap_err();
        match error {
            HFError::InvalidRepoType { expected, actual } => {
                assert_eq!(expected, types::RepoType::Space);
                assert_eq!(actual, types::RepoType::Model);
            },
            _ => panic!("expected invalid repo type error"),
        }
    }
}
