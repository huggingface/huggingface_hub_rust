use std::fmt;
use std::ops::Deref;
use std::sync::Arc;

use futures::{Stream, StreamExt};

use crate::client::HFClient;
use crate::error::{HfError, Result};
use crate::{repository as repo, types};

fn build_runtime() -> Result<Arc<tokio::runtime::Runtime>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map(Arc::new)
        .map_err(|e| HfError::Other(format!("Failed to create tokio runtime: {e}")))
}

fn collect_stream<T, S>(runtime: &tokio::runtime::Runtime, stream: S) -> Result<Vec<T>>
where
    S: Stream<Item = Result<T>>,
{
    runtime.block_on(async move {
        futures::pin_mut!(stream);
        let mut items = Vec::new();
        while let Some(item) = stream.next().await {
            items.push(item?);
        }
        Ok(items)
    })
}

#[derive(Clone)]
pub struct HfApiSync {
    pub(crate) inner: HFClient,
    pub(crate) runtime: Arc<tokio::runtime::Runtime>,
}

#[derive(Clone)]
pub struct HFRepositorySync {
    pub(crate) inner: repo::HFRepository,
    pub(crate) runtime: Arc<tokio::runtime::Runtime>,
}

#[derive(Clone)]
pub struct HFSpaceSync {
    repo: HFRepositorySync,
}

impl fmt::Debug for HfApiSync {
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
        f.debug_struct("HFSpaceSync").field("repo", &self.repo).finish()
    }
}

impl HfApiSync {
    pub fn new() -> Result<Self> {
        Ok(Self {
            inner: HFClient::new()?,
            runtime: build_runtime()?,
        })
    }

    pub fn from_api(api: HFClient) -> Result<Self> {
        Ok(Self {
            inner: api,
            runtime: build_runtime()?,
        })
    }

    pub fn api(&self) -> &HFClient {
        &self.inner
    }

    pub fn repo(
        &self,
        repo_type: types::RepoType,
        owner: impl Into<String>,
        name: impl Into<String>,
    ) -> HFRepositorySync {
        HFRepositorySync::new(self.clone(), repo_type, owner, name)
    }

    pub fn model(&self, owner: impl Into<String>, name: impl Into<String>) -> HFRepositorySync {
        self.repo(types::RepoType::Model, owner, name)
    }

    pub fn dataset(&self, owner: impl Into<String>, name: impl Into<String>) -> HFRepositorySync {
        self.repo(types::RepoType::Dataset, owner, name)
    }

    pub fn space(&self, owner: impl Into<String>, name: impl Into<String>) -> HFSpaceSync {
        HFSpaceSync::new(self.clone(), owner, name)
    }
}

impl HFRepositorySync {
    pub fn new(
        client: HfApiSync,
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

    pub fn repo(&self) -> &repo::HFRepository {
        &self.inner
    }

    pub fn api(&self) -> HfApiSync {
        HfApiSync {
            inner: self.inner.client().clone(),
            runtime: self.runtime.clone(),
        }
    }

    pub fn owner(&self) -> &str {
        self.inner.owner()
    }

    pub fn name(&self) -> &str {
        self.inner.name()
    }

    pub fn repo_path(&self) -> String {
        self.inner.repo_path()
    }

    pub fn repo_type(&self) -> types::RepoType {
        self.inner.repo_type()
    }

    pub fn default_revision(&self) -> Option<&str> {
        self.inner.default_revision()
    }

    pub fn with_revision(&self, revision: impl Into<String>) -> Self {
        Self::from_inner(self.inner.with_revision(revision), self.runtime.clone())
    }

    pub fn without_revision(&self) -> Self {
        Self::from_inner(self.inner.without_revision(), self.runtime.clone())
    }

    pub fn info(&self, params: &repo::RepoInfoParams) -> Result<types::RepoInfo> {
        self.runtime.block_on(self.inner.info(params))
    }

    pub fn exists(&self) -> Result<bool> {
        self.runtime.block_on(self.inner.exists())
    }

    pub fn revision_exists(&self, params: &repo::RepoRevisionExistsParams) -> Result<bool> {
        self.runtime.block_on(self.inner.revision_exists(params))
    }

    pub fn file_exists(&self, params: &repo::RepoFileExistsParams) -> Result<bool> {
        self.runtime.block_on(self.inner.file_exists(params))
    }

    pub fn list_files(&self, params: &repo::RepoListFilesParams) -> Result<Vec<String>> {
        self.runtime.block_on(self.inner.list_files(params))
    }

    pub fn list_tree(&self, params: &repo::RepoListTreeParams) -> Result<Vec<types::RepoTreeEntry>> {
        collect_stream(self.runtime.as_ref(), self.inner.list_tree(params))
    }

    pub fn get_paths_info(&self, params: &repo::RepoGetPathsInfoParams) -> Result<Vec<types::RepoTreeEntry>> {
        self.runtime.block_on(self.inner.get_paths_info(params))
    }

    pub fn download_file(&self, params: &repo::RepoDownloadFileParams) -> Result<std::path::PathBuf> {
        self.runtime.block_on(self.inner.download_file(params))
    }

    pub fn download_file_stream(
        &self,
        params: &repo::RepoDownloadFileStreamParams,
    ) -> Result<(Option<u64>, Vec<bytes::Bytes>)> {
        self.runtime.block_on(async {
            let (content_length, stream) = self.inner.download_file_stream(params).await?;
            futures::pin_mut!(stream);
            let mut chunks = Vec::new();
            while let Some(chunk) = stream.next().await {
                chunks.push(chunk?);
            }
            Ok((content_length, chunks))
        })
    }

    pub fn snapshot_download(&self, params: &repo::RepoSnapshotDownloadParams) -> Result<std::path::PathBuf> {
        self.runtime.block_on(self.inner.snapshot_download(params))
    }

    pub fn create_commit(&self, params: &repo::RepoCreateCommitParams) -> Result<types::CommitInfo> {
        self.runtime.block_on(self.inner.create_commit(params))
    }

    pub fn upload_file(&self, params: &repo::RepoUploadFileParams) -> Result<types::CommitInfo> {
        self.runtime.block_on(self.inner.upload_file(params))
    }

    pub fn upload_folder(&self, params: &repo::RepoUploadFolderParams) -> Result<types::CommitInfo> {
        self.runtime.block_on(self.inner.upload_folder(params))
    }

    pub fn delete_file(&self, params: &repo::RepoDeleteFileParams) -> Result<types::CommitInfo> {
        self.runtime.block_on(self.inner.delete_file(params))
    }

    pub fn delete_folder(&self, params: &repo::RepoDeleteFolderParams) -> Result<types::CommitInfo> {
        self.runtime.block_on(self.inner.delete_folder(params))
    }

    pub fn list_commits(&self, params: &repo::RepoListCommitsParams) -> Result<Vec<types::GitCommitInfo>> {
        collect_stream(self.runtime.as_ref(), self.inner.list_commits(params))
    }

    pub fn list_refs(&self, params: &repo::RepoListRefsParams) -> Result<types::GitRefs> {
        self.runtime.block_on(self.inner.list_refs(params))
    }

    pub fn get_commit_diff(&self, params: &repo::RepoGetCommitDiffParams) -> Result<String> {
        self.runtime.block_on(self.inner.get_commit_diff(params))
    }

    pub fn get_raw_diff(&self, params: &repo::RepoGetRawDiffParams) -> Result<String> {
        self.runtime.block_on(self.inner.get_raw_diff(params))
    }

    pub fn create_branch(&self, params: &repo::RepoCreateBranchParams) -> Result<()> {
        self.runtime.block_on(self.inner.create_branch(params))
    }

    pub fn delete_branch(&self, params: &repo::RepoDeleteBranchParams) -> Result<()> {
        self.runtime.block_on(self.inner.delete_branch(params))
    }

    pub fn create_tag(&self, params: &repo::RepoCreateTagParams) -> Result<()> {
        self.runtime.block_on(self.inner.create_tag(params))
    }

    pub fn delete_tag(&self, params: &repo::RepoDeleteTagParams) -> Result<()> {
        self.runtime.block_on(self.inner.delete_tag(params))
    }

    pub fn update_settings(&self, params: &repo::RepoUpdateSettingsParams) -> Result<()> {
        self.runtime.block_on(self.inner.update_settings(params))
    }

    #[cfg(feature = "discussions")]
    pub fn list_discussions(&self, params: &repo::RepoListDiscussionsParams) -> Result<types::DiscussionsResponse> {
        self.runtime.block_on(self.inner.list_discussions(params))
    }

    #[cfg(feature = "discussions")]
    pub fn discussion_details(
        &self,
        params: &repo::RepoDiscussionDetailsParams,
    ) -> Result<types::DiscussionWithDetails> {
        self.runtime.block_on(self.inner.discussion_details(params))
    }

    #[cfg(feature = "discussions")]
    pub fn create_discussion(&self, params: &repo::RepoCreateDiscussionParams) -> Result<types::DiscussionWithDetails> {
        self.runtime.block_on(self.inner.create_discussion(params))
    }

    #[cfg(feature = "discussions")]
    pub fn create_pull_request(
        &self,
        params: &repo::RepoCreatePullRequestParams,
    ) -> Result<types::DiscussionWithDetails> {
        self.runtime.block_on(self.inner.create_pull_request(params))
    }

    #[cfg(feature = "discussions")]
    pub fn comment_discussion(&self, params: &repo::RepoCommentDiscussionParams) -> Result<types::DiscussionComment> {
        self.runtime.block_on(self.inner.comment_discussion(params))
    }

    #[cfg(feature = "discussions")]
    pub fn edit_discussion_comment(
        &self,
        params: &repo::RepoEditDiscussionCommentParams,
    ) -> Result<types::DiscussionComment> {
        self.runtime.block_on(self.inner.edit_discussion_comment(params))
    }

    #[cfg(feature = "discussions")]
    pub fn hide_discussion_comment(
        &self,
        params: &repo::RepoHideDiscussionCommentParams,
    ) -> Result<types::DiscussionComment> {
        self.runtime.block_on(self.inner.hide_discussion_comment(params))
    }

    #[cfg(feature = "discussions")]
    pub fn rename_discussion(&self, params: &repo::RepoRenameDiscussionParams) -> Result<types::DiscussionWithDetails> {
        self.runtime.block_on(self.inner.rename_discussion(params))
    }

    #[cfg(feature = "discussions")]
    pub fn change_discussion_status(
        &self,
        params: &repo::RepoChangeDiscussionStatusParams,
    ) -> Result<types::DiscussionWithDetails> {
        self.runtime.block_on(self.inner.change_discussion_status(params))
    }

    #[cfg(feature = "discussions")]
    pub fn merge_pull_request(
        &self,
        params: &repo::RepoMergePullRequestParams,
    ) -> Result<types::DiscussionWithDetails> {
        self.runtime.block_on(self.inner.merge_pull_request(params))
    }

    #[cfg(feature = "access_requests")]
    pub fn list_pending_access_requests(&self) -> Result<Vec<types::AccessRequest>> {
        self.runtime.block_on(self.inner.list_pending_access_requests())
    }

    #[cfg(feature = "access_requests")]
    pub fn list_accepted_access_requests(&self) -> Result<Vec<types::AccessRequest>> {
        self.runtime.block_on(self.inner.list_accepted_access_requests())
    }

    #[cfg(feature = "access_requests")]
    pub fn list_rejected_access_requests(&self) -> Result<Vec<types::AccessRequest>> {
        self.runtime.block_on(self.inner.list_rejected_access_requests())
    }

    #[cfg(feature = "access_requests")]
    pub fn accept_access_request(&self, params: &repo::RepoAccessRequestUserParams) -> Result<()> {
        self.runtime.block_on(self.inner.accept_access_request(params))
    }

    #[cfg(feature = "access_requests")]
    pub fn reject_access_request(&self, params: &repo::RepoAccessRequestUserParams) -> Result<()> {
        self.runtime.block_on(self.inner.reject_access_request(params))
    }

    #[cfg(feature = "access_requests")]
    pub fn cancel_access_request(&self, params: &repo::RepoAccessRequestUserParams) -> Result<()> {
        self.runtime.block_on(self.inner.cancel_access_request(params))
    }

    #[cfg(feature = "access_requests")]
    pub fn grant_access(&self, params: &repo::RepoAccessRequestUserParams) -> Result<()> {
        self.runtime.block_on(self.inner.grant_access(params))
    }

    #[cfg(feature = "likes")]
    pub fn like(&self) -> Result<()> {
        self.runtime.block_on(self.inner.like())
    }

    #[cfg(feature = "likes")]
    pub fn unlike(&self) -> Result<()> {
        self.runtime.block_on(self.inner.unlike())
    }

    #[cfg(feature = "likes")]
    pub fn list_likers(&self) -> Result<Vec<types::User>> {
        collect_stream(self.runtime.as_ref(), self.inner.list_likers())
    }
}

impl HFSpaceSync {
    pub fn new(client: HfApiSync, owner: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            repo: HFRepositorySync::new(client, types::RepoType::Space, owner, name),
        }
    }

    fn from_repo(repo: HFRepositorySync) -> Self {
        Self { repo }
    }

    fn inner_space(&self) -> repo::HFSpace {
        repo::HFSpace::try_from(self.repo.inner.clone()).expect("HFSpaceSync invariant violated")
    }

    pub fn space(&self) -> repo::HFSpace {
        self.inner_space()
    }

    pub fn api(&self) -> HfApiSync {
        self.repo.api()
    }

    pub fn with_revision(&self, revision: impl Into<String>) -> Self {
        Self::from_repo(self.repo.with_revision(revision))
    }

    pub fn without_revision(&self) -> Self {
        Self::from_repo(self.repo.without_revision())
    }

    pub fn into_repo(self) -> HFRepositorySync {
        self.repo
    }

    #[cfg(feature = "spaces")]
    pub fn runtime(&self) -> Result<types::SpaceRuntime> {
        self.repo.runtime.block_on(self.inner_space().runtime())
    }

    #[cfg(feature = "spaces")]
    pub fn request_hardware(&self, params: &repo::SpaceHardwareRequestParams) -> Result<types::SpaceRuntime> {
        self.repo.runtime.block_on(self.inner_space().request_hardware(params))
    }

    #[cfg(feature = "spaces")]
    pub fn set_sleep_time(&self, params: &repo::SpaceSleepTimeParams) -> Result<()> {
        self.repo.runtime.block_on(self.inner_space().set_sleep_time(params))
    }

    #[cfg(feature = "spaces")]
    pub fn pause(&self) -> Result<types::SpaceRuntime> {
        self.repo.runtime.block_on(self.inner_space().pause())
    }

    #[cfg(feature = "spaces")]
    pub fn restart(&self) -> Result<types::SpaceRuntime> {
        self.repo.runtime.block_on(self.inner_space().restart())
    }

    #[cfg(feature = "spaces")]
    pub fn add_secret(&self, params: &repo::SpaceSecretParams) -> Result<()> {
        self.repo.runtime.block_on(self.inner_space().add_secret(params))
    }

    #[cfg(feature = "spaces")]
    pub fn delete_secret(&self, params: &repo::SpaceSecretDeleteParams) -> Result<()> {
        self.repo.runtime.block_on(self.inner_space().delete_secret(params))
    }

    #[cfg(feature = "spaces")]
    pub fn add_variable(&self, params: &repo::SpaceVariableParams) -> Result<()> {
        self.repo.runtime.block_on(self.inner_space().add_variable(params))
    }

    #[cfg(feature = "spaces")]
    pub fn delete_variable(&self, params: &repo::SpaceVariableDeleteParams) -> Result<()> {
        self.repo.runtime.block_on(self.inner_space().delete_variable(params))
    }
}

impl Deref for HFSpaceSync {
    type Target = HFRepoSync;

    fn deref(&self) -> &Self::Target {
        &self.repo
    }
}

impl TryFrom<HFRepositorySync> for HFSpaceSync {
    type Error = HfError;

    fn try_from(repo: HFRepositorySync) -> Result<Self> {
        let _ = repo::HFSpace::try_from(repo.inner.clone())?;
        Ok(Self::from_repo(repo))
    }
}

impl From<HFSpaceSync> for HFRepositorySync {
    fn from(space: HFSpaceSync) -> Self {
        space.repo
    }
}

pub type HFClientSync = HfApiSync;
pub type HfClientSync = HFClientSync;
pub type HFRepoSync = HFRepositorySync;
pub type HfRepositorySync = HFRepositorySync;
pub type HfRepoSync = HFRepoSync;
pub type HfSpaceSync = HFSpaceSync;

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
        let api = HFClient::builder().build().unwrap();
        let sync_api = HfApiSync::from_api(api);
        assert!(sync_api.is_ok());
    }

    #[test]
    fn test_sync_repo_constructors() {
        let api = HfApiSync::from_api(HFClient::builder().build().unwrap()).unwrap();
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
        let api = HfApiSync::from_api(HFClient::builder().build().unwrap()).unwrap();
        let space_repo = api.repo(types::RepoType::Space, "owner", "space");
        assert!(HFSpaceSync::try_from(space_repo).is_ok());

        let model_repo = api.repo(types::RepoType::Model, "owner", "model");
        let error = HFSpaceSync::try_from(model_repo).unwrap_err();
        match error {
            HfError::InvalidRepoType { expected, actual } => {
                assert_eq!(expected, types::RepoType::Space);
                assert_eq!(actual, types::RepoType::Model);
            },
            _ => panic!("expected invalid repo type error"),
        }
    }
}
