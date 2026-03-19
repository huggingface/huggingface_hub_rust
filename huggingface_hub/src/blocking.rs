use std::path::PathBuf;

use crate::client::HfApi;
use crate::error::{HfError, Result};
use crate::types::commit::{CommitInfo, GitCommitInfo, GitRefs};
use crate::types::params::*;
use crate::types::repo::{DatasetInfo, ModelInfo, RepoTreeEntry, RepoUrl, SpaceInfo};
use crate::types::user::{Organization, User};

pub struct HfApiSync {
    inner: HfApi,
    runtime: tokio::runtime::Runtime,
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

macro_rules! sync_method {
    ($method:ident () -> $ret:ty) => {
        pub fn $method(&self) -> $ret {
            self.runtime.block_on(self.inner.$method())
        }
    };
    ($method:ident ($p:ident : & $pt:ty) -> $ret:ty) => {
        pub fn $method(&self, $p: &$pt) -> $ret {
            self.runtime.block_on(self.inner.$method($p))
        }
    };
    ($method:ident ($p:ident : &str) -> $ret:ty) => {
        pub fn $method(&self, $p: &str) -> $ret {
            self.runtime.block_on(self.inner.$method($p))
        }
    };
    ($method:ident ($p1:ident : &str, $p2:ident : Option<&str>) -> $ret:ty) => {
        pub fn $method(&self, $p1: &str, $p2: Option<&str>) -> $ret {
            self.runtime.block_on(self.inner.$method($p1, $p2))
        }
    };
}

macro_rules! sync_stream {
    ($method:ident ($p:ident : & $pt:ty) -> $item:ty) => {
        pub fn $method(&self, $p: &$pt) -> Result<Vec<$item>> {
            use futures::StreamExt;
            self.runtime.block_on(async {
                let stream = self.inner.$method($p);
                futures::pin_mut!(stream);
                let mut items = Vec::new();
                while let Some(item) = stream.next().await {
                    items.push(item?);
                }
                Ok(items)
            })
        }
    };
    ($method:ident ($p:ident : &str) -> $item:ty) => {
        pub fn $method(&self, $p: &str) -> Result<Vec<$item>> {
            use futures::StreamExt;
            self.runtime.block_on(async {
                let stream = self.inner.$method($p);
                futures::pin_mut!(stream);
                let mut items = Vec::new();
                while let Some(item) = stream.next().await {
                    items.push(item?);
                }
                Ok(items)
            })
        }
    };
}

// --- Core: repo ---
impl HfApiSync {
    sync_method!(model_info(params: &ModelInfoParams) -> Result<ModelInfo>);
    sync_method!(dataset_info(params: &DatasetInfoParams) -> Result<DatasetInfo>);
    sync_method!(space_info(params: &SpaceInfoParams) -> Result<SpaceInfo>);
    sync_method!(repo_exists(params: &RepoExistsParams) -> Result<bool>);
    sync_method!(revision_exists(params: &RevisionExistsParams) -> Result<bool>);
    sync_method!(file_exists(params: &FileExistsParams) -> Result<bool>);
    sync_stream!(list_models(params: &ListModelsParams) -> ModelInfo);
    sync_stream!(list_datasets(params: &ListDatasetsParams) -> DatasetInfo);
    sync_stream!(list_spaces(params: &ListSpacesParams) -> SpaceInfo);
    sync_method!(create_repo(params: &CreateRepoParams) -> Result<RepoUrl>);
    sync_method!(delete_repo(params: &DeleteRepoParams) -> Result<()>);
    sync_method!(update_repo_settings(params: &UpdateRepoParams) -> Result<()>);
    sync_method!(move_repo(params: &MoveRepoParams) -> Result<RepoUrl>);
}

// --- Core: files ---
impl HfApiSync {
    sync_method!(list_repo_files(params: &ListRepoFilesParams) -> Result<Vec<String>>);
    sync_stream!(list_repo_tree(params: &ListRepoTreeParams) -> RepoTreeEntry);
    sync_method!(get_paths_info(params: &GetPathsInfoParams) -> Result<Vec<RepoTreeEntry>>);
    sync_method!(download_file(params: &DownloadFileParams) -> Result<PathBuf>);
    sync_method!(create_commit(params: &CreateCommitParams) -> Result<CommitInfo>);
    sync_method!(upload_file(params: &UploadFileParams) -> Result<CommitInfo>);
    sync_method!(upload_folder(params: &UploadFolderParams) -> Result<CommitInfo>);
    sync_method!(delete_file(params: &DeleteFileParams) -> Result<CommitInfo>);
    sync_method!(delete_folder(params: &DeleteFolderParams) -> Result<CommitInfo>);
}

// --- Core: commits ---
impl HfApiSync {
    sync_stream!(list_repo_commits(params: &ListRepoCommitsParams) -> GitCommitInfo);
    sync_method!(list_repo_refs(params: &ListRepoRefsParams) -> Result<GitRefs>);
    sync_method!(get_commit_diff(params: &GetCommitDiffParams) -> Result<String>);
    sync_method!(get_raw_diff(params: &GetRawDiffParams) -> Result<String>);
    sync_method!(create_branch(params: &CreateBranchParams) -> Result<()>);
    sync_method!(delete_branch(params: &DeleteBranchParams) -> Result<()>);
    sync_method!(create_tag(params: &CreateTagParams) -> Result<()>);
    sync_method!(delete_tag(params: &DeleteTagParams) -> Result<()>);
}

// --- Core: users ---
impl HfApiSync {
    sync_method!(whoami() -> Result<User>);
    sync_method!(auth_check() -> Result<()>);
    sync_method!(get_user_overview(username: &str) -> Result<User>);
    sync_method!(get_organization_overview(organization: &str) -> Result<Organization>);
    sync_stream!(list_user_followers(username: &str) -> User);
    sync_stream!(list_user_following(username: &str) -> User);
    sync_stream!(list_organization_members(organization: &str) -> User);
}

// --- Feature: spaces ---
#[cfg(feature = "spaces")]
impl HfApiSync {
    sync_method!(get_space_runtime(params: &GetSpaceRuntimeParams) -> Result<crate::types::spaces::SpaceRuntime>);
    sync_method!(request_space_hardware(params: &RequestSpaceHardwareParams) -> Result<crate::types::spaces::SpaceRuntime>);
    sync_method!(set_space_sleep_time(params: &SetSpaceSleepTimeParams) -> Result<()>);
    sync_method!(pause_space(params: &PauseSpaceParams) -> Result<crate::types::spaces::SpaceRuntime>);
    sync_method!(restart_space(params: &RestartSpaceParams) -> Result<crate::types::spaces::SpaceRuntime>);
    sync_method!(add_space_secret(params: &AddSpaceSecretParams) -> Result<()>);
    sync_method!(delete_space_secret(params: &DeleteSpaceSecretParams) -> Result<()>);
    sync_method!(add_space_variable(params: &AddSpaceVariableParams) -> Result<()>);
    sync_method!(delete_space_variable(params: &DeleteSpaceVariableParams) -> Result<()>);
    sync_method!(duplicate_space(params: &DuplicateSpaceParams) -> Result<RepoUrl>);
}

// --- Feature: inference_endpoints ---
#[cfg(feature = "inference_endpoints")]
impl HfApiSync {
    sync_method!(create_inference_endpoint(params: &CreateInferenceEndpointParams) -> Result<crate::types::inference_endpoints::InferenceEndpointInfo>);
    sync_method!(get_inference_endpoint(params: &GetInferenceEndpointParams) -> Result<crate::types::inference_endpoints::InferenceEndpointInfo>);
    sync_method!(list_inference_endpoints(params: &ListInferenceEndpointsParams) -> Result<Vec<crate::types::inference_endpoints::InferenceEndpointInfo>>);
    sync_method!(update_inference_endpoint(params: &UpdateInferenceEndpointParams) -> Result<crate::types::inference_endpoints::InferenceEndpointInfo>);
    sync_method!(delete_inference_endpoint(params: &DeleteInferenceEndpointParams) -> Result<()>);
    sync_method!(pause_inference_endpoint(params: &PauseInferenceEndpointParams) -> Result<crate::types::inference_endpoints::InferenceEndpointInfo>);
    sync_method!(resume_inference_endpoint(params: &ResumeInferenceEndpointParams) -> Result<crate::types::inference_endpoints::InferenceEndpointInfo>);
    sync_method!(scale_to_zero_inference_endpoint(params: &ScaleToZeroInferenceEndpointParams) -> Result<crate::types::inference_endpoints::InferenceEndpointInfo>);
}

// --- Feature: collections ---
#[cfg(feature = "collections")]
impl HfApiSync {
    sync_method!(get_collection(params: &GetCollectionParams) -> Result<crate::types::collections::Collection>);
    sync_method!(list_collections(params: &ListCollectionsParams) -> Result<Vec<crate::types::collections::Collection>>);
    sync_method!(create_collection(params: &CreateCollectionParams) -> Result<crate::types::collections::Collection>);
    sync_method!(update_collection_metadata(params: &UpdateCollectionMetadataParams) -> Result<crate::types::collections::Collection>);
    sync_method!(delete_collection(params: &DeleteCollectionParams) -> Result<()>);
    sync_method!(add_collection_item(params: &AddCollectionItemParams) -> Result<crate::types::collections::Collection>);
    sync_method!(update_collection_item(params: &UpdateCollectionItemParams) -> Result<crate::types::collections::CollectionItem>);
    sync_method!(delete_collection_item(params: &DeleteCollectionItemParams) -> Result<()>);
}

// --- Feature: discussions ---
#[cfg(feature = "discussions")]
impl HfApiSync {
    sync_method!(get_repo_discussions(params: &GetRepoDiscussionsParams) -> Result<crate::types::discussions::DiscussionsResponse>);
    sync_method!(get_discussion_details(params: &GetDiscussionDetailsParams) -> Result<crate::types::discussions::DiscussionWithDetails>);
    sync_method!(create_discussion(params: &CreateDiscussionParams) -> Result<crate::types::discussions::DiscussionWithDetails>);
    sync_method!(create_pull_request(params: &CreatePullRequestParams) -> Result<crate::types::discussions::DiscussionWithDetails>);
    sync_method!(comment_discussion(params: &CommentDiscussionParams) -> Result<crate::types::discussions::DiscussionComment>);
    sync_method!(edit_discussion_comment(params: &EditDiscussionCommentParams) -> Result<crate::types::discussions::DiscussionComment>);
    sync_method!(hide_discussion_comment(params: &HideDiscussionCommentParams) -> Result<crate::types::discussions::DiscussionComment>);
    sync_method!(rename_discussion(params: &RenameDiscussionParams) -> Result<crate::types::discussions::DiscussionWithDetails>);
    sync_method!(change_discussion_status(params: &ChangeDiscussionStatusParams) -> Result<crate::types::discussions::DiscussionWithDetails>);
    sync_method!(merge_pull_request(params: &MergePullRequestParams) -> Result<crate::types::discussions::DiscussionWithDetails>);
}

// --- Feature: webhooks ---
#[cfg(feature = "webhooks")]
impl HfApiSync {
    sync_method!(list_webhooks() -> Result<Vec<crate::types::webhooks::WebhookInfo>>);
    sync_method!(get_webhook(webhook_id: &str) -> Result<crate::types::webhooks::WebhookInfo>);
    sync_method!(create_webhook(params: &CreateWebhookParams) -> Result<crate::types::webhooks::WebhookInfo>);
    sync_method!(update_webhook(params: &UpdateWebhookParams) -> Result<crate::types::webhooks::WebhookInfo>);
    sync_method!(delete_webhook(webhook_id: &str) -> Result<()>);
    sync_method!(enable_webhook(webhook_id: &str) -> Result<crate::types::webhooks::WebhookInfo>);
    sync_method!(disable_webhook(webhook_id: &str) -> Result<crate::types::webhooks::WebhookInfo>);
}

// --- Feature: jobs ---
#[cfg(feature = "jobs")]
impl HfApiSync {
    sync_method!(run_job(params: &RunJobParams) -> Result<crate::types::jobs::JobInfo>);
    sync_method!(list_jobs(params: &ListJobsParams) -> Result<Vec<crate::types::jobs::JobInfo>>);
    sync_method!(inspect_job(job_id: &str, namespace: Option<&str>) -> Result<crate::types::jobs::JobInfo>);
    sync_method!(cancel_job(job_id: &str, namespace: Option<&str>) -> Result<crate::types::jobs::JobInfo>);
    sync_method!(fetch_job_logs(job_id: &str, namespace: Option<&str>) -> Result<Vec<crate::types::jobs::JobLogEntry>>);
    sync_method!(fetch_job_metrics(job_id: &str, namespace: Option<&str>) -> Result<Vec<crate::types::jobs::JobMetrics>>);
    sync_method!(list_job_hardware() -> Result<Vec<crate::types::jobs::JobHardware>>);
    sync_method!(create_scheduled_job(params: &CreateScheduledJobParams) -> Result<crate::types::jobs::ScheduledJobInfo>);
    sync_method!(list_scheduled_jobs() -> Result<Vec<crate::types::jobs::ScheduledJobInfo>>);
    sync_method!(inspect_scheduled_job(scheduled_job_id: &str) -> Result<crate::types::jobs::ScheduledJobInfo>);
    sync_method!(delete_scheduled_job(scheduled_job_id: &str) -> Result<()>);
    sync_method!(suspend_scheduled_job(scheduled_job_id: &str) -> Result<crate::types::jobs::ScheduledJobInfo>);
    sync_method!(resume_scheduled_job(scheduled_job_id: &str) -> Result<crate::types::jobs::ScheduledJobInfo>);
}

// --- Feature: access_requests ---
#[cfg(feature = "access_requests")]
impl HfApiSync {
    sync_method!(list_pending_access_requests(params: &ListAccessRequestsParams) -> Result<Vec<crate::types::access_requests::AccessRequest>>);
    sync_method!(list_accepted_access_requests(params: &ListAccessRequestsParams) -> Result<Vec<crate::types::access_requests::AccessRequest>>);
    sync_method!(list_rejected_access_requests(params: &ListAccessRequestsParams) -> Result<Vec<crate::types::access_requests::AccessRequest>>);
    sync_method!(accept_access_request(params: &HandleAccessRequestParams) -> Result<()>);
    sync_method!(reject_access_request(params: &HandleAccessRequestParams) -> Result<()>);
    sync_method!(cancel_access_request(params: &HandleAccessRequestParams) -> Result<()>);
    sync_method!(grant_access(params: &GrantAccessParams) -> Result<()>);
}

// --- Feature: likes ---
#[cfg(feature = "likes")]
impl HfApiSync {
    sync_method!(like(params: &LikeParams) -> Result<()>);
    sync_method!(unlike(params: &LikeParams) -> Result<()>);
    sync_method!(list_liked_repos(params: &ListLikedReposParams) -> Result<Vec<crate::types::likes::LikedRepo>>);
    sync_stream!(list_repo_likers(params: &ListRepoLikersParams) -> User);
}

// --- Feature: papers ---
#[cfg(feature = "papers")]
impl HfApiSync {
    sync_method!(list_papers(params: &ListPapersParams) -> Result<Vec<crate::types::papers::PaperSearchResult>>);
    sync_method!(list_daily_papers(params: &ListDailyPapersParams) -> Result<Vec<crate::types::papers::DailyPaper>>);
    sync_method!(paper_info(params: &PaperInfoParams) -> Result<crate::types::papers::PaperInfo>);
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

    #[test]
    fn test_hfapisync_creation() {
        let sync_api = HfApiSync::new().unwrap();
        let x = sync_api.get_collection(GetCollectionParams::builder().build());

    }    
}
