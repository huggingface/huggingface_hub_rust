use crate::client::HfApi;
use crate::constants;
use crate::error::{HfError, Result};
use crate::types::{
    CreateRepoParams, DatasetInfo, DatasetInfoParams, DeleteRepoParams, FileExistsParams,
    ListDatasetsParams, ListModelsParams, ListSpacesParams, ModelInfo, ModelInfoParams,
    MoveRepoParams, RepoExistsParams, RepoType, RepoUrl, RevisionExistsParams, SpaceInfo,
    SpaceInfoParams, UpdateRepoParams,
};
use futures::Stream;
use url::Url;

impl HfApi {
    /// Get info about a model repository.
    /// Endpoint: GET /api/models/{repo_id} or /api/models/{repo_id}/revision/{revision}
    pub async fn model_info(&self, params: &ModelInfoParams) -> Result<ModelInfo> {
        let mut url = self.api_url(Some(RepoType::Model), &params.repo_id);
        if let Some(ref revision) = params.revision {
            url = format!("{url}/revision/{revision}");
        }
        let response = self
            .inner
            .client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await?;
        let response = self
            .check_response(
                response,
                Some(&params.repo_id),
                crate::error::NotFoundContext::Repo,
            )
            .await?;
        Ok(response.json().await?)
    }

    /// Get info about a dataset repository.
    /// Endpoint: GET /api/datasets/{repo_id} or /api/datasets/{repo_id}/revision/{revision}
    pub async fn dataset_info(&self, params: &DatasetInfoParams) -> Result<DatasetInfo> {
        let mut url = self.api_url(Some(RepoType::Dataset), &params.repo_id);
        if let Some(ref revision) = params.revision {
            url = format!("{url}/revision/{revision}");
        }
        let response = self
            .inner
            .client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await?;
        let response = self
            .check_response(
                response,
                Some(&params.repo_id),
                crate::error::NotFoundContext::Repo,
            )
            .await?;
        Ok(response.json().await?)
    }

    /// Get info about a space.
    /// Endpoint: GET /api/spaces/{repo_id} or /api/spaces/{repo_id}/revision/{revision}
    pub async fn space_info(&self, params: &SpaceInfoParams) -> Result<SpaceInfo> {
        let mut url = self.api_url(Some(RepoType::Space), &params.repo_id);
        if let Some(ref revision) = params.revision {
            url = format!("{url}/revision/{revision}");
        }
        let response = self
            .inner
            .client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await?;
        let response = self
            .check_response(
                response,
                Some(&params.repo_id),
                crate::error::NotFoundContext::Repo,
            )
            .await?;
        Ok(response.json().await?)
    }

    /// Check if a repository exists.
    pub async fn repo_exists(&self, params: &RepoExistsParams) -> Result<bool> {
        let url = self.api_url(params.repo_type, &params.repo_id);
        let response = self
            .inner
            .client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await?;
        match response.status().as_u16() {
            200..=299 => Ok(true),
            404 => Ok(false),
            401 => Err(HfError::AuthRequired),
            status => {
                let url = response.url().to_string();
                let body = response.text().await.unwrap_or_default();
                Err(HfError::Http {
                    status: reqwest::StatusCode::from_u16(status).unwrap(),
                    url,
                    body,
                })
            }
        }
    }

    /// Check if a specific revision exists in a repository.
    pub async fn revision_exists(&self, params: &RevisionExistsParams) -> Result<bool> {
        let url = format!(
            "{}/revision/{}",
            self.api_url(params.repo_type, &params.repo_id),
            params.revision
        );
        let response = self
            .inner
            .client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await?;
        match response.status().as_u16() {
            200..=299 => Ok(true),
            404 => Ok(false),
            401 => Err(HfError::AuthRequired),
            status => {
                let url_str = response.url().to_string();
                let body = response.text().await.unwrap_or_default();
                Err(HfError::Http {
                    status: reqwest::StatusCode::from_u16(status).unwrap(),
                    url: url_str,
                    body,
                })
            }
        }
    }

    /// Check if a file exists in a repository by sending a HEAD request
    /// to the download URL.
    pub async fn file_exists(&self, params: &FileExistsParams) -> Result<bool> {
        let revision = params
            .revision
            .as_deref()
            .unwrap_or(constants::DEFAULT_REVISION);
        let url = self.download_url(
            params.repo_type,
            &params.repo_id,
            revision,
            &params.filename,
        );
        let response = self
            .inner
            .client
            .head(&url)
            .headers(self.auth_headers())
            .send()
            .await?;
        match response.status().as_u16() {
            200..=299 => Ok(true),
            404 => Ok(false),
            401 => Err(HfError::AuthRequired),
            status => {
                let url_str = response.url().to_string();
                let body = response.text().await.unwrap_or_default();
                Err(HfError::Http {
                    status: reqwest::StatusCode::from_u16(status).unwrap(),
                    url: url_str,
                    body,
                })
            }
        }
    }
}

impl HfApi {
    /// List models on the Hub.
    /// Endpoint: GET /api/models
    pub fn list_models(
        &self,
        params: &ListModelsParams,
    ) -> impl Stream<Item = Result<ModelInfo>> + '_ {
        let url = Url::parse(&format!("{}/api/models", self.inner.endpoint)).unwrap();
        let mut query: Vec<(String, String)> = Vec::new();
        if let Some(ref search) = params.search {
            query.push(("search".into(), search.clone()));
        }
        if let Some(ref author) = params.author {
            query.push(("author".into(), author.clone()));
        }
        if let Some(ref filter) = params.filter {
            query.push(("filter".into(), filter.clone()));
        }
        if let Some(ref sort) = params.sort {
            query.push(("sort".into(), sort.clone()));
        }
        if let Some(limit) = params.limit {
            query.push(("limit".into(), limit.to_string()));
        }
        if let Some(ref pipeline_tag) = params.pipeline_tag {
            query.push(("pipeline_tag".into(), pipeline_tag.clone()));
        }
        if params.full == Some(true) {
            query.push(("full".into(), "true".into()));
        }
        if params.card_data == Some(true) {
            query.push(("cardData".into(), "true".into()));
        }
        if params.fetch_config == Some(true) {
            query.push(("config".into(), "true".into()));
        }
        self.paginate(url, query)
    }

    /// List datasets on the Hub.
    /// Endpoint: GET /api/datasets
    pub fn list_datasets(
        &self,
        params: &ListDatasetsParams,
    ) -> impl Stream<Item = Result<DatasetInfo>> + '_ {
        let url = Url::parse(&format!("{}/api/datasets", self.inner.endpoint)).unwrap();
        let mut query: Vec<(String, String)> = Vec::new();
        if let Some(ref search) = params.search {
            query.push(("search".into(), search.clone()));
        }
        if let Some(ref author) = params.author {
            query.push(("author".into(), author.clone()));
        }
        if let Some(ref filter) = params.filter {
            query.push(("filter".into(), filter.clone()));
        }
        if let Some(ref sort) = params.sort {
            query.push(("sort".into(), sort.clone()));
        }
        if let Some(limit) = params.limit {
            query.push(("limit".into(), limit.to_string()));
        }
        if params.full == Some(true) {
            query.push(("full".into(), "true".into()));
        }
        self.paginate(url, query)
    }

    /// List spaces on the Hub.
    /// Endpoint: GET /api/spaces
    pub fn list_spaces(
        &self,
        params: &ListSpacesParams,
    ) -> impl Stream<Item = Result<SpaceInfo>> + '_ {
        let url = Url::parse(&format!("{}/api/spaces", self.inner.endpoint)).unwrap();
        let mut query: Vec<(String, String)> = Vec::new();
        if let Some(ref search) = params.search {
            query.push(("search".into(), search.clone()));
        }
        if let Some(ref author) = params.author {
            query.push(("author".into(), author.clone()));
        }
        if let Some(ref filter) = params.filter {
            query.push(("filter".into(), filter.clone()));
        }
        if let Some(ref sort) = params.sort {
            query.push(("sort".into(), sort.clone()));
        }
        if let Some(limit) = params.limit {
            query.push(("limit".into(), limit.to_string()));
        }
        if params.full == Some(true) {
            query.push(("full".into(), "true".into()));
        }
        self.paginate(url, query)
    }

    /// Create a new repository.
    /// Endpoint: POST /api/repos/create
    pub async fn create_repo(&self, params: &CreateRepoParams) -> Result<RepoUrl> {
        let url = format!("{}/api/repos/create", self.inner.endpoint);

        let (namespace, name) = split_repo_id(&params.repo_id);

        let mut body = serde_json::json!({
            "name": name,
            "private": params.private.unwrap_or(false),
        });

        if let Some(ns) = namespace {
            body["organization"] = serde_json::Value::String(ns.to_string());
        }
        if let Some(ref repo_type) = params.repo_type {
            body["type"] = serde_json::Value::String(repo_type.to_string());
        }
        if let Some(ref sdk) = params.space_sdk {
            body["sdk"] = serde_json::Value::String(sdk.clone());
        }

        let response = self
            .inner
            .client
            .post(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;

        if response.status().as_u16() == 409 && params.exist_ok {
            // Already exists and exist_ok=true, return its URL
            let prefix = constants::repo_type_url_prefix(params.repo_type);
            return Ok(RepoUrl {
                url: format!("{}/{}{}", self.inner.endpoint, prefix, params.repo_id),
            });
        }

        let response = self
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    /// Delete a repository.
    /// Endpoint: DELETE /api/repos/delete
    pub async fn delete_repo(&self, params: &DeleteRepoParams) -> Result<()> {
        let url = format!("{}/api/repos/delete", self.inner.endpoint);

        let (namespace, name) = split_repo_id(&params.repo_id);

        let mut body = serde_json::json!({ "name": name });
        if let Some(ns) = namespace {
            body["organization"] = serde_json::Value::String(ns.to_string());
        }
        if let Some(ref repo_type) = params.repo_type {
            body["type"] = serde_json::Value::String(repo_type.to_string());
        }

        let response = self
            .inner
            .client
            .delete(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;

        if response.status().as_u16() == 404 && params.missing_ok {
            return Ok(());
        }

        self.check_response(
            response,
            Some(&params.repo_id),
            crate::error::NotFoundContext::Repo,
        )
        .await?;
        Ok(())
    }

    /// Update repository settings.
    /// Endpoint: PUT /api/{repo_type}s/{repo_id}/settings
    pub async fn update_repo_settings(&self, params: &UpdateRepoParams) -> Result<()> {
        let url = format!(
            "{}/settings",
            self.api_url(params.repo_type, &params.repo_id)
        );
        let mut body = serde_json::Map::new();

        if let Some(private) = params.private {
            body.insert("private".into(), serde_json::Value::Bool(private));
        }
        if let Some(ref gated) = params.gated {
            body.insert("gated".into(), serde_json::Value::String(gated.clone()));
        }
        if let Some(ref description) = params.description {
            body.insert(
                "description".into(),
                serde_json::Value::String(description.clone()),
            );
        }

        let response = self
            .inner
            .client
            .put(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;

        self.check_response(
            response,
            Some(&params.repo_id),
            crate::error::NotFoundContext::Repo,
        )
        .await?;
        Ok(())
    }

    /// Move (rename) a repository.
    /// Endpoint: POST /api/repos/move
    pub async fn move_repo(&self, params: &MoveRepoParams) -> Result<RepoUrl> {
        let url = format!("{}/api/repos/move", self.inner.endpoint);
        let mut body = serde_json::json!({
            "fromRepo": params.from_id,
            "toRepo": params.to_id,
        });
        if let Some(ref repo_type) = params.repo_type {
            body["type"] = serde_json::Value::String(repo_type.to_string());
        }

        let response = self
            .inner
            .client
            .post(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;

        self.check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        let prefix = constants::repo_type_url_prefix(params.repo_type);
        Ok(RepoUrl {
            url: format!("{}/{}{}", self.inner.endpoint, prefix, params.to_id),
        })
    }
}

/// Split "namespace/name" into (Some("namespace"), "name") or (None, "name")
fn split_repo_id(repo_id: &str) -> (Option<&str>, &str) {
    match repo_id.split_once('/') {
        Some((ns, name)) => (Some(ns), name),
        None => (None, repo_id),
    }
}

#[cfg(test)]
mod tests {
    use super::split_repo_id;

    #[test]
    fn test_split_repo_id() {
        assert_eq!(split_repo_id("user/repo"), (Some("user"), "repo"));
        assert_eq!(split_repo_id("repo"), (None, "repo"));
        assert_eq!(split_repo_id("org/sub/repo"), (Some("org"), "sub/repo"));
    }
}

sync_api! {
    impl HfApiSync {
        fn model_info(&self, params: &ModelInfoParams) -> Result<ModelInfo>;
        fn dataset_info(&self, params: &DatasetInfoParams) -> Result<DatasetInfo>;
        fn space_info(&self, params: &SpaceInfoParams) -> Result<SpaceInfo>;
        fn repo_exists(&self, params: &RepoExistsParams) -> Result<bool>;
        fn revision_exists(&self, params: &RevisionExistsParams) -> Result<bool>;
        fn file_exists(&self, params: &FileExistsParams) -> Result<bool>;
        fn create_repo(&self, params: &CreateRepoParams) -> Result<RepoUrl>;
        fn delete_repo(&self, params: &DeleteRepoParams) -> Result<()>;
        fn update_repo_settings(&self, params: &UpdateRepoParams) -> Result<()>;
        fn move_repo(&self, params: &MoveRepoParams) -> Result<RepoUrl>;
    }
}

sync_api_stream! {
    impl HfApiSync {
        fn list_models(&self, params: &ListModelsParams) -> ModelInfo;
        fn list_datasets(&self, params: &ListDatasetsParams) -> DatasetInfo;
        fn list_spaces(&self, params: &ListSpacesParams) -> SpaceInfo;
    }
}
