use crate::client::HfApi;
use crate::error::Result;
use crate::types::{
    CreateScheduledJobParams, JobHardware, JobInfo, JobLogEntry, JobMetrics, ListJobsParams,
    RunJobParams, ScheduledJobInfo,
};

impl HfApi {
    async fn resolve_jobs_namespace(&self, namespace: &Option<String>) -> Result<String> {
        match namespace {
            Some(ns) => Ok(ns.clone()),
            None => {
                let user = self.whoami().await?;
                Ok(user.username)
            }
        }
    }

    pub async fn run_job(&self, params: &RunJobParams) -> Result<JobInfo> {
        let ns = self.resolve_jobs_namespace(&params.namespace).await?;
        let url = format!("{}/api/jobs/{}", self.inner.endpoint, ns);
        let mut body = serde_json::json!({
            "dockerImage": params.image,
            "command": params.command,
        });
        if let Some(ref flavor) = params.flavor {
            body["flavor"] = serde_json::json!(flavor);
        }
        if let Some(ref env) = params.env {
            body["environment"] = serde_json::json!(env);
        }
        if let Some(ref secrets) = params.secrets {
            body["secrets"] = serde_json::json!(secrets);
        }
        if let Some(ref timeout) = params.timeout {
            body["timeout"] = serde_json::json!(timeout);
        }
        if let Some(ref labels) = params.labels {
            body["labels"] = serde_json::json!(labels);
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

    pub async fn list_jobs(&self, params: &ListJobsParams) -> Result<Vec<JobInfo>> {
        let ns = self.resolve_jobs_namespace(&params.namespace).await?;
        let url = format!("{}/api/jobs/{}", self.inner.endpoint, ns);
        let query: Vec<(String, String)> = Vec::new();
        let response = self
            .inner
            .client
            .get(&url)
            .headers(self.auth_headers())
            .query(&query)
            .send()
            .await?;
        let response = self
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn inspect_job(&self, job_id: &str, namespace: Option<&str>) -> Result<JobInfo> {
        let ns = self
            .resolve_jobs_namespace(&namespace.map(String::from))
            .await?;
        let url = format!("{}/api/jobs/{}/{}", self.inner.endpoint, ns, job_id);
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

    pub async fn cancel_job(&self, job_id: &str, namespace: Option<&str>) -> Result<JobInfo> {
        let ns = self
            .resolve_jobs_namespace(&namespace.map(String::from))
            .await?;
        let url = format!("{}/api/jobs/{}/{}/cancel", self.inner.endpoint, ns, job_id);
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

    pub async fn fetch_job_logs(
        &self,
        job_id: &str,
        namespace: Option<&str>,
    ) -> Result<Vec<JobLogEntry>> {
        let ns = self
            .resolve_jobs_namespace(&namespace.map(String::from))
            .await?;
        let url = format!("{}/api/jobs/{}/{}/logs", self.inner.endpoint, ns, job_id);
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

    pub async fn fetch_job_metrics(
        &self,
        job_id: &str,
        namespace: Option<&str>,
    ) -> Result<Vec<JobMetrics>> {
        let ns = self
            .resolve_jobs_namespace(&namespace.map(String::from))
            .await?;
        let url = format!("{}/api/jobs/{}/{}/metrics", self.inner.endpoint, ns, job_id);
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

    pub async fn list_job_hardware(&self) -> Result<Vec<JobHardware>> {
        let url = format!("{}/api/jobs/hardware", self.inner.endpoint);
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

    pub async fn create_scheduled_job(
        &self,
        params: &CreateScheduledJobParams,
    ) -> Result<ScheduledJobInfo> {
        let url = format!("{}/api/jobs/scheduled", self.inner.endpoint);
        let mut body = serde_json::json!({
            "dockerImage": params.image,
            "command": params.command,
            "schedule": params.schedule,
        });
        if let Some(ref flavor) = params.flavor {
            body["flavor"] = serde_json::json!(flavor);
        }
        if let Some(ref env) = params.env {
            body["environment"] = serde_json::json!(env);
        }
        if let Some(ref secrets) = params.secrets {
            body["secrets"] = serde_json::json!(secrets);
        }
        if let Some(ref timeout) = params.timeout {
            body["timeout"] = serde_json::json!(timeout);
        }
        if let Some(ref ns) = params.namespace {
            body["namespace"] = serde_json::json!(ns);
        }
        if let Some(suspend) = params.suspend {
            body["suspend"] = serde_json::json!(suspend);
        }
        if let Some(concurrency) = params.concurrency {
            body["concurrency"] = serde_json::json!(concurrency);
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

    pub async fn list_scheduled_jobs(&self) -> Result<Vec<ScheduledJobInfo>> {
        let url = format!("{}/api/jobs/scheduled", self.inner.endpoint);
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

    pub async fn inspect_scheduled_job(&self, scheduled_job_id: &str) -> Result<ScheduledJobInfo> {
        let url = format!(
            "{}/api/jobs/scheduled/{}",
            self.inner.endpoint, scheduled_job_id
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

    pub async fn delete_scheduled_job(&self, scheduled_job_id: &str) -> Result<()> {
        let url = format!(
            "{}/api/jobs/scheduled/{}",
            self.inner.endpoint, scheduled_job_id
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

    pub async fn suspend_scheduled_job(&self, scheduled_job_id: &str) -> Result<ScheduledJobInfo> {
        let url = format!(
            "{}/api/jobs/scheduled/{}/suspend",
            self.inner.endpoint, scheduled_job_id
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

    pub async fn resume_scheduled_job(&self, scheduled_job_id: &str) -> Result<ScheduledJobInfo> {
        let url = format!(
            "{}/api/jobs/scheduled/{}/resume",
            self.inner.endpoint, scheduled_job_id
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
