use crate::error::Result;
use crate::types::{AccessRequest, GrantAccessParams, HandleAccessRequestParams, ListAccessRequestsParams};

impl crate::repository::HFRepository {
    async fn list_access_requests_by_status(
        &self,
        params: &ListAccessRequestsParams,
        status: &str,
    ) -> Result<Vec<AccessRequest>> {
        let url = format!("{}/user-access-request/{}", self.client.api_url(params.repo_type, &params.repo_id), status);
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
        Ok(response.json().await?)
    }

    pub async fn list_pending_access_requests(&self, params: &ListAccessRequestsParams) -> Result<Vec<AccessRequest>> {
        self.list_access_requests_by_status(params, "pending").await
    }

    pub async fn list_accepted_access_requests(&self, params: &ListAccessRequestsParams) -> Result<Vec<AccessRequest>> {
        self.list_access_requests_by_status(params, "accepted").await
    }

    pub async fn list_rejected_access_requests(&self, params: &ListAccessRequestsParams) -> Result<Vec<AccessRequest>> {
        self.list_access_requests_by_status(params, "rejected").await
    }

    async fn handle_access_request(&self, params: &HandleAccessRequestParams, status: &str) -> Result<()> {
        let url = format!("{}/user-access-request/handle", self.client.api_url(params.repo_type, &params.repo_id));
        let body = serde_json::json!({
            "user": params.user,
            "status": status,
        });
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

    pub async fn accept_access_request(&self, params: &HandleAccessRequestParams) -> Result<()> {
        self.handle_access_request(params, "accepted").await
    }

    pub async fn reject_access_request(&self, params: &HandleAccessRequestParams) -> Result<()> {
        self.handle_access_request(params, "rejected").await
    }

    pub async fn cancel_access_request(&self, params: &HandleAccessRequestParams) -> Result<()> {
        self.handle_access_request(params, "cancelled").await
    }

    pub async fn grant_access(&self, params: &GrantAccessParams) -> Result<()> {
        let url = format!("{}/user-access-request/grant", self.client.api_url(params.repo_type, &params.repo_id));
        let body = serde_json::json!({ "user": params.user });
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
}
