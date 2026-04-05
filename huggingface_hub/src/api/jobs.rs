use crate::client::HFClient;
use crate::error::Result;
use crate::types::{
    CreateScheduledJobParams, JobHardware, JobInfo, JobLogEntry, JobMetrics, ListJobsParams, RunJobParams,
    ScheduledJobInfo,
};

impl HFClient {
    async fn resolve_jobs_namespace(&self, namespace: &Option<String>) -> Result<String> {
        match namespace {
            Some(ns) => Ok(ns.clone()),
            None => {
                let user = self.whoami().await?;
                Ok(user.username)
            },
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
        let ns = self.resolve_jobs_namespace(&namespace.map(String::from)).await?;
        let url = format!("{}/api/jobs/{}/{}", self.inner.endpoint, ns, job_id);
        let response = self.inner.client.get(&url).headers(self.auth_headers()).send().await?;
        let response = self
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn cancel_job(&self, job_id: &str, namespace: Option<&str>) -> Result<JobInfo> {
        let ns = self.resolve_jobs_namespace(&namespace.map(String::from)).await?;
        let url = format!("{}/api/jobs/{}/{}/cancel", self.inner.endpoint, ns, job_id);
        let response = self.inner.client.post(&url).headers(self.auth_headers()).send().await?;
        let response = self
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn fetch_job_logs(&self, job_id: &str, namespace: Option<&str>) -> Result<Vec<JobLogEntry>> {
        let ns = self.resolve_jobs_namespace(&namespace.map(String::from)).await?;
        let url = format!("{}/api/jobs/{}/{}/logs", self.inner.endpoint, ns, job_id);
        let response = self.inner.client.get(&url).headers(self.auth_headers()).send().await?;
        let response = self
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        let body = response.text().await?;
        Ok(parse_job_log_sse(&body))
    }

    pub async fn fetch_job_metrics(&self, job_id: &str, namespace: Option<&str>) -> Result<Vec<JobMetrics>> {
        let ns = self.resolve_jobs_namespace(&namespace.map(String::from)).await?;
        let url = format!("{}/api/jobs/{}/{}/metrics", self.inner.endpoint, ns, job_id);
        let response = self.inner.client.get(&url).headers(self.auth_headers()).send().await?;
        let response = self
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn list_job_hardware(&self) -> Result<Vec<JobHardware>> {
        let url = format!("{}/api/jobs/hardware", self.inner.endpoint);
        let response = self.inner.client.get(&url).headers(self.auth_headers()).send().await?;
        let response = self
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn create_scheduled_job(&self, params: &CreateScheduledJobParams) -> Result<ScheduledJobInfo> {
        let ns = self.resolve_jobs_namespace(&params.namespace).await?;
        let url = format!("{}/api/scheduled-jobs/{}", self.inner.endpoint, ns);
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

    pub async fn list_scheduled_jobs(&self, namespace: Option<&str>) -> Result<Vec<ScheduledJobInfo>> {
        let ns = self.resolve_jobs_namespace(&namespace.map(String::from)).await?;
        let url = format!("{}/api/scheduled-jobs/{}", self.inner.endpoint, ns);
        let response = self.inner.client.get(&url).headers(self.auth_headers()).send().await?;
        let response = self
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn inspect_scheduled_job(
        &self,
        scheduled_job_id: &str,
        namespace: Option<&str>,
    ) -> Result<ScheduledJobInfo> {
        let ns = self.resolve_jobs_namespace(&namespace.map(String::from)).await?;
        let url = format!("{}/api/scheduled-jobs/{}/{}", self.inner.endpoint, ns, scheduled_job_id);
        let response = self.inner.client.get(&url).headers(self.auth_headers()).send().await?;
        let response = self
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn delete_scheduled_job(&self, scheduled_job_id: &str, namespace: Option<&str>) -> Result<()> {
        let ns = self.resolve_jobs_namespace(&namespace.map(String::from)).await?;
        let url = format!("{}/api/scheduled-jobs/{}/{}", self.inner.endpoint, ns, scheduled_job_id);
        let response = self.inner.client.delete(&url).headers(self.auth_headers()).send().await?;
        self.check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(())
    }

    pub async fn suspend_scheduled_job(
        &self,
        scheduled_job_id: &str,
        namespace: Option<&str>,
    ) -> Result<ScheduledJobInfo> {
        let ns = self.resolve_jobs_namespace(&namespace.map(String::from)).await?;
        let url = format!("{}/api/scheduled-jobs/{}/{}/suspend", self.inner.endpoint, ns, scheduled_job_id);
        let response = self.inner.client.post(&url).headers(self.auth_headers()).send().await?;
        let response = self
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn resume_scheduled_job(
        &self,
        scheduled_job_id: &str,
        namespace: Option<&str>,
    ) -> Result<ScheduledJobInfo> {
        let ns = self.resolve_jobs_namespace(&namespace.map(String::from)).await?;
        let url = format!("{}/api/scheduled-jobs/{}/{}/resume", self.inner.endpoint, ns, scheduled_job_id);
        let response = self.inner.client.post(&url).headers(self.auth_headers()).send().await?;
        let response = self
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }
}

sync_api! {
    impl HFClientSync {
        fn run_job(&self, params: &RunJobParams) -> Result<JobInfo>;
        fn list_jobs(&self, params: &ListJobsParams) -> Result<Vec<JobInfo>>;
        fn inspect_job(&self, job_id: &str, namespace: Option<&str>) -> Result<JobInfo>;
        fn cancel_job(&self, job_id: &str, namespace: Option<&str>) -> Result<JobInfo>;
        fn fetch_job_logs(&self, job_id: &str, namespace: Option<&str>) -> Result<Vec<JobLogEntry>>;
        fn fetch_job_metrics(&self, job_id: &str, namespace: Option<&str>) -> Result<Vec<JobMetrics>>;
        fn list_job_hardware(&self) -> Result<Vec<JobHardware>>;
        fn create_scheduled_job(&self, params: &CreateScheduledJobParams) -> Result<ScheduledJobInfo>;
        fn list_scheduled_jobs(&self, namespace: Option<&str>) -> Result<Vec<ScheduledJobInfo>>;
        fn inspect_scheduled_job(&self, scheduled_job_id: &str, namespace: Option<&str>) -> Result<ScheduledJobInfo>;
        fn delete_scheduled_job(&self, scheduled_job_id: &str, namespace: Option<&str>) -> Result<()>;
        fn suspend_scheduled_job(&self, scheduled_job_id: &str, namespace: Option<&str>) -> Result<ScheduledJobInfo>;
        fn resume_scheduled_job(&self, scheduled_job_id: &str, namespace: Option<&str>) -> Result<ScheduledJobInfo>;
    }
}

fn parse_job_log_sse(body: &str) -> Vec<JobLogEntry> {
    body.lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .filter_map(|json_str| serde_json::from_str::<JobLogEntry>(json_str).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_job_log_sse() {
        // Real SSE response from the HF Jobs logs endpoint
        let sse_body = "\
data: {\"data\":\"===== Job started at 2026-03-15 22:06:01 =====\",\"timestamp\":\"2026-03-15T22:06:01Z\"}\n\
\n\
data: {\"data\":\"hello from integration test\",\"timestamp\":\"2026-03-15T22:06:06.155Z\"}\n";

        let entries = parse_job_log_sse(sse_body);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].data.as_deref(), Some("===== Job started at 2026-03-15 22:06:01 ====="));
        assert_eq!(entries[0].timestamp.as_deref(), Some("2026-03-15T22:06:01Z"));
        assert_eq!(entries[1].data.as_deref(), Some("hello from integration test"));
        assert_eq!(entries[1].timestamp.as_deref(), Some("2026-03-15T22:06:06.155Z"));
    }

    #[test]
    fn test_parse_job_log_sse_skips_non_data_lines() {
        let sse_body =
            ": keep-alive\n\ndata: {\"data\":\"output line\",\"timestamp\":\"2026-03-15T22:06:06Z\"}\n\n: keep-alive\n";
        let entries = parse_job_log_sse(sse_body);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].data.as_deref(), Some("output line"));
    }

    #[test]
    fn test_parse_job_log_sse_empty() {
        let entries = parse_job_log_sse("");
        assert!(entries.is_empty());
    }
}
