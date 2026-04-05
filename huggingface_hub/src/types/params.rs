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
