use crate::error::Result;
use crate::repository::{
    RepoChangeDiscussionStatusParams, RepoCommentDiscussionParams, RepoCreateDiscussionParams,
    RepoCreatePullRequestParams, RepoDiscussionDetailsParams, RepoEditDiscussionCommentParams,
    RepoHideDiscussionCommentParams, RepoListDiscussionsParams, RepoMergePullRequestParams, RepoRenameDiscussionParams,
};
use crate::types::{DiscussionComment, DiscussionWithDetails, DiscussionsResponse};

impl crate::repository::HFRepository {
    /// List discussions for this repository, with optional filters on author, type, and status.
    pub async fn list_discussions(&self, params: &RepoListDiscussionsParams) -> Result<DiscussionsResponse> {
        let url = format!("{}/discussions", self.client.api_url(Some(self.repo_type), &self.repo_path()));
        let mut query: Vec<(String, String)> = Vec::new();
        if let Some(ref author) = params.author {
            query.push(("author".into(), author.clone()));
        }
        if let Some(ref discussion_type) = params.discussion_type {
            query.push(("type".into(), discussion_type.clone()));
        }
        if let Some(ref discussion_status) = params.discussion_status {
            query.push(("status".into(), discussion_status.clone()));
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
        let repo_path = self.repo_path();
        let response = self
            .client
            .check_response(response, Some(&repo_path), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(response.json().await?)
    }

    /// Fetch the full details and event timeline for a single discussion or pull request.
    pub async fn discussion_details(&self, params: &RepoDiscussionDetailsParams) -> Result<DiscussionWithDetails> {
        let url = format!(
            "{}/discussions/{}",
            self.client.api_url(Some(self.repo_type), &self.repo_path()),
            params.discussion_num
        );
        let response = self
            .client
            .inner
            .client
            .get(&url)
            .headers(self.client.auth_headers())
            .send()
            .await?;
        let repo_path = self.repo_path();
        let response = self
            .client
            .check_response(response, Some(&repo_path), crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn create_discussion(&self, params: &RepoCreateDiscussionParams) -> Result<DiscussionWithDetails> {
        let url = format!("{}/discussions", self.client.api_url(Some(self.repo_type), &self.repo_path()));
        let mut body = serde_json::json!({ "title": params.title });
        if let Some(ref desc) = params.description {
            body["description"] = serde_json::json!(desc);
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
        let repo_path = self.repo_path();
        let response = self
            .client
            .check_response(response, Some(&repo_path), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn create_pull_request(&self, params: &RepoCreatePullRequestParams) -> Result<DiscussionWithDetails> {
        let url = format!("{}/discussions", self.client.api_url(Some(self.repo_type), &self.repo_path()));
        let mut body = serde_json::json!({
            "title": params.title,
            "pullRequest": true,
        });
        if let Some(ref desc) = params.description {
            body["description"] = serde_json::json!(desc);
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
        let repo_path = self.repo_path();
        let response = self
            .client
            .check_response(response, Some(&repo_path), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn comment_discussion(&self, params: &RepoCommentDiscussionParams) -> Result<DiscussionComment> {
        let url = format!(
            "{}/discussions/{}/comment",
            self.client.api_url(Some(self.repo_type), &self.repo_path()),
            params.discussion_num
        );
        let body = serde_json::json!({ "comment": params.comment });
        let response = self
            .client
            .inner
            .client
            .post(&url)
            .headers(self.client.auth_headers())
            .json(&body)
            .send()
            .await?;
        let repo_path = self.repo_path();
        let response = self
            .client
            .check_response(response, Some(&repo_path), crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn edit_discussion_comment(&self, params: &RepoEditDiscussionCommentParams) -> Result<DiscussionComment> {
        let url = format!(
            "{}/discussions/{}/comment/{}/edit",
            self.client.api_url(Some(self.repo_type), &self.repo_path()),
            params.discussion_num,
            params.comment_id
        );
        let body = serde_json::json!({ "content": params.new_content });
        let response = self
            .client
            .inner
            .client
            .post(&url)
            .headers(self.client.auth_headers())
            .json(&body)
            .send()
            .await?;
        let repo_path = self.repo_path();
        let response = self
            .client
            .check_response(response, Some(&repo_path), crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn hide_discussion_comment(&self, params: &RepoHideDiscussionCommentParams) -> Result<DiscussionComment> {
        let url = format!(
            "{}/discussions/{}/comment/{}/hide",
            self.client.api_url(Some(self.repo_type), &self.repo_path()),
            params.discussion_num,
            params.comment_id
        );
        let response = self
            .client
            .inner
            .client
            .put(&url)
            .headers(self.client.auth_headers())
            .send()
            .await?;
        let repo_path = self.repo_path();
        let response = self
            .client
            .check_response(response, Some(&repo_path), crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn rename_discussion(&self, params: &RepoRenameDiscussionParams) -> Result<DiscussionWithDetails> {
        let url = format!(
            "{}/discussions/{}/title",
            self.client.api_url(Some(self.repo_type), &self.repo_path()),
            params.discussion_num
        );
        let body = serde_json::json!({ "title": params.new_title });
        let response = self
            .client
            .inner
            .client
            .post(&url)
            .headers(self.client.auth_headers())
            .json(&body)
            .send()
            .await?;
        let repo_path = self.repo_path();
        let response = self
            .client
            .check_response(response, Some(&repo_path), crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn change_discussion_status(
        &self,
        params: &RepoChangeDiscussionStatusParams,
    ) -> Result<DiscussionWithDetails> {
        let url = format!(
            "{}/discussions/{}/status",
            self.client.api_url(Some(self.repo_type), &self.repo_path()),
            params.discussion_num
        );
        let body = serde_json::json!({ "status": params.new_status });
        let response = self
            .client
            .inner
            .client
            .post(&url)
            .headers(self.client.auth_headers())
            .json(&body)
            .send()
            .await?;
        let repo_path = self.repo_path();
        let response = self
            .client
            .check_response(response, Some(&repo_path), crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn merge_pull_request(&self, params: &RepoMergePullRequestParams) -> Result<DiscussionWithDetails> {
        let url = format!(
            "{}/discussions/{}/merge",
            self.client.api_url(Some(self.repo_type), &self.repo_path()),
            params.discussion_num
        );
        let response = self
            .client
            .inner
            .client
            .post(&url)
            .headers(self.client.auth_headers())
            .send()
            .await?;
        let repo_path = self.repo_path();
        let response = self
            .client
            .check_response(response, Some(&repo_path), crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }
}
