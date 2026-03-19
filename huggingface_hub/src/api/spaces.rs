use crate::client::HfApi;
use crate::error::Result;
use crate::types::{
    AddSpaceSecretParams, AddSpaceVariableParams, DeleteSpaceSecretParams,
    DeleteSpaceVariableParams, DuplicateSpaceParams, GetSpaceRuntimeParams, PauseSpaceParams,
    RepoUrl, RequestSpaceHardwareParams, RestartSpaceParams, SetSpaceSleepTimeParams, SpaceRuntime,
};

impl HfApi {
    pub async fn get_space_runtime(&self, params: &GetSpaceRuntimeParams) -> Result<SpaceRuntime> {
        let url = format!(
            "{}/api/spaces/{}/runtime",
            self.inner.endpoint, params.repo_id
        );
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

    pub async fn request_space_hardware(
        &self,
        params: &RequestSpaceHardwareParams,
    ) -> Result<SpaceRuntime> {
        let url = format!(
            "{}/api/spaces/{}/hardware",
            self.inner.endpoint, params.repo_id
        );
        let mut body = serde_json::json!({ "flavor": params.hardware });
        if let Some(sleep_time) = params.sleep_time {
            body["sleepTime"] = serde_json::json!(sleep_time);
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
            .check_response(
                response,
                Some(&params.repo_id),
                crate::error::NotFoundContext::Repo,
            )
            .await?;
        Ok(response.json().await?)
    }

    pub async fn set_space_sleep_time(&self, params: &SetSpaceSleepTimeParams) -> Result<()> {
        let url = format!(
            "{}/api/spaces/{}/sleeptime",
            self.inner.endpoint, params.repo_id
        );
        let body = serde_json::json!({ "seconds": params.sleep_time });
        let response = self
            .inner
            .client
            .post(&url)
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

    pub async fn pause_space(&self, params: &PauseSpaceParams) -> Result<SpaceRuntime> {
        let url = format!(
            "{}/api/spaces/{}/pause",
            self.inner.endpoint, params.repo_id
        );
        let response = self
            .inner
            .client
            .post(&url)
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

    pub async fn restart_space(&self, params: &RestartSpaceParams) -> Result<SpaceRuntime> {
        let url = format!(
            "{}/api/spaces/{}/restart",
            self.inner.endpoint, params.repo_id
        );
        let response = self
            .inner
            .client
            .post(&url)
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

    pub async fn add_space_secret(&self, params: &AddSpaceSecretParams) -> Result<()> {
        let url = format!(
            "{}/api/spaces/{}/secrets",
            self.inner.endpoint, params.repo_id
        );
        let mut body = serde_json::json!({
            "key": params.key,
            "value": params.value,
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
        self.check_response(
            response,
            Some(&params.repo_id),
            crate::error::NotFoundContext::Repo,
        )
        .await?;
        Ok(())
    }

    pub async fn delete_space_secret(&self, params: &DeleteSpaceSecretParams) -> Result<()> {
        let url = format!(
            "{}/api/spaces/{}/secrets",
            self.inner.endpoint, params.repo_id
        );
        let body = serde_json::json!({ "key": params.key });
        let response = self
            .inner
            .client
            .delete(&url)
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

    pub async fn add_space_variable(&self, params: &AddSpaceVariableParams) -> Result<()> {
        let url = format!(
            "{}/api/spaces/{}/variables",
            self.inner.endpoint, params.repo_id
        );
        let mut body = serde_json::json!({
            "key": params.key,
            "value": params.value,
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
        self.check_response(
            response,
            Some(&params.repo_id),
            crate::error::NotFoundContext::Repo,
        )
        .await?;
        Ok(())
    }

    pub async fn delete_space_variable(&self, params: &DeleteSpaceVariableParams) -> Result<()> {
        let url = format!(
            "{}/api/spaces/{}/variables",
            self.inner.endpoint, params.repo_id
        );
        let body = serde_json::json!({ "key": params.key });
        let response = self
            .inner
            .client
            .delete(&url)
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

    pub async fn duplicate_space(&self, params: &DuplicateSpaceParams) -> Result<RepoUrl> {
        let url = format!(
            "{}/api/spaces/{}/duplicate",
            self.inner.endpoint, params.from_id
        );
        let mut body = serde_json::Map::new();
        if let Some(ref to_id) = params.to_id {
            body.insert("repository".into(), serde_json::json!(to_id));
        }
        if let Some(private) = params.private {
            body.insert("private".into(), serde_json::json!(private));
        }
        if let Some(ref hw) = params.hardware {
            body.insert("hardware".into(), serde_json::json!(hw));
        }
        if let Some(ref storage) = params.storage {
            body.insert("storage".into(), serde_json::json!(storage));
        }
        if let Some(sleep_time) = params.sleep_time {
            body.insert("sleepTime".into(), serde_json::json!(sleep_time));
        }
        if let Some(ref secrets) = params.secrets {
            body.insert("secrets".into(), serde_json::json!(secrets));
        }
        if let Some(ref variables) = params.variables {
            body.insert("variables".into(), serde_json::json!(variables));
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
            .check_response(
                response,
                Some(&params.from_id),
                crate::error::NotFoundContext::Repo,
            )
            .await?;
        Ok(response.json().await?)
    }
}

sync_api! {
    impl HfApi {
        fn get_space_runtime(&self, params: &GetSpaceRuntimeParams) -> Result<SpaceRuntime>;
        fn request_space_hardware(&self, params: &RequestSpaceHardwareParams) -> Result<SpaceRuntime>;
        fn set_space_sleep_time(&self, params: &SetSpaceSleepTimeParams) -> Result<()>;
        fn pause_space(&self, params: &PauseSpaceParams) -> Result<SpaceRuntime>;
        fn restart_space(&self, params: &RestartSpaceParams) -> Result<SpaceRuntime>;
        fn add_space_secret(&self, params: &AddSpaceSecretParams) -> Result<()>;
        fn delete_space_secret(&self, params: &DeleteSpaceSecretParams) -> Result<()>;
        fn add_space_variable(&self, params: &AddSpaceVariableParams) -> Result<()>;
        fn delete_space_variable(&self, params: &DeleteSpaceVariableParams) -> Result<()>;
        fn duplicate_space(&self, params: &DuplicateSpaceParams) -> Result<RepoUrl>;
    }
}
