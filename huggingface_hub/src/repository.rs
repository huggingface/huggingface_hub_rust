use std::fmt;
use std::ops::Deref;
use std::path::PathBuf;

use futures::Stream;
use typed_builder::TypedBuilder;
use url::Url;

use crate::client::HFClient;
use crate::constants;
use crate::error::{HfError, Result};
use crate::types::{
    AddSource, CommitOperation, CreateBranchParams, CreateTagParams, DatasetInfoParams, DeleteBranchParams,
    DeleteTagParams, GetCommitDiffParams, GetRawDiffParams, GitCommitInfo, GitRefs, ListRepoFilesParams,
    ListRepoRefsParams, ModelInfoParams, RepoInfo, RepoTreeEntry, RepoType, SpaceInfoParams, UpdateRepoParams,
};
#[cfg(feature = "discussions")]
use crate::types::{DiscussionWithDetails, DiscussionsResponse, GetDiscussionDetailsParams, GetRepoDiscussionsParams};
#[cfg(feature = "likes")]
use crate::types::{ListRepoLikersParams, User};

/// A handle for a single repository on the Hugging Face Hub.
///
/// `HFRepository` is created via [`HFClient::repo`], [`HFClient::model`], or
/// [`HFClient::dataset`] and binds together the client, owner, repo name, and repo type.
/// All repo-scoped API operations are methods on this type.
///
/// Cheap to clone — the inner [`HFClient`] is `Arc`-backed.
///
/// # Example
///
/// ```rust,no_run
/// # use huggingface_hub::{HFClient, types::RepoType};
/// # #[tokio::main] async fn main() -> huggingface_hub::error::Result<()> {
/// let client = HFClient::builder().build()?;
/// let repo = client.model("openai-community", "gpt2");
/// let info = repo.info(&Default::default()).await?;
/// # Ok(()) }
/// ```
#[derive(Clone)]
pub struct HFRepository {
    pub(crate) client: HFClient,
    owner: String,
    name: String,
    pub(crate) repo_type: RepoType,
    default_revision: Option<String>,
}

/// Alias for [`HFRepository`].
pub type HFRepo = HFRepository;
/// Alias for [`HFRepository`].
pub type HfRepository = HFRepository;
/// Alias for [`HFRepository`].
pub type HfRepo = HFRepo;

/// A handle for a Space repository, providing Space-specific operations on top of [`HFRepository`].
///
/// `HFSpace` wraps an [`HFRepository`] fixed to [`RepoType::Space`] and exposes hardware,
/// secret, and variable management. It derefs to [`HFRepository`], so all general repo
/// methods are accessible directly.
///
/// Created via [`HFClient::space`] or [`TryFrom<HFRepository>`].
///
/// # Example
///
/// ```rust,no_run
/// # use huggingface_hub::HFClient;
/// # #[tokio::main] async fn main() -> huggingface_hub::error::Result<()> {
/// let client = HFClient::builder().build()?;
/// let space = client.space("huggingface", "diffusers-gallery");
/// // General repo methods are available via Deref:
/// let exists = space.exists().await?;
/// # Ok(()) }
/// ```
#[derive(Clone)]
pub struct HFSpace {
    repo: HFRepository,
}

/// Alias for [`HFSpace`].
pub type HfSpace = HFSpace;

impl fmt::Debug for HFRepository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HFRepository")
            .field("owner", &self.owner)
            .field("name", &self.name)
            .field("repo_type", &self.repo_type)
            .field("default_revision", &self.default_revision)
            .finish()
    }
}

impl fmt::Debug for HFSpace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HFSpace").field("repo", &self.repo).finish()
    }
}

#[derive(Default, TypedBuilder)]
pub struct RepoInfoParams {
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
}

#[derive(TypedBuilder)]
pub struct RepoRevisionExistsParams {
    #[builder(setter(into))]
    pub revision: String,
}

#[derive(TypedBuilder)]
pub struct RepoFileExistsParams {
    #[builder(setter(into))]
    pub filename: String,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
}

#[derive(Default, TypedBuilder)]
pub struct RepoListFilesParams {
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
}

#[derive(Default, TypedBuilder)]
pub struct RepoListTreeParams {
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default)]
    pub recursive: bool,
    #[builder(default)]
    pub expand: bool,
    #[builder(default, setter(strip_option))]
    pub max_items: Option<usize>,
}

#[derive(TypedBuilder)]
pub struct RepoGetPathsInfoParams {
    pub paths: Vec<String>,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
}

#[derive(TypedBuilder)]
pub struct RepoDownloadFileParams {
    #[builder(setter(into))]
    pub filename: String,
    #[builder(default, setter(strip_option))]
    pub local_dir: Option<PathBuf>,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(strip_option))]
    pub force_download: Option<bool>,
    #[builder(default, setter(strip_option))]
    pub local_files_only: Option<bool>,
}

#[derive(TypedBuilder)]
pub struct RepoDownloadFileStreamParams {
    #[builder(setter(into))]
    pub filename: String,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
}

#[derive(Default, TypedBuilder)]
pub struct RepoSnapshotDownloadParams {
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(strip_option))]
    pub allow_patterns: Option<Vec<String>>,
    #[builder(default, setter(strip_option))]
    pub ignore_patterns: Option<Vec<String>>,
    #[builder(default, setter(strip_option))]
    pub local_dir: Option<PathBuf>,
    #[builder(default, setter(strip_option))]
    pub force_download: Option<bool>,
    #[builder(default, setter(strip_option))]
    pub local_files_only: Option<bool>,
    #[builder(default, setter(strip_option))]
    pub max_workers: Option<usize>,
}

#[derive(TypedBuilder)]
pub struct RepoUploadFileParams {
    pub source: AddSource,
    #[builder(setter(into))]
    pub path_in_repo: String,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub commit_message: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub commit_description: Option<String>,
    #[builder(default, setter(strip_option))]
    pub create_pr: Option<bool>,
    #[builder(default, setter(into, strip_option))]
    pub parent_commit: Option<String>,
}

#[derive(TypedBuilder)]
pub struct RepoUploadFolderParams {
    #[builder(setter(into))]
    pub folder_path: PathBuf,
    #[builder(default, setter(into, strip_option))]
    pub path_in_repo: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub commit_message: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub commit_description: Option<String>,
    #[builder(default, setter(strip_option))]
    pub create_pr: Option<bool>,
    #[builder(default, setter(strip_option))]
    pub allow_patterns: Option<Vec<String>>,
    #[builder(default, setter(strip_option))]
    pub ignore_patterns: Option<Vec<String>>,
    #[builder(default, setter(strip_option))]
    pub delete_patterns: Option<Vec<String>>,
}

#[derive(TypedBuilder)]
pub struct RepoDeleteFileParams {
    #[builder(setter(into))]
    pub path_in_repo: String,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub commit_message: Option<String>,
    #[builder(default, setter(strip_option))]
    pub create_pr: Option<bool>,
}

#[derive(TypedBuilder)]
pub struct RepoDeleteFolderParams {
    #[builder(setter(into))]
    pub path_in_repo: String,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub commit_message: Option<String>,
    #[builder(default, setter(strip_option))]
    pub create_pr: Option<bool>,
}

#[derive(TypedBuilder)]
pub struct RepoCreateCommitParams {
    pub operations: Vec<CommitOperation>,
    #[builder(setter(into))]
    pub commit_message: String,
    #[builder(default, setter(into, strip_option))]
    pub commit_description: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(strip_option))]
    pub create_pr: Option<bool>,
    #[builder(default, setter(into, strip_option))]
    pub parent_commit: Option<String>,
}

#[derive(Default, TypedBuilder)]
pub struct RepoListCommitsParams {
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(strip_option))]
    pub max_items: Option<usize>,
}

#[derive(Default, TypedBuilder)]
pub struct RepoListRefsParams {
    #[builder(default)]
    pub include_pull_requests: bool,
}

#[derive(TypedBuilder)]
pub struct RepoGetCommitDiffParams {
    #[builder(setter(into))]
    pub compare: String,
}

#[derive(TypedBuilder)]
pub struct RepoGetRawDiffParams {
    #[builder(setter(into))]
    pub compare: String,
}

#[derive(TypedBuilder)]
pub struct RepoCreateBranchParams {
    #[builder(setter(into))]
    pub branch: String,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
}

#[derive(TypedBuilder)]
pub struct RepoDeleteBranchParams {
    #[builder(setter(into))]
    pub branch: String,
}

#[derive(TypedBuilder)]
pub struct RepoCreateTagParams {
    #[builder(setter(into))]
    pub tag: String,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub message: Option<String>,
}

#[derive(TypedBuilder)]
pub struct RepoDeleteTagParams {
    #[builder(setter(into))]
    pub tag: String,
}

#[derive(Default, TypedBuilder)]
pub struct RepoUpdateSettingsParams {
    #[builder(default, setter(strip_option))]
    pub private: Option<bool>,
    #[builder(default, setter(into, strip_option))]
    pub gated: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub description: Option<String>,
}

#[cfg(feature = "discussions")]
#[derive(Default, TypedBuilder)]
pub struct RepoListDiscussionsParams {
    #[builder(default, setter(into, strip_option))]
    pub author: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub discussion_type: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub discussion_status: Option<String>,
}

#[cfg(feature = "discussions")]
#[derive(TypedBuilder)]
pub struct RepoDiscussionDetailsParams {
    pub discussion_num: u64,
}

#[cfg(feature = "discussions")]
#[derive(TypedBuilder)]
pub struct RepoCreateDiscussionParams {
    #[builder(setter(into))]
    pub title: String,
    #[builder(default, setter(into, strip_option))]
    pub description: Option<String>,
}

#[cfg(feature = "discussions")]
#[derive(TypedBuilder)]
pub struct RepoCreatePullRequestParams {
    #[builder(setter(into))]
    pub title: String,
    #[builder(default, setter(into, strip_option))]
    pub description: Option<String>,
}

#[cfg(feature = "discussions")]
#[derive(TypedBuilder)]
pub struct RepoCommentDiscussionParams {
    pub discussion_num: u64,
    #[builder(setter(into))]
    pub comment: String,
}

#[cfg(feature = "discussions")]
#[derive(TypedBuilder)]
pub struct RepoEditDiscussionCommentParams {
    pub discussion_num: u64,
    #[builder(setter(into))]
    pub comment_id: String,
    #[builder(setter(into))]
    pub new_content: String,
}

#[cfg(feature = "discussions")]
#[derive(TypedBuilder)]
pub struct RepoHideDiscussionCommentParams {
    pub discussion_num: u64,
    #[builder(setter(into))]
    pub comment_id: String,
}

#[cfg(feature = "discussions")]
#[derive(TypedBuilder)]
pub struct RepoRenameDiscussionParams {
    pub discussion_num: u64,
    #[builder(setter(into))]
    pub new_title: String,
}

#[cfg(feature = "discussions")]
#[derive(TypedBuilder)]
pub struct RepoChangeDiscussionStatusParams {
    pub discussion_num: u64,
    #[builder(setter(into))]
    pub new_status: String,
}

#[cfg(feature = "discussions")]
#[derive(TypedBuilder)]
pub struct RepoMergePullRequestParams {
    pub discussion_num: u64,
}

#[cfg(feature = "access_requests")]
#[derive(TypedBuilder)]
pub struct RepoAccessRequestUserParams {
    #[builder(setter(into))]
    pub user: String,
}

#[cfg(feature = "spaces")]
#[derive(TypedBuilder)]
pub struct SpaceHardwareRequestParams {
    #[builder(setter(into))]
    pub hardware: String,
    #[builder(default, setter(strip_option))]
    pub sleep_time: Option<u64>,
}

#[cfg(feature = "spaces")]
#[derive(TypedBuilder)]
pub struct SpaceSleepTimeParams {
    pub sleep_time: u64,
}

#[cfg(feature = "spaces")]
#[derive(TypedBuilder)]
pub struct SpaceSecretParams {
    #[builder(setter(into))]
    pub key: String,
    #[builder(setter(into))]
    pub value: String,
    #[builder(default, setter(into, strip_option))]
    pub description: Option<String>,
}

#[cfg(feature = "spaces")]
#[derive(TypedBuilder)]
pub struct SpaceSecretDeleteParams {
    #[builder(setter(into))]
    pub key: String,
}

#[cfg(feature = "spaces")]
#[derive(TypedBuilder)]
pub struct SpaceVariableParams {
    #[builder(setter(into))]
    pub key: String,
    #[builder(setter(into))]
    pub value: String,
    #[builder(default, setter(into, strip_option))]
    pub description: Option<String>,
}

#[cfg(feature = "spaces")]
#[derive(TypedBuilder)]
pub struct SpaceVariableDeleteParams {
    #[builder(setter(into))]
    pub key: String,
}

impl HFClient {
    /// Create an [`HFRepository`] handle for any repo type.
    pub fn repo(&self, repo_type: RepoType, owner: impl Into<String>, name: impl Into<String>) -> HFRepository {
        HFRepository::new(self.clone(), repo_type, owner, name)
    }

    /// Create an [`HFRepository`] handle for a model repository.
    pub fn model(&self, owner: impl Into<String>, name: impl Into<String>) -> HFRepository {
        self.repo(RepoType::Model, owner, name)
    }

    /// Create an [`HFRepository`] handle for a dataset repository.
    pub fn dataset(&self, owner: impl Into<String>, name: impl Into<String>) -> HFRepository {
        self.repo(RepoType::Dataset, owner, name)
    }

    /// Create an [`HFSpace`] handle for a Space repository.
    pub fn space(&self, owner: impl Into<String>, name: impl Into<String>) -> HFSpace {
        HFSpace::new(self.clone(), owner, name)
    }
}

impl HFRepository {
    /// Construct a new repository handle. Prefer the factory methods on [`HFClient`] instead.
    pub fn new(client: HFClient, repo_type: RepoType, owner: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            client,
            owner: owner.into(),
            name: name.into(),
            repo_type,
            default_revision: None,
        }
    }

    /// Return a reference to the underlying [`HFClient`].
    pub fn client(&self) -> &HFClient {
        &self.client
    }

    /// The repository owner (user or organization name).
    pub fn owner(&self) -> &str {
        &self.owner
    }

    /// The repository name (without owner prefix).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The full `"owner/name"` identifier used in Hub API calls.
    ///
    /// If no owner is set, returns just the name (for repos using short-form IDs like `"gpt2"`).
    pub fn repo_path(&self) -> String {
        if self.owner.is_empty() {
            self.name.clone()
        } else {
            format!("{}/{}", self.owner, self.name)
        }
    }

    /// The type of this repository (model, dataset, or space).
    pub fn repo_type(&self) -> RepoType {
        self.repo_type
    }

    /// The default revision used when no per-call revision is supplied, if any.
    pub fn default_revision(&self) -> Option<&str> {
        self.default_revision.as_deref()
    }

    /// Return a clone of this handle pinned to the given revision.
    ///
    /// Methods that accept an optional revision will use this value when none is specified.
    pub fn with_revision(&self, revision: impl Into<String>) -> Self {
        let mut repo = self.clone();
        repo.default_revision = Some(revision.into());
        repo
    }

    /// Return a clone of this handle with the default revision cleared.
    pub fn without_revision(&self) -> Self {
        let mut repo = self.clone();
        repo.default_revision = None;
        repo
    }

    /// Fetch repository metadata, returning the appropriate [`RepoInfo`] variant.
    pub async fn info(&self, params: &RepoInfoParams) -> Result<RepoInfo> {
        let repo_id = self.repo_path();
        let revision = self.resolve_revision(params.revision.as_deref());

        match self.repo_type {
            RepoType::Model => self
                .model_info(&ModelInfoParams { repo_id, revision })
                .await
                .map(RepoInfo::Model),
            RepoType::Dataset => self
                .dataset_info(&DatasetInfoParams { repo_id, revision })
                .await
                .map(RepoInfo::Dataset),
            RepoType::Space => self
                .space_info(&SpaceInfoParams { repo_id, revision })
                .await
                .map(RepoInfo::Space),
            RepoType::Kernel => {
                Err(HfError::Other("Repository info is not implemented yet for kernel repositories".to_string()))
            },
        }
    }

    /// Return `true` if the repository exists and is accessible with the current credentials.
    pub async fn exists(&self) -> Result<bool> {
        self.repo_exists(&crate::types::RepoExistsParams {
            repo_id: self.repo_path(),
            repo_type: Some(self.repo_type),
        })
        .await
    }

    /// Return a flat list of all file paths in the repository at the given revision.
    pub async fn list_files(&self, params: &RepoListFilesParams) -> Result<Vec<String>> {
        self.list_repo_files(&ListRepoFilesParams {
            repo_id: self.repo_path(),
            revision: self.resolve_revision(params.revision.as_deref()),
            repo_type: Some(self.repo_type),
        })
        .await
    }

    /// Stream file and directory entries in the repository tree.
    ///
    /// Returns `Result<impl Stream<Item = Result<RepoTreeEntry>>>`. Set `recursive` to traverse
    /// subdirectories. Use `max_items` to cap the total number of entries yielded.
    pub fn list_tree(&self, params: &RepoListTreeParams) -> Result<impl Stream<Item = Result<RepoTreeEntry>> + '_> {
        let revision = self.effective_revision(params.revision.as_deref());
        let url_str = format!("{}/tree/{}", self.client.api_url(Some(self.repo_type), &self.repo_path()), revision);
        let url = Url::parse(&url_str)?;

        let mut query: Vec<(String, String)> = Vec::new();
        if params.recursive {
            query.push(("recursive".into(), "true".into()));
        }
        if params.expand {
            query.push(("expand".into(), "true".into()));
        }

        Ok(self.client.paginate(url, query, params.max_items))
    }

    /// Stream commit history for the repository at a given revision.
    ///
    /// Returns `Result<impl Stream<Item = Result<GitCommitInfo>>>`. Use `max_items` to limit
    /// the total number of commits yielded.
    pub fn list_commits(
        &self,
        params: &RepoListCommitsParams,
    ) -> Result<impl Stream<Item = Result<GitCommitInfo>> + '_> {
        let revision = self.effective_revision(params.revision.as_deref());
        let url_str = format!("{}/commits/{}", self.client.api_url(Some(self.repo_type), &self.repo_path()), revision);
        let url = Url::parse(&url_str)?;
        Ok(self.client.paginate(url, vec![], params.max_items))
    }

    /// Fetch all branches, tags, and optionally pull request refs for the repository.
    pub async fn list_refs(&self, params: &RepoListRefsParams) -> Result<GitRefs> {
        self.list_repo_refs(&ListRepoRefsParams {
            repo_id: self.repo_path(),
            repo_type: Some(self.repo_type),
            include_pull_requests: params.include_pull_requests,
        })
        .await
    }

    /// Fetch a structured diff between two revisions (HEAD..compare or a commit SHA).
    pub async fn get_commit_diff(&self, params: &RepoGetCommitDiffParams) -> Result<String> {
        self.list_repo_commit_diff(&GetCommitDiffParams {
            repo_id: self.repo_path(),
            compare: params.compare.clone(),
            repo_type: Some(self.repo_type),
        })
        .await
    }

    /// Fetch the raw unified diff between two revisions as a string.
    pub async fn get_raw_diff(&self, params: &RepoGetRawDiffParams) -> Result<String> {
        self.list_repo_raw_diff(&GetRawDiffParams {
            repo_id: self.repo_path(),
            compare: params.compare.clone(),
            repo_type: Some(self.repo_type),
        })
        .await
    }

    /// Create a new branch, optionally starting from a specific revision.
    pub async fn create_branch(&self, params: &RepoCreateBranchParams) -> Result<()> {
        self.create_repo_branch(&CreateBranchParams {
            repo_id: self.repo_path(),
            branch: params.branch.clone(),
            revision: self.resolve_revision(params.revision.as_deref()),
            repo_type: Some(self.repo_type),
        })
        .await
    }

    /// Delete a branch from the repository.
    pub async fn delete_branch(&self, params: &RepoDeleteBranchParams) -> Result<()> {
        self.delete_repo_branch(&DeleteBranchParams {
            repo_id: self.repo_path(),
            branch: params.branch.clone(),
            repo_type: Some(self.repo_type),
        })
        .await
    }

    /// Create a lightweight or annotated tag, optionally at a specific revision.
    pub async fn create_tag(&self, params: &RepoCreateTagParams) -> Result<()> {
        self.create_repo_tag(&CreateTagParams {
            repo_id: self.repo_path(),
            tag: params.tag.clone(),
            revision: self.resolve_revision(params.revision.as_deref()),
            message: params.message.clone(),
            repo_type: Some(self.repo_type),
        })
        .await
    }

    /// Delete a tag from the repository.
    pub async fn delete_tag(&self, params: &RepoDeleteTagParams) -> Result<()> {
        self.delete_repo_tag(&DeleteTagParams {
            repo_id: self.repo_path(),
            tag: params.tag.clone(),
            repo_type: Some(self.repo_type),
        })
        .await
    }

    /// Update repository settings such as visibility, gating policy, or description.
    pub async fn update_settings(&self, params: &RepoUpdateSettingsParams) -> Result<()> {
        self.update_repo_settings(&UpdateRepoParams {
            repo_id: self.repo_path(),
            repo_type: Some(self.repo_type),
            private: params.private,
            gated: params.gated.clone(),
            description: params.description.clone(),
        })
        .await
    }

    /// List discussions for this repository, with optional filters on author, type, and status.
    #[cfg(feature = "discussions")]
    pub async fn list_discussions(&self, params: &RepoListDiscussionsParams) -> Result<DiscussionsResponse> {
        self.get_repo_discussions(&GetRepoDiscussionsParams {
            repo_id: self.repo_path(),
            repo_type: Some(self.repo_type),
            author: params.author.clone(),
            discussion_type: params.discussion_type.clone(),
            discussion_status: params.discussion_status.clone(),
        })
        .await
    }

    /// Fetch the full details and event timeline for a single discussion or pull request.
    #[cfg(feature = "discussions")]
    pub async fn discussion_details(&self, params: &RepoDiscussionDetailsParams) -> Result<DiscussionWithDetails> {
        self.get_discussion_details(&GetDiscussionDetailsParams {
            repo_id: self.repo_path(),
            discussion_num: params.discussion_num,
            repo_type: Some(self.repo_type),
        })
        .await
    }

    /// Stream users who have liked this repository.
    ///
    /// Returns `Result<impl Stream<Item = Result<User>>>`. Pass `max_items` to cap the total
    /// number of users yielded.
    #[cfg(feature = "likes")]
    pub fn list_likers(&self, max_items: Option<usize>) -> Result<impl Stream<Item = Result<User>> + '_> {
        self.list_repo_likers(&ListRepoLikersParams {
            repo_id: self.repo_path(),
            repo_type: Some(self.repo_type),
            max_items,
        })
    }

    pub(crate) fn resolve_revision(&self, revision: Option<&str>) -> Option<String> {
        revision.map(ToOwned::to_owned).or_else(|| self.default_revision.clone())
    }

    pub(crate) fn effective_revision<'a>(&'a self, revision: Option<&'a str>) -> &'a str {
        revision
            .or(self.default_revision.as_deref())
            .unwrap_or(constants::DEFAULT_REVISION)
    }
}

impl HFSpace {
    /// Construct a new Space handle. Prefer [`HFClient::space`] in most cases.
    pub fn new(client: HFClient, owner: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            repo: HFRepository::new(client, RepoType::Space, owner, name),
        }
    }

    /// Return a clone of this handle pinned to the given revision.
    pub fn with_revision(&self, revision: impl Into<String>) -> Self {
        Self {
            repo: self.repo.with_revision(revision),
        }
    }

    /// Return a clone of this handle with the default revision cleared.
    pub fn without_revision(&self) -> Self {
        Self {
            repo: self.repo.without_revision(),
        }
    }

    /// Consume this handle and return the underlying [`HFRepository`].
    pub fn into_repo(self) -> HFRepository {
        self.repo
    }
}

impl TryFrom<HFRepository> for HFSpace {
    type Error = HfError;

    fn try_from(repo: HFRepository) -> Result<Self> {
        if repo.repo_type() != RepoType::Space {
            return Err(HfError::InvalidRepoType {
                expected: RepoType::Space,
                actual: repo.repo_type(),
            });
        }
        Ok(Self { repo })
    }
}

impl From<HFSpace> for HFRepository {
    fn from(space: HFSpace) -> Self {
        space.repo
    }
}

impl Deref for HFSpace {
    type Target = HFRepo;

    fn deref(&self) -> &Self::Target {
        &self.repo
    }
}

#[cfg(test)]
mod tests {
    use super::{HFRepository, HFSpace};
    use crate::types::RepoType;

    #[test]
    fn test_repo_path_and_accessors() {
        let client = crate::HFClient::builder().build().unwrap();
        let repo = HFRepository::new(client, RepoType::Model, "openai-community", "gpt2");

        assert_eq!(repo.owner(), "openai-community");
        assert_eq!(repo.name(), "gpt2");
        assert_eq!(repo.repo_path(), "openai-community/gpt2");
        assert_eq!(repo.repo_type(), RepoType::Model);
        assert_eq!(repo.default_revision(), None);
    }

    #[test]
    fn test_with_and_without_revision() {
        let client = crate::HFClient::builder().build().unwrap();
        let repo = HFRepository::new(client, RepoType::Dataset, "rajpurkar", "squad");
        let pinned = repo.with_revision("refs/pr/1");

        assert_eq!(repo.default_revision(), None);
        assert_eq!(pinned.default_revision(), Some("refs/pr/1"));
        assert_eq!(pinned.without_revision().default_revision(), None);
    }

    #[test]
    fn test_hfspace_constructor_and_deref() {
        let client = crate::HFClient::builder().build().unwrap();
        let space = HFSpace::new(client, "huggingface-projects", "diffusers-gallery");

        assert_eq!(space.repo_type(), RepoType::Space);
        assert_eq!(space.repo_path(), "huggingface-projects/diffusers-gallery");
    }

    #[test]
    fn test_hfspace_try_from_repo() {
        let client = crate::HFClient::builder().build().unwrap();
        let space_repo = HFRepository::new(client.clone(), RepoType::Space, "owner", "space");
        assert!(HFSpace::try_from(space_repo).is_ok());

        let model_repo = HFRepository::new(client, RepoType::Model, "owner", "model");
        let error = HFSpace::try_from(model_repo).unwrap_err();
        match error {
            crate::HfError::InvalidRepoType { expected, actual } => {
                assert_eq!(expected, RepoType::Space);
                assert_eq!(actual, RepoType::Model);
            },
            _ => panic!("expected invalid repo type error"),
        }
    }
}
