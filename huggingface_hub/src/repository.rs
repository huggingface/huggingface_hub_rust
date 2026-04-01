use std::fmt;
use std::ops::Deref;
use std::path::PathBuf;

use futures::Stream;
use typed_builder::TypedBuilder;
use url::Url;

use crate::client::HFClient;
use crate::constants;
use crate::error::{HfError, Result};
#[cfg(feature = "access_requests")]
use crate::types::{AccessRequest, GrantAccessParams, HandleAccessRequestParams, ListAccessRequestsParams};
use crate::types::{
    AddSource, CommitInfo, CommitOperation, CreateBranchParams, CreateCommitParams, CreateTagParams, DatasetInfoParams,
    DeleteBranchParams, DeleteFileParams, DeleteFolderParams, DeleteTagParams, DownloadFileParams,
    DownloadFileStreamParams, FileExistsParams, GetCommitDiffParams, GetPathsInfoParams, GetRawDiffParams,
    GitCommitInfo, GitRefs, ListRepoFilesParams, ListRepoRefsParams, ModelInfoParams, RepoInfo, RepoTreeEntry,
    RepoType, RevisionExistsParams, SnapshotDownloadParams, SpaceInfoParams, UpdateRepoParams, UploadFileParams,
    UploadFolderParams,
};
#[cfg(feature = "spaces")]
use crate::types::{
    AddSpaceSecretParams, AddSpaceVariableParams, DeleteSpaceSecretParams, DeleteSpaceVariableParams,
    GetSpaceRuntimeParams, PauseSpaceParams, RequestSpaceHardwareParams, RestartSpaceParams, SetSpaceSleepTimeParams,
    SpaceRuntime,
};
#[cfg(feature = "discussions")]
use crate::types::{
    ChangeDiscussionStatusParams, CommentDiscussionParams, CreateDiscussionParams, CreatePullRequestParams,
    DiscussionComment, DiscussionWithDetails, DiscussionsResponse, EditDiscussionCommentParams,
    GetDiscussionDetailsParams, GetRepoDiscussionsParams, HideDiscussionCommentParams, MergePullRequestParams,
    RenameDiscussionParams,
};
#[cfg(feature = "likes")]
use crate::types::{LikeParams, ListRepoLikersParams, User};

#[derive(Clone)]
pub struct HFRepository {
    client: HFClient,
    owner: String,
    name: String,
    repo_type: RepoType,
    default_revision: Option<String>,
}

pub type HFRepo = HFRepository;
pub type HfRepository = HFRepository;
pub type HfRepo = HFRepo;

#[derive(Clone)]
pub struct HFSpace {
    repo: HFRepository,
}

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
    pub fn repo(&self, repo_type: RepoType, owner: impl Into<String>, name: impl Into<String>) -> HFRepository {
        HFRepository::new(self.clone(), repo_type, owner, name)
    }

    pub fn model(&self, owner: impl Into<String>, name: impl Into<String>) -> HFRepository {
        self.repo(RepoType::Model, owner, name)
    }

    pub fn dataset(&self, owner: impl Into<String>, name: impl Into<String>) -> HFRepository {
        self.repo(RepoType::Dataset, owner, name)
    }

    pub fn space(&self, owner: impl Into<String>, name: impl Into<String>) -> HFSpace {
        HFSpace::new(self.clone(), owner, name)
    }
}

impl HFRepository {
    pub fn new(client: HFClient, repo_type: RepoType, owner: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            client,
            owner: owner.into(),
            name: name.into(),
            repo_type,
            default_revision: None,
        }
    }

    pub fn client(&self) -> &HFClient {
        &self.client
    }

    pub fn owner(&self) -> &str {
        &self.owner
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn repo_path(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }

    pub fn repo_type(&self) -> RepoType {
        self.repo_type
    }

    pub fn default_revision(&self) -> Option<&str> {
        self.default_revision.as_deref()
    }

    pub fn with_revision(&self, revision: impl Into<String>) -> Self {
        let mut repo = self.clone();
        repo.default_revision = Some(revision.into());
        repo
    }

    pub fn without_revision(&self) -> Self {
        let mut repo = self.clone();
        repo.default_revision = None;
        repo
    }

    pub async fn info(&self, params: &RepoInfoParams) -> Result<RepoInfo> {
        let repo_id = self.repo_path();
        let revision = self.resolve_revision(params.revision.as_deref());

        match self.repo_type {
            RepoType::Model => self
                .client
                .model_info(&ModelInfoParams { repo_id, revision })
                .await
                .map(RepoInfo::Model),
            RepoType::Dataset => self
                .client
                .dataset_info(&DatasetInfoParams { repo_id, revision })
                .await
                .map(RepoInfo::Dataset),
            RepoType::Space => self
                .client
                .space_info(&SpaceInfoParams { repo_id, revision })
                .await
                .map(RepoInfo::Space),
            RepoType::Kernel => {
                Err(HfError::Other("Repository info is not implemented yet for kernel repositories".to_string()))
            },
        }
    }

    pub async fn exists(&self) -> Result<bool> {
        self.client
            .repo_exists(&crate::types::RepoExistsParams {
                repo_id: self.repo_path(),
                repo_type: Some(self.repo_type),
            })
            .await
    }

    pub async fn revision_exists(&self, params: &RepoRevisionExistsParams) -> Result<bool> {
        self.client
            .revision_exists(&RevisionExistsParams {
                repo_id: self.repo_path(),
                revision: params.revision.clone(),
                repo_type: Some(self.repo_type),
            })
            .await
    }

    pub async fn file_exists(&self, params: &RepoFileExistsParams) -> Result<bool> {
        self.client
            .file_exists(&FileExistsParams {
                repo_id: self.repo_path(),
                filename: params.filename.clone(),
                revision: self.resolve_revision(params.revision.as_deref()),
                repo_type: Some(self.repo_type),
            })
            .await
    }

    pub async fn list_files(&self, params: &RepoListFilesParams) -> Result<Vec<String>> {
        self.client
            .list_repo_files(&ListRepoFilesParams {
                repo_id: self.repo_path(),
                revision: self.resolve_revision(params.revision.as_deref()),
                repo_type: Some(self.repo_type),
            })
            .await
    }

    pub fn list_tree(&self, params: &RepoListTreeParams) -> impl Stream<Item = Result<RepoTreeEntry>> + '_ {
        let revision = self.effective_revision(params.revision.as_deref());
        let url_str = format!("{}/tree/{}", self.client.api_url(Some(self.repo_type), &self.repo_path()), revision);
        let url = Url::parse(&url_str).unwrap();

        let mut query: Vec<(String, String)> = Vec::new();
        if params.recursive {
            query.push(("recursive".into(), "true".into()));
        }
        if params.expand {
            query.push(("expand".into(), "true".into()));
        }

        self.client.paginate(url, query)
    }

    pub async fn get_paths_info(&self, params: &RepoGetPathsInfoParams) -> Result<Vec<RepoTreeEntry>> {
        self.client
            .get_paths_info(&GetPathsInfoParams {
                repo_id: self.repo_path(),
                paths: params.paths.clone(),
                revision: self.resolve_revision(params.revision.as_deref()),
                repo_type: Some(self.repo_type),
            })
            .await
    }

    pub async fn download_file(&self, params: &RepoDownloadFileParams) -> Result<PathBuf> {
        self.client
            .download_file(&DownloadFileParams {
                repo_id: self.repo_path(),
                filename: params.filename.clone(),
                local_dir: params.local_dir.clone(),
                repo_type: Some(self.repo_type),
                revision: self.resolve_revision(params.revision.as_deref()),
                force_download: params.force_download,
                local_files_only: params.local_files_only,
            })
            .await
    }

    pub async fn download_file_stream(
        &self,
        params: &RepoDownloadFileStreamParams,
    ) -> Result<(Option<u64>, impl Stream<Item = std::result::Result<bytes::Bytes, reqwest::Error>>)> {
        self.client
            .download_file_stream(&DownloadFileStreamParams {
                repo_id: self.repo_path(),
                filename: params.filename.clone(),
                repo_type: Some(self.repo_type),
                revision: self.resolve_revision(params.revision.as_deref()),
            })
            .await
    }

    pub async fn snapshot_download(&self, params: &RepoSnapshotDownloadParams) -> Result<PathBuf> {
        self.client
            .snapshot_download(&SnapshotDownloadParams {
                repo_id: self.repo_path(),
                repo_type: Some(self.repo_type),
                revision: self.resolve_revision(params.revision.as_deref()),
                allow_patterns: params.allow_patterns.clone(),
                ignore_patterns: params.ignore_patterns.clone(),
                local_dir: params.local_dir.clone(),
                force_download: params.force_download,
                local_files_only: params.local_files_only,
                max_workers: params.max_workers,
            })
            .await
    }

    pub async fn create_commit(&self, params: &RepoCreateCommitParams) -> Result<CommitInfo> {
        self.client
            .create_commit(&CreateCommitParams {
                repo_id: self.repo_path(),
                operations: params.operations.clone(),
                commit_message: params.commit_message.clone(),
                commit_description: params.commit_description.clone(),
                repo_type: Some(self.repo_type),
                revision: self.resolve_revision(params.revision.as_deref()),
                create_pr: params.create_pr,
                parent_commit: params.parent_commit.clone(),
            })
            .await
    }

    pub async fn upload_file(&self, params: &RepoUploadFileParams) -> Result<CommitInfo> {
        self.client
            .upload_file(&UploadFileParams {
                repo_id: self.repo_path(),
                source: params.source.clone(),
                path_in_repo: params.path_in_repo.clone(),
                repo_type: Some(self.repo_type),
                revision: self.resolve_revision(params.revision.as_deref()),
                commit_message: params.commit_message.clone(),
                commit_description: params.commit_description.clone(),
                create_pr: params.create_pr,
                parent_commit: params.parent_commit.clone(),
            })
            .await
    }

    pub async fn upload_folder(&self, params: &RepoUploadFolderParams) -> Result<CommitInfo> {
        self.client
            .upload_folder(&UploadFolderParams {
                repo_id: self.repo_path(),
                folder_path: params.folder_path.clone(),
                path_in_repo: params.path_in_repo.clone(),
                repo_type: Some(self.repo_type),
                revision: self.resolve_revision(params.revision.as_deref()),
                commit_message: params.commit_message.clone(),
                commit_description: params.commit_description.clone(),
                create_pr: params.create_pr,
                allow_patterns: params.allow_patterns.clone(),
                ignore_patterns: params.ignore_patterns.clone(),
                delete_patterns: params.delete_patterns.clone(),
            })
            .await
    }

    pub async fn delete_file(&self, params: &RepoDeleteFileParams) -> Result<CommitInfo> {
        self.client
            .delete_file(&DeleteFileParams {
                repo_id: self.repo_path(),
                path_in_repo: params.path_in_repo.clone(),
                repo_type: Some(self.repo_type),
                revision: self.resolve_revision(params.revision.as_deref()),
                commit_message: params.commit_message.clone(),
                create_pr: params.create_pr,
            })
            .await
    }

    pub async fn delete_folder(&self, params: &RepoDeleteFolderParams) -> Result<CommitInfo> {
        self.client
            .delete_folder(&DeleteFolderParams {
                repo_id: self.repo_path(),
                path_in_repo: params.path_in_repo.clone(),
                repo_type: Some(self.repo_type),
                revision: self.resolve_revision(params.revision.as_deref()),
                commit_message: params.commit_message.clone(),
                create_pr: params.create_pr,
            })
            .await
    }

    pub fn list_commits(&self, params: &RepoListCommitsParams) -> impl Stream<Item = Result<GitCommitInfo>> + '_ {
        let revision = self.effective_revision(params.revision.as_deref());
        let url_str = format!("{}/commits/{}", self.client.api_url(Some(self.repo_type), &self.repo_path()), revision);
        let url = Url::parse(&url_str).unwrap();
        self.client.paginate(url, vec![])
    }

    pub async fn list_refs(&self, params: &RepoListRefsParams) -> Result<GitRefs> {
        self.client
            .list_repo_refs(&ListRepoRefsParams {
                repo_id: self.repo_path(),
                repo_type: Some(self.repo_type),
                include_pull_requests: params.include_pull_requests,
            })
            .await
    }

    pub async fn get_commit_diff(&self, params: &RepoGetCommitDiffParams) -> Result<String> {
        self.client
            .get_commit_diff(&GetCommitDiffParams {
                repo_id: self.repo_path(),
                compare: params.compare.clone(),
                repo_type: Some(self.repo_type),
            })
            .await
    }

    pub async fn get_raw_diff(&self, params: &RepoGetRawDiffParams) -> Result<String> {
        self.client
            .get_raw_diff(&GetRawDiffParams {
                repo_id: self.repo_path(),
                compare: params.compare.clone(),
                repo_type: Some(self.repo_type),
            })
            .await
    }

    pub async fn create_branch(&self, params: &RepoCreateBranchParams) -> Result<()> {
        self.client
            .create_branch(&CreateBranchParams {
                repo_id: self.repo_path(),
                branch: params.branch.clone(),
                revision: self.resolve_revision(params.revision.as_deref()),
                repo_type: Some(self.repo_type),
            })
            .await
    }

    pub async fn delete_branch(&self, params: &RepoDeleteBranchParams) -> Result<()> {
        self.client
            .delete_branch(&DeleteBranchParams {
                repo_id: self.repo_path(),
                branch: params.branch.clone(),
                repo_type: Some(self.repo_type),
            })
            .await
    }

    pub async fn create_tag(&self, params: &RepoCreateTagParams) -> Result<()> {
        self.client
            .create_tag(&CreateTagParams {
                repo_id: self.repo_path(),
                tag: params.tag.clone(),
                revision: self.resolve_revision(params.revision.as_deref()),
                message: params.message.clone(),
                repo_type: Some(self.repo_type),
            })
            .await
    }

    pub async fn delete_tag(&self, params: &RepoDeleteTagParams) -> Result<()> {
        self.client
            .delete_tag(&DeleteTagParams {
                repo_id: self.repo_path(),
                tag: params.tag.clone(),
                repo_type: Some(self.repo_type),
            })
            .await
    }

    pub async fn update_settings(&self, params: &RepoUpdateSettingsParams) -> Result<()> {
        self.client
            .update_repo_settings(&UpdateRepoParams {
                repo_id: self.repo_path(),
                repo_type: Some(self.repo_type),
                private: params.private,
                gated: params.gated.clone(),
                description: params.description.clone(),
            })
            .await
    }

    #[cfg(feature = "discussions")]
    pub async fn list_discussions(&self, params: &RepoListDiscussionsParams) -> Result<DiscussionsResponse> {
        self.client
            .get_repo_discussions(&GetRepoDiscussionsParams {
                repo_id: self.repo_path(),
                repo_type: Some(self.repo_type),
                author: params.author.clone(),
                discussion_type: params.discussion_type.clone(),
                discussion_status: params.discussion_status.clone(),
            })
            .await
    }

    #[cfg(feature = "discussions")]
    pub async fn discussion_details(&self, params: &RepoDiscussionDetailsParams) -> Result<DiscussionWithDetails> {
        self.client
            .get_discussion_details(&GetDiscussionDetailsParams {
                repo_id: self.repo_path(),
                discussion_num: params.discussion_num,
                repo_type: Some(self.repo_type),
            })
            .await
    }

    #[cfg(feature = "discussions")]
    pub async fn create_discussion(&self, params: &RepoCreateDiscussionParams) -> Result<DiscussionWithDetails> {
        self.client
            .create_discussion(&CreateDiscussionParams {
                repo_id: self.repo_path(),
                title: params.title.clone(),
                description: params.description.clone(),
                repo_type: Some(self.repo_type),
            })
            .await
    }

    #[cfg(feature = "discussions")]
    pub async fn create_pull_request(&self, params: &RepoCreatePullRequestParams) -> Result<DiscussionWithDetails> {
        self.client
            .create_pull_request(&CreatePullRequestParams {
                repo_id: self.repo_path(),
                title: params.title.clone(),
                description: params.description.clone(),
                repo_type: Some(self.repo_type),
            })
            .await
    }

    #[cfg(feature = "discussions")]
    pub async fn comment_discussion(&self, params: &RepoCommentDiscussionParams) -> Result<DiscussionComment> {
        self.client
            .comment_discussion(&CommentDiscussionParams {
                repo_id: self.repo_path(),
                discussion_num: params.discussion_num,
                comment: params.comment.clone(),
                repo_type: Some(self.repo_type),
            })
            .await
    }

    #[cfg(feature = "discussions")]
    pub async fn edit_discussion_comment(&self, params: &RepoEditDiscussionCommentParams) -> Result<DiscussionComment> {
        self.client
            .edit_discussion_comment(&EditDiscussionCommentParams {
                repo_id: self.repo_path(),
                discussion_num: params.discussion_num,
                comment_id: params.comment_id.clone(),
                new_content: params.new_content.clone(),
                repo_type: Some(self.repo_type),
            })
            .await
    }

    #[cfg(feature = "discussions")]
    pub async fn hide_discussion_comment(&self, params: &RepoHideDiscussionCommentParams) -> Result<DiscussionComment> {
        self.client
            .hide_discussion_comment(&HideDiscussionCommentParams {
                repo_id: self.repo_path(),
                discussion_num: params.discussion_num,
                comment_id: params.comment_id.clone(),
                repo_type: Some(self.repo_type),
            })
            .await
    }

    #[cfg(feature = "discussions")]
    pub async fn rename_discussion(&self, params: &RepoRenameDiscussionParams) -> Result<DiscussionWithDetails> {
        self.client
            .rename_discussion(&RenameDiscussionParams {
                repo_id: self.repo_path(),
                discussion_num: params.discussion_num,
                new_title: params.new_title.clone(),
                repo_type: Some(self.repo_type),
            })
            .await
    }

    #[cfg(feature = "discussions")]
    pub async fn change_discussion_status(
        &self,
        params: &RepoChangeDiscussionStatusParams,
    ) -> Result<DiscussionWithDetails> {
        self.client
            .change_discussion_status(&ChangeDiscussionStatusParams {
                repo_id: self.repo_path(),
                discussion_num: params.discussion_num,
                new_status: params.new_status.clone(),
                repo_type: Some(self.repo_type),
            })
            .await
    }

    #[cfg(feature = "discussions")]
    pub async fn merge_pull_request(&self, params: &RepoMergePullRequestParams) -> Result<DiscussionWithDetails> {
        self.client
            .merge_pull_request(&MergePullRequestParams {
                repo_id: self.repo_path(),
                discussion_num: params.discussion_num,
                repo_type: Some(self.repo_type),
            })
            .await
    }

    #[cfg(feature = "access_requests")]
    pub async fn list_pending_access_requests(&self) -> Result<Vec<AccessRequest>> {
        self.client
            .list_pending_access_requests(&ListAccessRequestsParams {
                repo_id: self.repo_path(),
                repo_type: Some(self.repo_type),
            })
            .await
    }

    #[cfg(feature = "access_requests")]
    pub async fn list_accepted_access_requests(&self) -> Result<Vec<AccessRequest>> {
        self.client
            .list_accepted_access_requests(&ListAccessRequestsParams {
                repo_id: self.repo_path(),
                repo_type: Some(self.repo_type),
            })
            .await
    }

    #[cfg(feature = "access_requests")]
    pub async fn list_rejected_access_requests(&self) -> Result<Vec<AccessRequest>> {
        self.client
            .list_rejected_access_requests(&ListAccessRequestsParams {
                repo_id: self.repo_path(),
                repo_type: Some(self.repo_type),
            })
            .await
    }

    #[cfg(feature = "access_requests")]
    pub async fn accept_access_request(&self, params: &RepoAccessRequestUserParams) -> Result<()> {
        self.client
            .accept_access_request(&HandleAccessRequestParams {
                repo_id: self.repo_path(),
                user: params.user.clone(),
                repo_type: Some(self.repo_type),
            })
            .await
    }

    #[cfg(feature = "access_requests")]
    pub async fn reject_access_request(&self, params: &RepoAccessRequestUserParams) -> Result<()> {
        self.client
            .reject_access_request(&HandleAccessRequestParams {
                repo_id: self.repo_path(),
                user: params.user.clone(),
                repo_type: Some(self.repo_type),
            })
            .await
    }

    #[cfg(feature = "access_requests")]
    pub async fn cancel_access_request(&self, params: &RepoAccessRequestUserParams) -> Result<()> {
        self.client
            .cancel_access_request(&HandleAccessRequestParams {
                repo_id: self.repo_path(),
                user: params.user.clone(),
                repo_type: Some(self.repo_type),
            })
            .await
    }

    #[cfg(feature = "access_requests")]
    pub async fn grant_access(&self, params: &RepoAccessRequestUserParams) -> Result<()> {
        self.client
            .grant_access(&GrantAccessParams {
                repo_id: self.repo_path(),
                user: params.user.clone(),
                repo_type: Some(self.repo_type),
            })
            .await
    }

    #[cfg(feature = "likes")]
    pub async fn like(&self) -> Result<()> {
        self.client
            .like(&LikeParams {
                repo_id: self.repo_path(),
                repo_type: Some(self.repo_type),
            })
            .await
    }

    #[cfg(feature = "likes")]
    pub async fn unlike(&self) -> Result<()> {
        self.client
            .unlike(&LikeParams {
                repo_id: self.repo_path(),
                repo_type: Some(self.repo_type),
            })
            .await
    }

    #[cfg(feature = "likes")]
    pub fn list_likers(&self) -> impl Stream<Item = Result<User>> + '_ {
        self.client.list_repo_likers(&ListRepoLikersParams {
            repo_id: self.repo_path(),
            repo_type: Some(self.repo_type),
        })
    }

    fn resolve_revision(&self, revision: Option<&str>) -> Option<String> {
        revision.map(ToOwned::to_owned).or_else(|| self.default_revision.clone())
    }

    fn effective_revision<'a>(&'a self, revision: Option<&'a str>) -> &'a str {
        revision
            .or(self.default_revision.as_deref())
            .unwrap_or(constants::DEFAULT_REVISION)
    }
}

impl HFSpace {
    pub fn new(client: HFClient, owner: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            repo: HFRepository::new(client, RepoType::Space, owner, name),
        }
    }

    pub fn with_revision(&self, revision: impl Into<String>) -> Self {
        Self {
            repo: self.repo.with_revision(revision),
        }
    }

    pub fn without_revision(&self) -> Self {
        Self {
            repo: self.repo.without_revision(),
        }
    }

    pub fn into_repo(self) -> HFRepository {
        self.repo
    }

    #[cfg(feature = "spaces")]
    pub async fn runtime(&self) -> Result<SpaceRuntime> {
        self.repo
            .client()
            .get_space_runtime(&GetSpaceRuntimeParams {
                repo_id: self.repo.repo_path(),
            })
            .await
    }

    #[cfg(feature = "spaces")]
    pub async fn request_hardware(&self, params: &SpaceHardwareRequestParams) -> Result<SpaceRuntime> {
        self.repo
            .client()
            .request_space_hardware(&RequestSpaceHardwareParams {
                repo_id: self.repo.repo_path(),
                hardware: params.hardware.clone(),
                sleep_time: params.sleep_time,
            })
            .await
    }

    #[cfg(feature = "spaces")]
    pub async fn set_sleep_time(&self, params: &SpaceSleepTimeParams) -> Result<()> {
        self.repo
            .client()
            .set_space_sleep_time(&SetSpaceSleepTimeParams {
                repo_id: self.repo.repo_path(),
                sleep_time: params.sleep_time,
            })
            .await
    }

    #[cfg(feature = "spaces")]
    pub async fn pause(&self) -> Result<SpaceRuntime> {
        self.repo
            .client()
            .pause_space(&PauseSpaceParams {
                repo_id: self.repo.repo_path(),
            })
            .await
    }

    #[cfg(feature = "spaces")]
    pub async fn restart(&self) -> Result<SpaceRuntime> {
        self.repo
            .client()
            .restart_space(&RestartSpaceParams {
                repo_id: self.repo.repo_path(),
            })
            .await
    }

    #[cfg(feature = "spaces")]
    pub async fn add_secret(&self, params: &SpaceSecretParams) -> Result<()> {
        self.repo
            .client()
            .add_space_secret(&AddSpaceSecretParams {
                repo_id: self.repo.repo_path(),
                key: params.key.clone(),
                value: params.value.clone(),
                description: params.description.clone(),
            })
            .await
    }

    #[cfg(feature = "spaces")]
    pub async fn delete_secret(&self, params: &SpaceSecretDeleteParams) -> Result<()> {
        self.repo
            .client()
            .delete_space_secret(&DeleteSpaceSecretParams {
                repo_id: self.repo.repo_path(),
                key: params.key.clone(),
            })
            .await
    }

    #[cfg(feature = "spaces")]
    pub async fn add_variable(&self, params: &SpaceVariableParams) -> Result<()> {
        self.repo
            .client()
            .add_space_variable(&AddSpaceVariableParams {
                repo_id: self.repo.repo_path(),
                key: params.key.clone(),
                value: params.value.clone(),
                description: params.description.clone(),
            })
            .await
    }

    #[cfg(feature = "spaces")]
    pub async fn delete_variable(&self, params: &SpaceVariableDeleteParams) -> Result<()> {
        self.repo
            .client()
            .delete_space_variable(&DeleteSpaceVariableParams {
                repo_id: self.repo.repo_path(),
                key: params.key.clone(),
            })
            .await
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
