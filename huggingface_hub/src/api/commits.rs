use futures::Stream;
use url::Url;

use crate::constants;
use crate::error::Result;
use crate::repository::HFRepository;
use crate::types::{
    CreateBranchParams, CreateTagParams, DeleteBranchParams, DeleteTagParams, GetCommitDiffParams, GetRawDiffParams,
    GitCommitInfo, GitRefs, ListRepoCommitsParams, ListRepoRefsParams,
};

impl HFRepository {
    /// List commits in a repository.
    /// Endpoint: GET /api/{repo_type}s/{repo_id}/commits/{revision}
    pub fn list_repo_commits(
        &self,
        params: &ListRepoCommitsParams,
    ) -> Result<impl Stream<Item = Result<GitCommitInfo>> + '_> {
        let revision = params.revision.as_deref().unwrap_or(constants::DEFAULT_REVISION);
        let url_str = format!("{}/commits/{}", self.client.api_url(params.repo_type, &params.repo_id), revision);
        let url = Url::parse(&url_str)?;
        Ok(self.client.paginate(url, vec![], params.max_items))
    }

    /// List branches, tags, and (optionally) pull requests.
    /// Endpoint: GET /api/{repo_type}s/{repo_id}/refs
    pub async fn list_repo_refs(&self, params: &ListRepoRefsParams) -> Result<GitRefs> {
        let url = format!("{}/refs", self.client.api_url(params.repo_type, &params.repo_id));
        let mut query: Vec<(&str, String)> = Vec::new();
        if params.include_pull_requests {
            query.push(("include_prs", "1".into()));
        }

        let response = self
            .client
            .inner
            .client
            .get(&url)
            .headers(self.client.auth_headers())
            .query(&query)
            .send()
            .await?;

        let response = self
            .client
            .check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(response.json().await?)
    }

    /// Get the diff between two revisions as a unified diff string.
    /// Endpoint: GET /api/{repo_type}s/{repo_id}/compare/{compare}
    /// `compare` is in the format "revA..revB"
    pub async fn list_repo_commit_diff(&self, params: &GetCommitDiffParams) -> Result<String> {
        let url = format!("{}/compare/{}", self.client.api_url(params.repo_type, &params.repo_id), params.compare);

        let response = self
            .client
            .inner
            .client
            .get(&url)
            .headers(self.client.auth_headers())
            .send()
            .await?;

        let response = self
            .client
            .check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(response.text().await?)
    }

    /// Get the raw diff between two revisions.
    /// Endpoint: GET /api/{repo_type}s/{repo_id}/compare/{compare}?raw=true
    pub async fn list_repo_raw_diff(&self, params: &GetRawDiffParams) -> Result<String> {
        let url = format!("{}/compare/{}", self.client.api_url(params.repo_type, &params.repo_id), params.compare);

        let response = self
            .client
            .inner
            .client
            .get(&url)
            .headers(self.client.auth_headers())
            .query(&[("raw", "true")])
            .send()
            .await?;

        let response = self
            .client
            .check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(response.text().await?)
    }

    /// Create a new branch.
    /// Endpoint: POST /api/{repo_type}s/{repo_id}/branch/{branch}
    pub async fn create_repo_branch(&self, params: &CreateBranchParams) -> Result<()> {
        let url = format!("{}/branch/{}", self.client.api_url(params.repo_type, &params.repo_id), params.branch);

        let mut body = serde_json::Map::new();
        if let Some(ref revision) = params.revision {
            body.insert("startingPoint".into(), serde_json::Value::String(revision.clone()));
        }

        let response = self
            .client
            .inner
            .client
            .post(&url)
            .headers(self.client.auth_headers())
            .json(&body)
            .send()
            .await?;

        self.client
            .check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(())
    }

    /// Delete a branch.
    /// Endpoint: DELETE /api/{repo_type}s/{repo_id}/branch/{branch}
    pub async fn delete_repo_branch(&self, params: &DeleteBranchParams) -> Result<()> {
        let url = format!("{}/branch/{}", self.client.api_url(params.repo_type, &params.repo_id), params.branch);

        let response = self
            .client
            .inner
            .client
            .delete(&url)
            .headers(self.client.auth_headers())
            .send()
            .await?;

        self.client
            .check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(())
    }

    /// Create a new tag.
    /// Endpoint: POST /api/{repo_type}s/{repo_id}/tag/{revision}
    pub async fn create_repo_tag(&self, params: &CreateTagParams) -> Result<()> {
        let revision = params.revision.as_deref().unwrap_or(constants::DEFAULT_REVISION);
        let url = format!("{}/tag/{}", self.client.api_url(params.repo_type, &params.repo_id), revision);

        let mut body = serde_json::json!({ "tag": params.tag });
        if let Some(ref message) = params.message {
            body["message"] = serde_json::Value::String(message.clone());
        }

        let response = self
            .client
            .inner
            .client
            .post(&url)
            .headers(self.client.auth_headers())
            .json(&body)
            .send()
            .await?;

        self.client
            .check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(())
    }

    /// Delete a tag.
    /// Endpoint: DELETE /api/{repo_type}s/{repo_id}/tag/{tag}
    pub async fn delete_repo_tag(&self, params: &DeleteTagParams) -> Result<()> {
        let url = format!("{}/tag/{}", self.client.api_url(params.repo_type, &params.repo_id), params.tag);

        let response = self
            .client
            .inner
            .client
            .delete(&url)
            .headers(self.client.auth_headers())
            .send()
            .await?;

        self.client
            .check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(())
    }
}
