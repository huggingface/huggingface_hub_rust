use crate::client::HfApi;
use crate::error::Result;
use crate::types::{CreateWebhookParams, UpdateWebhookParams, WebhookInfo};

impl HfApi {
    pub async fn list_webhooks(&self) -> Result<Vec<WebhookInfo>> {
        let url = format!("{}/api/settings/webhooks", self.inner.endpoint);
        let response = self
            .inner
            .client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await?;
        let response = self
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn get_webhook(&self, webhook_id: &str) -> Result<WebhookInfo> {
        let url = format!(
            "{}/api/settings/webhooks/{}",
            self.inner.endpoint, webhook_id
        );
        let response = self
            .inner
            .client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await?;
        let response = self
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn create_webhook(&self, params: &CreateWebhookParams) -> Result<WebhookInfo> {
        let url = format!("{}/api/settings/webhooks", self.inner.endpoint);
        let mut body = serde_json::json!({
            "url": params.url,
            "watched": params.watched,
        });
        if let Some(ref domains) = params.domains {
            body["domains"] = serde_json::json!(domains);
        }
        if let Some(ref secret) = params.secret {
            body["secret"] = serde_json::json!(secret);
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
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn update_webhook(&self, params: &UpdateWebhookParams) -> Result<WebhookInfo> {
        let url = format!(
            "{}/api/settings/webhooks/{}",
            self.inner.endpoint, params.webhook_id
        );
        let mut body = serde_json::Map::new();
        if let Some(ref wh_url) = params.url {
            body.insert("url".into(), serde_json::json!(wh_url));
        }
        if let Some(ref watched) = params.watched {
            body.insert("watched".into(), serde_json::json!(watched));
        }
        if let Some(ref domains) = params.domains {
            body.insert("domains".into(), serde_json::json!(domains));
        }
        if let Some(ref secret) = params.secret {
            body.insert("secret".into(), serde_json::json!(secret));
        }
        let response = self
            .inner
            .client
            .put(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;
        let response = self
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn delete_webhook(&self, webhook_id: &str) -> Result<()> {
        let url = format!(
            "{}/api/settings/webhooks/{}",
            self.inner.endpoint, webhook_id
        );
        let response = self
            .inner
            .client
            .delete(&url)
            .headers(self.auth_headers())
            .send()
            .await?;
        self.check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(())
    }

    pub async fn enable_webhook(&self, webhook_id: &str) -> Result<WebhookInfo> {
        let url = format!(
            "{}/api/settings/webhooks/{}/enable",
            self.inner.endpoint, webhook_id
        );
        let response = self
            .inner
            .client
            .post(&url)
            .headers(self.auth_headers())
            .send()
            .await?;
        let response = self
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn disable_webhook(&self, webhook_id: &str) -> Result<WebhookInfo> {
        let url = format!(
            "{}/api/settings/webhooks/{}/disable",
            self.inner.endpoint, webhook_id
        );
        let response = self
            .inner
            .client
            .post(&url)
            .headers(self.auth_headers())
            .send()
            .await?;
        let response = self
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }
}

sync_api! {
    impl HfApi {
        fn list_webhooks(&self) -> Result<Vec<WebhookInfo>>;
        fn get_webhook(&self, webhook_id: &str) -> Result<WebhookInfo>;
        fn create_webhook(&self, params: &CreateWebhookParams) -> Result<WebhookInfo>;
        fn update_webhook(&self, params: &UpdateWebhookParams) -> Result<WebhookInfo>;
        fn delete_webhook(&self, webhook_id: &str) -> Result<()>;
        fn enable_webhook(&self, webhook_id: &str) -> Result<WebhookInfo>;
        fn disable_webhook(&self, webhook_id: &str) -> Result<WebhookInfo>;
    }
}
