use crate::error::Result;
use crate::repository::HFSpace;
use crate::types::{RepoUrl, SpaceRuntime};

impl HFSpace {
    /// Fetch the current runtime state of the Space (hardware, stage, URL, etc.).
    pub async fn runtime(&self) -> Result<SpaceRuntime> {
        let url = format!("{}/api/spaces/{}/runtime", self.client.inner.endpoint, self.repo_path());
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
            .check_response(response, Some(&self.repo_path()), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(response.json().await?)
    }

    /// Request an upgrade or downgrade of the Space's hardware tier.
    pub async fn request_hardware(
        &self,
        params: &crate::repository::SpaceHardwareRequestParams,
    ) -> Result<SpaceRuntime> {
        let url = format!("{}/api/spaces/{}/hardware", self.client.inner.endpoint, self.repo_path());
        let mut body = serde_json::json!({ "flavor": params.hardware });
        if let Some(sleep_time) = params.sleep_time {
            body["sleepTime"] = serde_json::json!(sleep_time);
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
        let response = self
            .client
            .check_response(response, Some(&self.repo_path()), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(response.json().await?)
    }

    /// Configure the number of seconds of inactivity before the Space is put to sleep.
    pub async fn set_sleep_time(&self, params: &crate::repository::SpaceSleepTimeParams) -> Result<()> {
        let url = format!("{}/api/spaces/{}/sleeptime", self.client.inner.endpoint, self.repo_path());
        let body = serde_json::json!({ "seconds": params.sleep_time });
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
            .check_response(response, Some(&self.repo_path()), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(())
    }

    /// Pause the Space, stopping it from consuming compute resources.
    pub async fn pause(&self) -> Result<SpaceRuntime> {
        let url = format!("{}/api/spaces/{}/pause", self.client.inner.endpoint, self.repo_path());
        let response = self
            .client
            .inner
            .client
            .post(&url)
            .headers(self.client.auth_headers())
            .send()
            .await?;
        let response = self
            .client
            .check_response(response, Some(&self.repo_path()), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(response.json().await?)
    }

    /// Restart a paused or errored Space.
    pub async fn restart(&self) -> Result<SpaceRuntime> {
        let url = format!("{}/api/spaces/{}/restart", self.client.inner.endpoint, self.repo_path());
        let response = self
            .client
            .inner
            .client
            .post(&url)
            .headers(self.client.auth_headers())
            .send()
            .await?;
        let response = self
            .client
            .check_response(response, Some(&self.repo_path()), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(response.json().await?)
    }

    /// Add or update a secret (encrypted environment variable) on the Space.
    pub async fn add_secret(&self, params: &crate::repository::SpaceSecretParams) -> Result<()> {
        let url = format!("{}/api/spaces/{}/secrets", self.client.inner.endpoint, self.repo_path());
        let mut body = serde_json::json!({
            "key": params.key,
            "value": params.value,
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
        self.client
            .check_response(response, Some(&self.repo_path()), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(())
    }

    /// Delete a secret from the Space by key.
    pub async fn delete_secret(&self, params: &crate::repository::SpaceSecretDeleteParams) -> Result<()> {
        let url = format!("{}/api/spaces/{}/secrets", self.client.inner.endpoint, self.repo_path());
        let body = serde_json::json!({ "key": params.key });
        let response = self
            .client
            .inner
            .client
            .delete(&url)
            .headers(self.client.auth_headers())
            .json(&body)
            .send()
            .await?;
        self.client
            .check_response(response, Some(&self.repo_path()), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(())
    }

    /// Add or update a public environment variable on the Space.
    pub async fn add_variable(&self, params: &crate::repository::SpaceVariableParams) -> Result<()> {
        let url = format!("{}/api/spaces/{}/variables", self.client.inner.endpoint, self.repo_path());
        let mut body = serde_json::json!({
            "key": params.key,
            "value": params.value,
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
        self.client
            .check_response(response, Some(&self.repo_path()), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(())
    }

    /// Delete a public environment variable from the Space by key.
    pub async fn delete_variable(&self, params: &crate::repository::SpaceVariableDeleteParams) -> Result<()> {
        let url = format!("{}/api/spaces/{}/variables", self.client.inner.endpoint, self.repo_path());
        let body = serde_json::json!({ "key": params.key });
        let response = self
            .client
            .inner
            .client
            .delete(&url)
            .headers(self.client.auth_headers())
            .json(&body)
            .send()
            .await?;
        self.client
            .check_response(response, Some(&self.repo_path()), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(())
    }

    /// Duplicate this Space to a new repository.
    pub async fn duplicate(&self, params: &crate::types::DuplicateSpaceParams) -> Result<RepoUrl> {
        let url = format!("{}/api/spaces/{}/duplicate", self.client.inner.endpoint, self.repo_path());
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
            .client
            .inner
            .client
            .post(&url)
            .headers(self.client.auth_headers())
            .json(&body)
            .send()
            .await?;
        let response = self
            .client
            .check_response(response, Some(&self.repo_path()), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(response.json().await?)
    }
}

sync_api! {
    #[cfg(feature = "spaces")]
    impl HFSpaceSync => HFSpace {
        fn runtime(&self) -> crate::error::Result<SpaceRuntime>;
        fn request_hardware(&self, params: &crate::repository::SpaceHardwareRequestParams) -> crate::error::Result<SpaceRuntime>;
        fn set_sleep_time(&self, params: &crate::repository::SpaceSleepTimeParams) -> crate::error::Result<()>;
        fn pause(&self) -> crate::error::Result<SpaceRuntime>;
        fn restart(&self) -> crate::error::Result<SpaceRuntime>;
        fn add_secret(&self, params: &crate::repository::SpaceSecretParams) -> crate::error::Result<()>;
        fn delete_secret(&self, params: &crate::repository::SpaceSecretDeleteParams) -> crate::error::Result<()>;
        fn add_variable(&self, params: &crate::repository::SpaceVariableParams) -> crate::error::Result<()>;
        fn delete_variable(&self, params: &crate::repository::SpaceVariableDeleteParams) -> crate::error::Result<()>;
        fn duplicate(&self, params: &crate::types::DuplicateSpaceParams) -> crate::error::Result<RepoUrl>;
    }
}
