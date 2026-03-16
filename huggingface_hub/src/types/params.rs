use super::commit::{AddSource, CommitOperation};
use super::repo::RepoType;
use std::path::PathBuf;
use typed_builder::TypedBuilder;

#[derive(TypedBuilder)]
pub struct ModelInfoParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
}

#[derive(TypedBuilder)]
pub struct DatasetInfoParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
}

#[derive(TypedBuilder)]
pub struct SpaceInfoParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
}

#[derive(TypedBuilder)]
pub struct RepoExistsParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[derive(TypedBuilder)]
pub struct RevisionExistsParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub revision: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[derive(TypedBuilder)]
pub struct FileExistsParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub filename: String,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[derive(TypedBuilder)]
pub struct ListModelsParams {
    #[builder(default, setter(into, strip_option))]
    pub search: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub author: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub filter: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub sort: Option<String>,
    #[builder(default, setter(strip_option))]
    pub limit: Option<usize>,
    #[builder(default, setter(into, strip_option))]
    pub pipeline_tag: Option<String>,
    #[builder(default, setter(strip_option))]
    pub full: Option<bool>,
    #[builder(default, setter(strip_option))]
    pub card_data: Option<bool>,
    #[builder(default, setter(strip_option))]
    pub fetch_config: Option<bool>,
}

#[derive(TypedBuilder)]
pub struct ListDatasetsParams {
    #[builder(default, setter(into, strip_option))]
    pub search: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub author: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub filter: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub sort: Option<String>,
    #[builder(default, setter(strip_option))]
    pub limit: Option<usize>,
    #[builder(default, setter(strip_option))]
    pub full: Option<bool>,
}

#[derive(TypedBuilder)]
pub struct ListSpacesParams {
    #[builder(default, setter(into, strip_option))]
    pub search: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub author: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub filter: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub sort: Option<String>,
    #[builder(default, setter(strip_option))]
    pub limit: Option<usize>,
    #[builder(default, setter(strip_option))]
    pub full: Option<bool>,
}

#[derive(TypedBuilder)]
pub struct CreateRepoParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
    #[builder(default, setter(strip_option))]
    pub private: Option<bool>,
    #[builder(default)]
    pub exist_ok: bool,
    #[builder(default, setter(into, strip_option))]
    pub space_sdk: Option<String>,
}

#[derive(TypedBuilder)]
pub struct DeleteRepoParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
    #[builder(default)]
    pub missing_ok: bool,
}

#[derive(TypedBuilder)]
pub struct UpdateRepoParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
    #[builder(default, setter(strip_option))]
    pub private: Option<bool>,
    #[builder(default, setter(into, strip_option))]
    pub gated: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub description: Option<String>,
}

#[derive(TypedBuilder)]
pub struct MoveRepoParams {
    #[builder(setter(into))]
    pub from_id: String,
    #[builder(setter(into))]
    pub to_id: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[derive(TypedBuilder)]
pub struct ListRepoFilesParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[derive(TypedBuilder)]
pub struct ListRepoTreeParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
    #[builder(default)]
    pub recursive: bool,
    #[builder(default)]
    pub expand: bool,
}

#[derive(TypedBuilder)]
pub struct GetPathsInfoParams {
    #[builder(setter(into))]
    pub repo_id: String,
    pub paths: Vec<String>,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[derive(TypedBuilder)]
pub struct DownloadFileParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub filename: String,
    pub local_dir: PathBuf,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
}

#[derive(TypedBuilder)]
pub struct UploadFileParams {
    #[builder(setter(into))]
    pub repo_id: String,
    pub source: AddSource,
    #[builder(setter(into))]
    pub path_in_repo: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
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
pub struct UploadFolderParams {
    #[builder(setter(into))]
    pub repo_id: String,
    pub folder_path: PathBuf,
    #[builder(default, setter(into, strip_option))]
    pub path_in_repo: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub commit_message: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub commit_description: Option<String>,
    #[builder(default, setter(strip_option))]
    pub create_pr: Option<bool>,
    #[builder(default, setter(into, strip_option))]
    pub allow_patterns: Option<Vec<String>>,
    #[builder(default, setter(into, strip_option))]
    pub ignore_patterns: Option<Vec<String>>,
    #[builder(default, setter(into, strip_option))]
    pub delete_patterns: Option<Vec<String>>,
}

#[derive(TypedBuilder)]
pub struct DeleteFileParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub path_in_repo: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub commit_message: Option<String>,
    #[builder(default, setter(strip_option))]
    pub create_pr: Option<bool>,
}

#[derive(TypedBuilder)]
pub struct DeleteFolderParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub path_in_repo: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub commit_message: Option<String>,
    #[builder(default, setter(strip_option))]
    pub create_pr: Option<bool>,
}

#[derive(TypedBuilder)]
pub struct CreateCommitParams {
    #[builder(setter(into))]
    pub repo_id: String,
    pub operations: Vec<CommitOperation>,
    #[builder(setter(into))]
    pub commit_message: String,
    #[builder(default, setter(into, strip_option))]
    pub commit_description: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(strip_option))]
    pub create_pr: Option<bool>,
    #[builder(default, setter(into, strip_option))]
    pub parent_commit: Option<String>,
}

#[derive(TypedBuilder)]
pub struct ListRepoCommitsParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[derive(TypedBuilder)]
pub struct ListRepoRefsParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
    #[builder(default)]
    pub include_pull_requests: bool,
}

#[derive(TypedBuilder)]
pub struct GetCommitDiffParams {
    #[builder(setter(into))]
    pub repo_id: String,
    /// Revision range in the format "revA...revB"
    #[builder(setter(into))]
    pub compare: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[derive(TypedBuilder)]
pub struct GetRawDiffParams {
    #[builder(setter(into))]
    pub repo_id: String,
    /// Revision range in the format "revA...revB"
    #[builder(setter(into))]
    pub compare: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[derive(TypedBuilder)]
pub struct CreateBranchParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub branch: String,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[derive(TypedBuilder)]
pub struct DeleteBranchParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub branch: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[derive(TypedBuilder)]
pub struct CreateTagParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub tag: String,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub message: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[derive(TypedBuilder)]
pub struct DeleteTagParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub tag: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XetTokenType {
    Read,
    Write,
}

impl XetTokenType {
    pub fn as_str(&self) -> &'static str {
        match self {
            XetTokenType::Read => "read",
            XetTokenType::Write => "write",
        }
    }
}

#[derive(TypedBuilder)]
pub struct GetXetTokenParams {
    #[builder(setter(into))]
    pub repo_id: String,
    pub token_type: XetTokenType,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
}

#[cfg(feature = "spaces")]
#[derive(TypedBuilder)]
pub struct GetSpaceRuntimeParams {
    #[builder(setter(into))]
    pub repo_id: String,
}

#[cfg(feature = "spaces")]
#[derive(TypedBuilder)]
pub struct RequestSpaceHardwareParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub hardware: String,
    #[builder(default, setter(strip_option))]
    pub sleep_time: Option<u64>,
}

#[cfg(feature = "spaces")]
#[derive(TypedBuilder)]
pub struct SetSpaceSleepTimeParams {
    #[builder(setter(into))]
    pub repo_id: String,
    pub sleep_time: u64,
}

#[cfg(feature = "spaces")]
#[derive(TypedBuilder)]
pub struct PauseSpaceParams {
    #[builder(setter(into))]
    pub repo_id: String,
}

#[cfg(feature = "spaces")]
#[derive(TypedBuilder)]
pub struct RestartSpaceParams {
    #[builder(setter(into))]
    pub repo_id: String,
}

#[cfg(feature = "spaces")]
#[derive(TypedBuilder)]
pub struct AddSpaceSecretParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub key: String,
    #[builder(setter(into))]
    pub value: String,
    #[builder(default, setter(into, strip_option))]
    pub description: Option<String>,
}

#[cfg(feature = "spaces")]
#[derive(TypedBuilder)]
pub struct DeleteSpaceSecretParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub key: String,
}

#[cfg(feature = "spaces")]
#[derive(TypedBuilder)]
pub struct AddSpaceVariableParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub key: String,
    #[builder(setter(into))]
    pub value: String,
    #[builder(default, setter(into, strip_option))]
    pub description: Option<String>,
}

#[cfg(feature = "spaces")]
#[derive(TypedBuilder)]
pub struct DeleteSpaceVariableParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub key: String,
}

#[cfg(feature = "spaces")]
#[derive(TypedBuilder)]
pub struct DuplicateSpaceParams {
    #[builder(setter(into))]
    pub from_id: String,
    #[builder(default, setter(into, strip_option))]
    pub to_id: Option<String>,
    #[builder(default, setter(strip_option))]
    pub private: Option<bool>,
    #[builder(default, setter(into, strip_option))]
    pub hardware: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub storage: Option<String>,
    #[builder(default, setter(strip_option))]
    pub sleep_time: Option<u64>,
    #[builder(default, setter(into, strip_option))]
    pub secrets: Option<Vec<serde_json::Value>>,
    #[builder(default, setter(into, strip_option))]
    pub variables: Option<Vec<serde_json::Value>>,
}

#[cfg(feature = "inference_endpoints")]
#[derive(TypedBuilder)]
pub struct CreateInferenceEndpointParams {
    #[builder(setter(into))]
    pub name: String,
    #[builder(setter(into))]
    pub repository: String,
    #[builder(setter(into))]
    pub framework: String,
    #[builder(setter(into))]
    pub task: String,
    #[builder(setter(into))]
    pub accelerator: String,
    #[builder(setter(into))]
    pub instance_size: String,
    #[builder(setter(into))]
    pub instance_type: String,
    #[builder(setter(into))]
    pub region: String,
    #[builder(setter(into))]
    pub vendor: String,
    #[builder(default, setter(into, strip_option))]
    pub namespace: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(strip_option))]
    pub min_replica: Option<u32>,
    #[builder(default, setter(strip_option))]
    pub max_replica: Option<u32>,
    #[builder(default, setter(strip_option))]
    pub scale_to_zero_timeout: Option<u32>,
    #[builder(default, setter(into, strip_option))]
    pub endpoint_type: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub custom_image: Option<serde_json::Value>,
    #[builder(default, setter(into, strip_option))]
    pub secrets: Option<std::collections::HashMap<String, String>>,
}

#[cfg(feature = "inference_endpoints")]
#[derive(TypedBuilder)]
pub struct GetInferenceEndpointParams {
    #[builder(setter(into))]
    pub name: String,
    #[builder(default, setter(into, strip_option))]
    pub namespace: Option<String>,
}

#[cfg(feature = "inference_endpoints")]
#[derive(TypedBuilder)]
pub struct ListInferenceEndpointsParams {
    #[builder(default, setter(into, strip_option))]
    pub namespace: Option<String>,
}

#[cfg(feature = "inference_endpoints")]
#[derive(TypedBuilder)]
pub struct UpdateInferenceEndpointParams {
    #[builder(setter(into))]
    pub name: String,
    #[builder(default, setter(into, strip_option))]
    pub namespace: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub accelerator: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub instance_size: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub instance_type: Option<String>,
    #[builder(default, setter(strip_option))]
    pub min_replica: Option<u32>,
    #[builder(default, setter(strip_option))]
    pub max_replica: Option<u32>,
    #[builder(default, setter(strip_option))]
    pub scale_to_zero_timeout: Option<u32>,
    #[builder(default, setter(into, strip_option))]
    pub repository: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub framework: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub task: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub custom_image: Option<serde_json::Value>,
    #[builder(default, setter(into, strip_option))]
    pub secrets: Option<std::collections::HashMap<String, String>>,
}

#[cfg(feature = "inference_endpoints")]
#[derive(TypedBuilder)]
pub struct DeleteInferenceEndpointParams {
    #[builder(setter(into))]
    pub name: String,
    #[builder(default, setter(into, strip_option))]
    pub namespace: Option<String>,
}

#[cfg(feature = "inference_endpoints")]
#[derive(TypedBuilder)]
pub struct PauseInferenceEndpointParams {
    #[builder(setter(into))]
    pub name: String,
    #[builder(default, setter(into, strip_option))]
    pub namespace: Option<String>,
}

#[cfg(feature = "inference_endpoints")]
#[derive(TypedBuilder)]
pub struct ResumeInferenceEndpointParams {
    #[builder(setter(into))]
    pub name: String,
    #[builder(default, setter(into, strip_option))]
    pub namespace: Option<String>,
}

#[cfg(feature = "inference_endpoints")]
#[derive(TypedBuilder)]
pub struct ScaleToZeroInferenceEndpointParams {
    #[builder(setter(into))]
    pub name: String,
    #[builder(default, setter(into, strip_option))]
    pub namespace: Option<String>,
}

#[cfg(feature = "collections")]
#[derive(TypedBuilder)]
pub struct GetCollectionParams {
    #[builder(setter(into))]
    pub slug: String,
}

#[cfg(feature = "collections")]
#[derive(TypedBuilder)]
pub struct ListCollectionsParams {
    #[builder(default, setter(into, strip_option))]
    pub owner: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub item: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub item_type: Option<String>,
    #[builder(default, setter(strip_option))]
    pub limit: Option<usize>,
    #[builder(default, setter(strip_option))]
    pub offset: Option<usize>,
}

#[cfg(feature = "collections")]
#[derive(TypedBuilder)]
pub struct CreateCollectionParams {
    #[builder(setter(into))]
    pub title: String,
    #[builder(default, setter(into, strip_option))]
    pub description: Option<String>,
    #[builder(default, setter(strip_option))]
    pub private: Option<bool>,
    #[builder(default, setter(into, strip_option))]
    pub namespace: Option<String>,
}

#[cfg(feature = "collections")]
#[derive(TypedBuilder)]
pub struct UpdateCollectionMetadataParams {
    #[builder(setter(into))]
    pub slug: String,
    #[builder(default, setter(into, strip_option))]
    pub title: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub description: Option<String>,
    #[builder(default, setter(strip_option))]
    pub private: Option<bool>,
    #[builder(default, setter(strip_option))]
    pub position: Option<i64>,
    #[builder(default, setter(into, strip_option))]
    pub theme: Option<String>,
}

#[cfg(feature = "collections")]
#[derive(TypedBuilder)]
pub struct DeleteCollectionParams {
    #[builder(setter(into))]
    pub slug: String,
    #[builder(default)]
    pub missing_ok: bool,
}

#[cfg(feature = "collections")]
#[derive(TypedBuilder)]
pub struct AddCollectionItemParams {
    #[builder(setter(into))]
    pub slug: String,
    #[builder(setter(into))]
    pub item_id: String,
    #[builder(setter(into))]
    pub item_type: String,
    #[builder(default, setter(into, strip_option))]
    pub note: Option<String>,
    #[builder(default)]
    pub exists_ok: bool,
}

#[cfg(feature = "collections")]
#[derive(TypedBuilder)]
pub struct UpdateCollectionItemParams {
    #[builder(setter(into))]
    pub slug: String,
    #[builder(setter(into))]
    pub item_object_id: String,
    #[builder(default, setter(into, strip_option))]
    pub note: Option<String>,
    #[builder(default, setter(strip_option))]
    pub position: Option<i64>,
}

#[cfg(feature = "collections")]
#[derive(TypedBuilder)]
pub struct DeleteCollectionItemParams {
    #[builder(setter(into))]
    pub slug: String,
    #[builder(setter(into))]
    pub item_object_id: String,
}

#[cfg(feature = "discussions")]
#[derive(TypedBuilder)]
pub struct GetRepoDiscussionsParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
    #[builder(default, setter(into, strip_option))]
    pub author: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub discussion_type: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub discussion_status: Option<String>,
}

#[cfg(feature = "discussions")]
#[derive(TypedBuilder)]
pub struct GetDiscussionDetailsParams {
    #[builder(setter(into))]
    pub repo_id: String,
    pub discussion_num: u64,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[cfg(feature = "discussions")]
#[derive(TypedBuilder)]
pub struct CreateDiscussionParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub title: String,
    #[builder(default, setter(into, strip_option))]
    pub description: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[cfg(feature = "discussions")]
#[derive(TypedBuilder)]
pub struct CreatePullRequestParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub title: String,
    #[builder(default, setter(into, strip_option))]
    pub description: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[cfg(feature = "discussions")]
#[derive(TypedBuilder)]
pub struct CommentDiscussionParams {
    #[builder(setter(into))]
    pub repo_id: String,
    pub discussion_num: u64,
    #[builder(setter(into))]
    pub comment: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[cfg(feature = "discussions")]
#[derive(TypedBuilder)]
pub struct EditDiscussionCommentParams {
    #[builder(setter(into))]
    pub repo_id: String,
    pub discussion_num: u64,
    #[builder(setter(into))]
    pub comment_id: String,
    #[builder(setter(into))]
    pub new_content: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[cfg(feature = "discussions")]
#[derive(TypedBuilder)]
pub struct HideDiscussionCommentParams {
    #[builder(setter(into))]
    pub repo_id: String,
    pub discussion_num: u64,
    #[builder(setter(into))]
    pub comment_id: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[cfg(feature = "discussions")]
#[derive(TypedBuilder)]
pub struct RenameDiscussionParams {
    #[builder(setter(into))]
    pub repo_id: String,
    pub discussion_num: u64,
    #[builder(setter(into))]
    pub new_title: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[cfg(feature = "discussions")]
#[derive(TypedBuilder)]
pub struct ChangeDiscussionStatusParams {
    #[builder(setter(into))]
    pub repo_id: String,
    pub discussion_num: u64,
    #[builder(setter(into))]
    pub new_status: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[cfg(feature = "discussions")]
#[derive(TypedBuilder)]
pub struct MergePullRequestParams {
    #[builder(setter(into))]
    pub repo_id: String,
    pub discussion_num: u64,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[cfg(feature = "webhooks")]
#[derive(TypedBuilder)]
pub struct CreateWebhookParams {
    #[builder(setter(into))]
    pub url: String,
    pub watched: Vec<serde_json::Value>,
    #[builder(default, setter(into, strip_option))]
    pub domains: Option<Vec<String>>,
    #[builder(default, setter(into, strip_option))]
    pub secret: Option<String>,
}

#[cfg(feature = "webhooks")]
#[derive(TypedBuilder)]
pub struct UpdateWebhookParams {
    #[builder(setter(into))]
    pub webhook_id: String,
    #[builder(default, setter(into, strip_option))]
    pub url: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub watched: Option<Vec<serde_json::Value>>,
    #[builder(default, setter(into, strip_option))]
    pub domains: Option<Vec<String>>,
    #[builder(default, setter(into, strip_option))]
    pub secret: Option<String>,
}

#[cfg(feature = "jobs")]
#[derive(TypedBuilder)]
pub struct RunJobParams {
    #[builder(setter(into))]
    pub image: String,
    pub command: Vec<String>,
    #[builder(default, setter(into, strip_option))]
    pub flavor: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub env: Option<std::collections::HashMap<String, String>>,
    #[builder(default, setter(into, strip_option))]
    pub secrets: Option<std::collections::HashMap<String, String>>,
    #[builder(default, setter(into, strip_option))]
    pub timeout: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub labels: Option<std::collections::HashMap<String, String>>,
    #[builder(default, setter(into, strip_option))]
    pub namespace: Option<String>,
}

#[cfg(feature = "jobs")]
#[derive(TypedBuilder)]
pub struct ListJobsParams {
    #[builder(default, setter(into, strip_option))]
    pub namespace: Option<String>,
}

#[cfg(feature = "jobs")]
#[derive(TypedBuilder)]
pub struct CreateScheduledJobParams {
    #[builder(setter(into))]
    pub image: String,
    pub command: Vec<String>,
    #[builder(setter(into))]
    pub schedule: String,
    #[builder(default, setter(into, strip_option))]
    pub flavor: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub env: Option<std::collections::HashMap<String, String>>,
    #[builder(default, setter(into, strip_option))]
    pub secrets: Option<std::collections::HashMap<String, String>>,
    #[builder(default, setter(into, strip_option))]
    pub timeout: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub namespace: Option<String>,
    #[builder(default, setter(strip_option))]
    pub suspend: Option<bool>,
    #[builder(default, setter(strip_option))]
    pub concurrency: Option<bool>,
}

#[cfg(feature = "access_requests")]
#[derive(TypedBuilder)]
pub struct ListAccessRequestsParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[cfg(feature = "access_requests")]
#[derive(TypedBuilder)]
pub struct HandleAccessRequestParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub user: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[cfg(feature = "access_requests")]
#[derive(TypedBuilder)]
pub struct GrantAccessParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub user: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[cfg(feature = "likes")]
#[derive(TypedBuilder)]
pub struct LikeParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[cfg(feature = "likes")]
#[derive(TypedBuilder)]
pub struct ListLikedReposParams {
    #[builder(setter(into))]
    pub username: String,
}

#[cfg(feature = "likes")]
#[derive(TypedBuilder)]
pub struct ListRepoLikersParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[cfg(feature = "papers")]
#[derive(TypedBuilder)]
pub struct ListPapersParams {
    #[builder(default, setter(into, strip_option))]
    pub query: Option<String>,
    #[builder(default, setter(strip_option))]
    pub limit: Option<usize>,
}

#[cfg(feature = "papers")]
#[derive(TypedBuilder)]
pub struct ListDailyPapersParams {
    #[builder(default, setter(into, strip_option))]
    pub date: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub week: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub month: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub submitter: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub sort: Option<String>,
    #[builder(default, setter(strip_option))]
    pub p: Option<usize>,
    #[builder(default, setter(strip_option))]
    pub limit: Option<usize>,
}

#[cfg(feature = "papers")]
#[derive(TypedBuilder)]
pub struct PaperInfoParams {
    #[builder(setter(into))]
    pub paper_id: String,
}
