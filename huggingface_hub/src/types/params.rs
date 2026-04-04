use typed_builder::TypedBuilder;

use super::repo::RepoType;

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
    #[builder(default, setter(into, strip_option))]
    pub pipeline_tag: Option<String>,
    #[builder(default, setter(strip_option))]
    pub full: Option<bool>,
    #[builder(default, setter(strip_option))]
    pub card_data: Option<bool>,
    #[builder(default, setter(strip_option))]
    pub fetch_config: Option<bool>,
    /// Cap on the total number of items returned.
    /// Pagination stops once this many items have been yielded.
    /// When less than 1000, also used as the server page size for efficiency.
    #[builder(default, setter(strip_option))]
    pub limit: Option<usize>,
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
    pub full: Option<bool>,
    /// Cap on the total number of items returned.
    /// Pagination stops once this many items have been yielded.
    /// When less than 1000, also used as the server page size for efficiency.
    #[builder(default, setter(strip_option))]
    pub limit: Option<usize>,
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
    pub full: Option<bool>,
    /// Cap on the total number of items returned.
    /// Pagination stops once this many items have been yielded.
    /// When less than 1000, also used as the server page size for efficiency.
    #[builder(default, setter(strip_option))]
    pub limit: Option<usize>,
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
pub struct MoveRepoParams {
    #[builder(setter(into))]
    pub from_id: String,
    #[builder(setter(into))]
    pub to_id: String,
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
pub struct DuplicateSpaceParams {
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

#[cfg(feature = "likes")]
#[derive(TypedBuilder)]
pub struct ListLikedReposParams {
    #[builder(setter(into))]
    pub username: String,
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
