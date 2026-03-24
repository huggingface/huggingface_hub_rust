use crate::client::HfApi;
use crate::error::Result;
use crate::types::{
    ChangeDiscussionStatusParams, CommentDiscussionParams, CreateDiscussionParams, CreatePullRequestParams,
    DiscussionComment, DiscussionWithDetails, DiscussionsResponse, EditDiscussionCommentParams,
    GetDiscussionDetailsParams, GetRepoDiscussionsParams, HideDiscussionCommentParams, MergePullRequestParams,
    RenameDiscussionParams,
};

impl HfApi {
    pub async fn get_repo_discussions(&self, params: &GetRepoDiscussionsParams) -> Result<DiscussionsResponse> {
        let url = format!("{}/discussions", self.api_url(params.repo_type, &params.repo_id));
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
            .inner
            .client
            .get(&url)
            .headers(self.auth_headers())
            .query(&query)
            .send()
            .await?;
        let response = self
            .check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn get_discussion_details(&self, params: &GetDiscussionDetailsParams) -> Result<DiscussionWithDetails> {
        let url = format!("{}/discussions/{}", self.api_url(params.repo_type, &params.repo_id), params.discussion_num);
        let response = self.inner.client.get(&url).headers(self.auth_headers()).send().await?;
        let response = self
            .check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn create_discussion(&self, params: &CreateDiscussionParams) -> Result<DiscussionWithDetails> {
        let url = format!("{}/discussions", self.api_url(params.repo_type, &params.repo_id));
        let mut body = serde_json::json!({ "title": params.title });
        if let Some(ref desc) = params.description {
            body["description"] = serde_json::json!(desc);
        }
        let response = self
            .inner
            .client
            .post(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;
        let response = self
            .check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn create_pull_request(&self, params: &CreatePullRequestParams) -> Result<DiscussionWithDetails> {
        let url = format!("{}/discussions", self.api_url(params.repo_type, &params.repo_id));
        let mut body = serde_json::json!({
            "title": params.title,
            "pullRequest": true,
        });
        if let Some(ref desc) = params.description {
            body["description"] = serde_json::json!(desc);
        }
        let response = self
            .inner
            .client
            .post(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;
        let response = self
            .check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn comment_discussion(&self, params: &CommentDiscussionParams) -> Result<DiscussionComment> {
        let url = format!(
            "{}/discussions/{}/comment",
            self.api_url(params.repo_type, &params.repo_id),
            params.discussion_num
        );
        let body = serde_json::json!({ "comment": params.comment });
        let response = self
            .inner
            .client
            .post(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;
        let response = self
            .check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn edit_discussion_comment(&self, params: &EditDiscussionCommentParams) -> Result<DiscussionComment> {
        let url = format!(
            "{}/discussions/{}/comment/{}/edit",
            self.api_url(params.repo_type, &params.repo_id),
            params.discussion_num,
            params.comment_id
        );
        let body = serde_json::json!({ "content": params.new_content });
        let response = self
            .inner
            .client
            .post(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;
        let response = self
            .check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn hide_discussion_comment(&self, params: &HideDiscussionCommentParams) -> Result<DiscussionComment> {
        let url = format!(
            "{}/discussions/{}/comment/{}/hide",
            self.api_url(params.repo_type, &params.repo_id),
            params.discussion_num,
            params.comment_id
        );
        let response = self.inner.client.put(&url).headers(self.auth_headers()).send().await?;
        let response = self
            .check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn rename_discussion(&self, params: &RenameDiscussionParams) -> Result<DiscussionWithDetails> {
        let url =
            format!("{}/discussions/{}/title", self.api_url(params.repo_type, &params.repo_id), params.discussion_num);
        let body = serde_json::json!({ "title": params.new_title });
        let response = self
            .inner
            .client
            .post(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;
        let response = self
            .check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn change_discussion_status(
        &self,
        params: &ChangeDiscussionStatusParams,
    ) -> Result<DiscussionWithDetails> {
        let url =
            format!("{}/discussions/{}/status", self.api_url(params.repo_type, &params.repo_id), params.discussion_num);
        let body = serde_json::json!({ "status": params.new_status });
        let response = self
            .inner
            .client
            .post(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;
        let response = self
            .check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn merge_pull_request(&self, params: &MergePullRequestParams) -> Result<DiscussionWithDetails> {
        let url =
            format!("{}/discussions/{}/merge", self.api_url(params.repo_type, &params.repo_id), params.discussion_num);
        let response = self.inner.client.post(&url).headers(self.auth_headers()).send().await?;
        let response = self
            .check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }
}

sync_api! {
    impl HfApiSync {
        fn get_repo_discussions(&self, params: &GetRepoDiscussionsParams) -> Result<DiscussionsResponse>;
        fn get_discussion_details(&self, params: &GetDiscussionDetailsParams) -> Result<DiscussionWithDetails>;
        fn create_discussion(&self, params: &CreateDiscussionParams) -> Result<DiscussionWithDetails>;
        fn create_pull_request(&self, params: &CreatePullRequestParams) -> Result<DiscussionWithDetails>;
        fn comment_discussion(&self, params: &CommentDiscussionParams) -> Result<DiscussionComment>;
        fn edit_discussion_comment(&self, params: &EditDiscussionCommentParams) -> Result<DiscussionComment>;
        fn hide_discussion_comment(&self, params: &HideDiscussionCommentParams) -> Result<DiscussionComment>;
        fn rename_discussion(&self, params: &RenameDiscussionParams) -> Result<DiscussionWithDetails>;
        fn change_discussion_status(&self, params: &ChangeDiscussionStatusParams) -> Result<DiscussionWithDetails>;
        fn merge_pull_request(&self, params: &MergePullRequestParams) -> Result<DiscussionWithDetails>;
    }
}
