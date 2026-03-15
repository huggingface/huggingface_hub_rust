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
