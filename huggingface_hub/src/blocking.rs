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

/// Synchronous/blocking counterpart to [`HFClient`].
///
/// Wraps an `HFClient` together with a dedicated single-threaded tokio runtime so
/// that every async API method can be called from synchronous code. The runtime is
/// shared with all repo/space handles derived from this client.
#[derive(Clone)]
pub struct HfApiSync {
    pub(crate) inner: HFClient,
    pub(crate) runtime: Arc<tokio::runtime::Runtime>,
}

/// Synchronous/blocking counterpart to [`repo::HFRepository`].
///
/// Holds a reference to the underlying async handle and the shared tokio runtime.
/// All repo-scoped API methods are available directly on this type and block until
/// completion.
#[derive(Clone)]
pub struct HFRepositorySync {
    pub(crate) inner: repo::HFRepository,
    pub(crate) runtime: Arc<tokio::runtime::Runtime>,
}

/// Synchronous/blocking counterpart to [`repo::HFSpace`].
///
/// Combines an [`HFRepositorySync`] for general repo operations with an inner
/// [`repo::HFSpace`] for space-specific operations. Derefs to [`HFRepositorySync`],
/// so all blocking repository methods are accessible directly.
#[derive(Clone)]
pub struct HFSpaceSync {
    repo: HFRepositorySync,
    space: repo::HFSpace,
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
    /// Creates an `HfApiSync` using default configuration from the environment.
    ///
    /// Reads `HF_TOKEN`, `HF_ENDPOINT`, and other standard environment variables.
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

    /// Creates an `HfApiSync` wrapping an already-configured [`HFClient`].
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

    /// Returns a reference to the underlying async [`HFClient`].
    pub fn api(&self) -> &HFClient {
        &self.inner
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

impl HFRepositorySync {
    /// Creates a blocking repository handle from a client, repo type, owner, and name.
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

    /// Returns a reference to the underlying async [`repo::HFRepository`].
    pub fn repo(&self) -> &repo::HFRepository {
        &self.inner
    }

    /// Returns the [`HfApiSync`] client this handle belongs to.
    pub fn api(&self) -> HfApiSync {
        HfApiSync {
            inner: self.inner.client().clone(),
            runtime: self.runtime.clone(),
        }
    }

    /// Returns the repository owner (user or organization name).
    pub fn owner(&self) -> &str {
        self.inner.owner()
    }

    /// Returns the repository name.
    pub fn name(&self) -> &str {
        self.inner.name()
    }

    /// Returns the `{owner}/{name}` path string used in Hub API URLs.
    pub fn repo_path(&self) -> String {
        self.inner.repo_path()
    }

    /// Returns the repository type (model, dataset, or space).
    pub fn repo_type(&self) -> types::RepoType {
        self.inner.repo_type()
    }

    /// Returns the pinned revision, if one was set via [`with_revision`](Self::with_revision).
    pub fn default_revision(&self) -> Option<&str> {
        self.inner.default_revision()
    }

    /// Returns a new handle pinned to the given revision (branch, tag, or commit SHA).
    pub fn with_revision(&self, revision: impl Into<String>) -> Self {
        Self::from_inner(self.inner.with_revision(revision), self.runtime.clone())
    }

    /// Returns a new handle with the pinned revision cleared, using the repo's default branch.
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
        self.runtime.block_on(self.inner.revision_exists(&types::RevisionExistsParams {
            repo_id: self.inner.repo_path(),
            revision: params.revision.clone(),
            repo_type: Some(self.inner.repo_type()),
        }))
    }

    pub fn file_exists(&self, params: &repo::RepoFileExistsParams) -> Result<bool> {
        self.runtime.block_on(self.inner.file_exists(&types::FileExistsParams {
            repo_id: self.inner.repo_path(),
            filename: params.filename.clone(),
            revision: params.revision.clone(),
            repo_type: Some(self.inner.repo_type()),
        }))
    }

    pub fn list_files(&self, params: &repo::RepoListFilesParams) -> Result<Vec<String>> {
        self.runtime.block_on(self.inner.list_files(params))
    }

    pub fn list_tree(&self, params: &repo::RepoListTreeParams) -> Result<Vec<types::RepoTreeEntry>> {
        collect_stream(self.runtime.as_ref(), self.inner.list_tree(params)?)
    }

    pub fn get_paths_info(&self, params: &repo::RepoGetPathsInfoParams) -> Result<Vec<types::RepoTreeEntry>> {
        self.runtime.block_on(self.inner.get_paths_info(&types::GetPathsInfoParams {
            repo_id: self.inner.repo_path(),
            paths: params.paths.clone(),
            revision: params.revision.clone(),
            repo_type: Some(self.inner.repo_type()),
        }))
    }

    pub fn download_file(&self, params: &repo::RepoDownloadFileParams) -> Result<std::path::PathBuf> {
        self.runtime.block_on(self.inner.download_file(&types::DownloadFileParams {
            repo_id: self.inner.repo_path(),
            filename: params.filename.clone(),
            local_dir: params.local_dir.clone(),
            repo_type: Some(self.inner.repo_type()),
            revision: self.inner.resolve_revision(params.revision.as_deref()),
            force_download: params.force_download,
            local_files_only: params.local_files_only,
        }))
    }

    pub fn download_file_stream(
        &self,
        params: &repo::RepoDownloadFileStreamParams,
    ) -> Result<(Option<u64>, Vec<bytes::Bytes>)> {
        self.runtime.block_on(async {
            let (content_length, stream) = self
                .inner
                .download_file_stream(&types::DownloadFileStreamParams {
                    repo_id: self.inner.repo_path(),
                    filename: params.filename.clone(),
                    repo_type: Some(self.inner.repo_type()),
                    revision: self.inner.resolve_revision(params.revision.as_deref()),
                })
                .await?;
            futures::pin_mut!(stream);
            let mut chunks = Vec::new();
            while let Some(chunk) = stream.next().await {
                chunks.push(chunk?);
            }
            Ok((content_length, chunks))
        })
    }

    pub fn snapshot_download(&self, params: &repo::RepoSnapshotDownloadParams) -> Result<std::path::PathBuf> {
        self.runtime
            .block_on(self.inner.snapshot_download(&types::SnapshotDownloadParams {
                repo_id: self.inner.repo_path(),
                repo_type: Some(self.inner.repo_type()),
                revision: self.inner.resolve_revision(params.revision.as_deref()),
                allow_patterns: params.allow_patterns.clone(),
                ignore_patterns: params.ignore_patterns.clone(),
                local_dir: params.local_dir.clone(),
                force_download: params.force_download,
                local_files_only: params.local_files_only,
                max_workers: params.max_workers,
            }))
    }

    pub fn create_commit(&self, params: &repo::RepoCreateCommitParams) -> Result<types::CommitInfo> {
        self.runtime.block_on(self.inner.create_commit(&types::CreateCommitParams {
            repo_id: self.inner.repo_path(),
            operations: params.operations.clone(),
            commit_message: params.commit_message.clone(),
            commit_description: params.commit_description.clone(),
            repo_type: Some(self.inner.repo_type()),
            revision: self.inner.resolve_revision(params.revision.as_deref()),
            create_pr: params.create_pr,
            parent_commit: params.parent_commit.clone(),
        }))
    }

    pub fn upload_file(&self, params: &repo::RepoUploadFileParams) -> Result<types::CommitInfo> {
        self.runtime.block_on(self.inner.upload_file(&types::UploadFileParams {
            repo_id: self.inner.repo_path(),
            source: params.source.clone(),
            path_in_repo: params.path_in_repo.clone(),
            repo_type: Some(self.inner.repo_type()),
            revision: self.inner.resolve_revision(params.revision.as_deref()),
            commit_message: params.commit_message.clone(),
            commit_description: params.commit_description.clone(),
            create_pr: params.create_pr,
            parent_commit: params.parent_commit.clone(),
        }))
    }

    pub fn upload_folder(&self, params: &repo::RepoUploadFolderParams) -> Result<types::CommitInfo> {
        self.runtime.block_on(self.inner.upload_folder(&types::UploadFolderParams {
            repo_id: self.inner.repo_path(),
            folder_path: params.folder_path.clone(),
            path_in_repo: params.path_in_repo.clone(),
            repo_type: Some(self.inner.repo_type()),
            revision: self.inner.resolve_revision(params.revision.as_deref()),
            commit_message: params.commit_message.clone(),
            commit_description: params.commit_description.clone(),
            create_pr: params.create_pr,
            allow_patterns: params.allow_patterns.clone(),
            ignore_patterns: params.ignore_patterns.clone(),
            delete_patterns: params.delete_patterns.clone(),
        }))
    }

    pub fn delete_file(&self, params: &repo::RepoDeleteFileParams) -> Result<types::CommitInfo> {
        self.runtime.block_on(self.inner.delete_file(&types::DeleteFileParams {
            repo_id: self.inner.repo_path(),
            path_in_repo: params.path_in_repo.clone(),
            repo_type: Some(self.inner.repo_type()),
            revision: self.inner.resolve_revision(params.revision.as_deref()),
            commit_message: params.commit_message.clone(),
            create_pr: params.create_pr,
        }))
    }

    pub fn delete_folder(&self, params: &repo::RepoDeleteFolderParams) -> Result<types::CommitInfo> {
        self.runtime.block_on(self.inner.delete_folder(&types::DeleteFolderParams {
            repo_id: self.inner.repo_path(),
            path_in_repo: params.path_in_repo.clone(),
            repo_type: Some(self.inner.repo_type()),
            revision: self.inner.resolve_revision(params.revision.as_deref()),
            commit_message: params.commit_message.clone(),
            create_pr: params.create_pr,
        }))
    }

    pub fn list_commits(&self, params: &repo::RepoListCommitsParams) -> Result<Vec<types::GitCommitInfo>> {
        collect_stream(self.runtime.as_ref(), self.inner.list_commits(params)?)
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
        self.runtime
            .block_on(self.inner.create_discussion(&types::CreateDiscussionParams {
                repo_id: self.inner.repo_path(),
                title: params.title.clone(),
                description: params.description.clone(),
                repo_type: Some(self.inner.repo_type()),
            }))
    }

    #[cfg(feature = "discussions")]
    pub fn create_pull_request(
        &self,
        params: &repo::RepoCreatePullRequestParams,
    ) -> Result<types::DiscussionWithDetails> {
        self.runtime
            .block_on(self.inner.create_pull_request(&types::CreatePullRequestParams {
                repo_id: self.inner.repo_path(),
                title: params.title.clone(),
                description: params.description.clone(),
                repo_type: Some(self.inner.repo_type()),
            }))
    }

    #[cfg(feature = "discussions")]
    pub fn comment_discussion(&self, params: &repo::RepoCommentDiscussionParams) -> Result<types::DiscussionComment> {
        self.runtime
            .block_on(self.inner.comment_discussion(&types::CommentDiscussionParams {
                repo_id: self.inner.repo_path(),
                discussion_num: params.discussion_num,
                comment: params.comment.clone(),
                repo_type: Some(self.inner.repo_type()),
            }))
    }

    #[cfg(feature = "discussions")]
    pub fn edit_discussion_comment(
        &self,
        params: &repo::RepoEditDiscussionCommentParams,
    ) -> Result<types::DiscussionComment> {
        self.runtime
            .block_on(self.inner.edit_discussion_comment(&types::EditDiscussionCommentParams {
                repo_id: self.inner.repo_path(),
                discussion_num: params.discussion_num,
                comment_id: params.comment_id.clone(),
                new_content: params.new_content.clone(),
                repo_type: Some(self.inner.repo_type()),
            }))
    }

    #[cfg(feature = "discussions")]
    pub fn hide_discussion_comment(
        &self,
        params: &repo::RepoHideDiscussionCommentParams,
    ) -> Result<types::DiscussionComment> {
        self.runtime
            .block_on(self.inner.hide_discussion_comment(&types::HideDiscussionCommentParams {
                repo_id: self.inner.repo_path(),
                discussion_num: params.discussion_num,
                comment_id: params.comment_id.clone(),
                repo_type: Some(self.inner.repo_type()),
            }))
    }

    #[cfg(feature = "discussions")]
    pub fn rename_discussion(&self, params: &repo::RepoRenameDiscussionParams) -> Result<types::DiscussionWithDetails> {
        self.runtime
            .block_on(self.inner.rename_discussion(&types::RenameDiscussionParams {
                repo_id: self.inner.repo_path(),
                discussion_num: params.discussion_num,
                new_title: params.new_title.clone(),
                repo_type: Some(self.inner.repo_type()),
            }))
    }

    #[cfg(feature = "discussions")]
    pub fn change_discussion_status(
        &self,
        params: &repo::RepoChangeDiscussionStatusParams,
    ) -> Result<types::DiscussionWithDetails> {
        self.runtime
            .block_on(self.inner.change_discussion_status(&types::ChangeDiscussionStatusParams {
                repo_id: self.inner.repo_path(),
                discussion_num: params.discussion_num,
                new_status: params.new_status.clone(),
                repo_type: Some(self.inner.repo_type()),
            }))
    }

    #[cfg(feature = "discussions")]
    pub fn merge_pull_request(
        &self,
        params: &repo::RepoMergePullRequestParams,
    ) -> Result<types::DiscussionWithDetails> {
        self.runtime
            .block_on(self.inner.merge_pull_request(&types::MergePullRequestParams {
                repo_id: self.inner.repo_path(),
                discussion_num: params.discussion_num,
                repo_type: Some(self.inner.repo_type()),
            }))
    }

    #[cfg(feature = "access_requests")]
    pub fn list_pending_access_requests(&self) -> Result<Vec<types::AccessRequest>> {
        self.runtime
            .block_on(self.inner.list_pending_access_requests(&types::ListAccessRequestsParams {
                repo_id: self.inner.repo_path(),
                repo_type: Some(self.inner.repo_type()),
            }))
    }

    #[cfg(feature = "access_requests")]
    pub fn list_accepted_access_requests(&self) -> Result<Vec<types::AccessRequest>> {
        self.runtime
            .block_on(self.inner.list_accepted_access_requests(&types::ListAccessRequestsParams {
                repo_id: self.inner.repo_path(),
                repo_type: Some(self.inner.repo_type()),
            }))
    }

    #[cfg(feature = "access_requests")]
    pub fn list_rejected_access_requests(&self) -> Result<Vec<types::AccessRequest>> {
        self.runtime
            .block_on(self.inner.list_rejected_access_requests(&types::ListAccessRequestsParams {
                repo_id: self.inner.repo_path(),
                repo_type: Some(self.inner.repo_type()),
            }))
    }

    #[cfg(feature = "access_requests")]
    pub fn accept_access_request(&self, params: &repo::RepoAccessRequestUserParams) -> Result<()> {
        self.runtime
            .block_on(self.inner.accept_access_request(&types::HandleAccessRequestParams {
                repo_id: self.inner.repo_path(),
                user: params.user.clone(),
                repo_type: Some(self.inner.repo_type()),
            }))
    }

    #[cfg(feature = "access_requests")]
    pub fn reject_access_request(&self, params: &repo::RepoAccessRequestUserParams) -> Result<()> {
        self.runtime
            .block_on(self.inner.reject_access_request(&types::HandleAccessRequestParams {
                repo_id: self.inner.repo_path(),
                user: params.user.clone(),
                repo_type: Some(self.inner.repo_type()),
            }))
    }

    #[cfg(feature = "access_requests")]
    pub fn cancel_access_request(&self, params: &repo::RepoAccessRequestUserParams) -> Result<()> {
        self.runtime
            .block_on(self.inner.cancel_access_request(&types::HandleAccessRequestParams {
                repo_id: self.inner.repo_path(),
                user: params.user.clone(),
                repo_type: Some(self.inner.repo_type()),
            }))
    }

    #[cfg(feature = "access_requests")]
    pub fn grant_access(&self, params: &repo::RepoAccessRequestUserParams) -> Result<()> {
        self.runtime.block_on(self.inner.grant_access(&types::GrantAccessParams {
            repo_id: self.inner.repo_path(),
            user: params.user.clone(),
            repo_type: Some(self.inner.repo_type()),
        }))
    }

    #[cfg(feature = "likes")]
    pub fn like(&self) -> Result<()> {
        self.runtime.block_on(self.inner.like(&types::LikeParams {
            repo_id: self.inner.repo_path(),
            repo_type: Some(self.inner.repo_type()),
        }))
    }

    #[cfg(feature = "likes")]
    pub fn unlike(&self) -> Result<()> {
        self.runtime.block_on(self.inner.unlike(&types::LikeParams {
            repo_id: self.inner.repo_path(),
            repo_type: Some(self.inner.repo_type()),
        }))
    }

    #[cfg(feature = "likes")]
    pub fn list_likers(&self, max_items: Option<usize>) -> Result<Vec<types::User>> {
        collect_stream(self.runtime.as_ref(), self.inner.list_likers(max_items)?)
    }
}

impl HFSpaceSync {
    /// Creates a blocking space handle for the given owner and name.
    pub fn new(client: HfApiSync, owner: impl Into<String>, name: impl Into<String>) -> Self {
        let owner = owner.into();
        let name = name.into();
        let space = repo::HFSpace::new(client.inner.clone(), &owner, &name);
        let repo = HFRepositorySync::new(client, types::RepoType::Space, owner, name);
        Self { repo, space }
    }

    /// Returns a clone of the underlying async [`repo::HFSpace`] handle.
    pub fn space(&self) -> repo::HFSpace {
        self.space.clone()
    }

    /// Returns the [`HfApiSync`] client this handle belongs to.
    pub fn api(&self) -> HfApiSync {
        self.repo.api()
    }

    /// Returns a new handle pinned to the given revision (branch, tag, or commit SHA).
    pub fn with_revision(&self, revision: impl Into<String>) -> Self {
        let rev = revision.into();
        Self {
            space: self.space.with_revision(&rev),
            repo: self.repo.with_revision(rev),
        }
    }

    /// Returns a new handle with the pinned revision cleared, using the space's default branch.
    pub fn without_revision(&self) -> Self {
        Self {
            space: self.space.without_revision(),
            repo: self.repo.without_revision(),
        }
    }

    /// Converts this space handle into a plain [`HFRepositorySync`], discarding space-specific state.
    pub fn into_repo(self) -> HFRepositorySync {
        self.repo
    }

    #[cfg(feature = "spaces")]
    pub fn runtime(&self) -> Result<types::SpaceRuntime> {
        self.repo.runtime.block_on(self.space.clone().runtime())
    }

    #[cfg(feature = "spaces")]
    pub fn request_hardware(&self, params: &repo::SpaceHardwareRequestParams) -> Result<types::SpaceRuntime> {
        self.repo.runtime.block_on(self.space.clone().request_hardware(params))
    }

    #[cfg(feature = "spaces")]
    pub fn set_sleep_time(&self, params: &repo::SpaceSleepTimeParams) -> Result<()> {
        self.repo.runtime.block_on(self.space.clone().set_sleep_time(params))
    }

    #[cfg(feature = "spaces")]
    pub fn pause(&self) -> Result<types::SpaceRuntime> {
        self.repo.runtime.block_on(self.space.clone().pause())
    }

    #[cfg(feature = "spaces")]
    pub fn restart(&self) -> Result<types::SpaceRuntime> {
        self.repo.runtime.block_on(self.space.clone().restart())
    }

    #[cfg(feature = "spaces")]
    pub fn add_secret(&self, params: &repo::SpaceSecretParams) -> Result<()> {
        self.repo.runtime.block_on(self.space.clone().add_secret(params))
    }

    #[cfg(feature = "spaces")]
    pub fn delete_secret(&self, params: &repo::SpaceSecretDeleteParams) -> Result<()> {
        self.repo.runtime.block_on(self.space.clone().delete_secret(params))
    }

    #[cfg(feature = "spaces")]
    pub fn add_variable(&self, params: &repo::SpaceVariableParams) -> Result<()> {
        self.repo.runtime.block_on(self.space.clone().add_variable(params))
    }

    #[cfg(feature = "spaces")]
    pub fn delete_variable(&self, params: &repo::SpaceVariableDeleteParams) -> Result<()> {
        self.repo.runtime.block_on(self.space.clone().delete_variable(params))
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
        let space = repo::HFSpace::try_from(repo.inner.clone())?;
        Ok(Self { repo, space })
    }
}

impl From<HFSpaceSync> for HFRepositorySync {
    fn from(space: HFSpaceSync) -> Self {
        space.repo
    }
}

/// Alias for [`HfApiSync`].
pub type HFClientSync = HfApiSync;
/// Alias for [`HfApiSync`].
pub type HfClientSync = HFClientSync;
/// Alias for [`HFRepositorySync`].
pub type HFRepoSync = HFRepositorySync;
/// Alias for [`HFRepositorySync`].
pub type HfRepositorySync = HFRepositorySync;
/// Alias for [`HFRepositorySync`].
pub type HfRepoSync = HFRepoSync;
/// Alias for [`HFSpaceSync`].
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
