use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobInfo {
    pub id: String,
    pub created_at: Option<String>,
    pub docker_image: Option<String>,
    pub space_id: Option<String>,
    #[serde(default)]
    pub command: Vec<String>,
    #[serde(default)]
    pub arguments: Vec<String>,
    #[serde(default)]
    pub environment: serde_json::Value,
    #[serde(default)]
    pub secrets: serde_json::Value,
    pub flavor: Option<String>,
    pub status: Option<JobStatus>,
    pub owner: Option<JobOwner>,
    pub endpoint: Option<String>,
    pub url: Option<String>,
    #[serde(default)]
    pub labels: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobStatus {
    pub stage: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobOwner {
    pub id: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobLogEntry {
    pub timestamp: Option<String>,
    pub data: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JobMetrics {
    pub cpu_usage_pct: Option<f64>,
    pub cpu_millicores: Option<u64>,
    pub memory_used_bytes: Option<u64>,
    pub memory_total_bytes: Option<u64>,
    pub rx_bps: Option<f64>,
    pub tx_bps: Option<f64>,
    pub gpus: Option<serde_json::Value>,
    pub replica: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobHardware {
    pub name: Option<String>,
    pub pretty_name: Option<String>,
    pub cpu: Option<String>,
    pub ram: Option<String>,
    pub accelerator: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledJobInfo {
    pub id: String,
    pub created_at: Option<String>,
    pub docker_image: Option<String>,
    #[serde(default)]
    pub command: Vec<String>,
    pub schedule: Option<String>,
    pub flavor: Option<String>,
    pub suspended: Option<bool>,
    pub owner: Option<JobOwner>,
    pub url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_info_deserialize() {
        let json = r#"{
            "id": "abc123",
            "dockerImage": "python:3.12",
            "command": ["python", "-c", "print('hi')"],
            "flavor": "cpu-basic",
            "status": {"stage": "COMPLETED"}
        }"#;
        let job: JobInfo = serde_json::from_str(json).unwrap();
        assert_eq!(job.id, "abc123");
        assert_eq!(job.status.as_ref().unwrap().stage.as_deref(), Some("COMPLETED"));
    }

    #[test]
    fn test_job_metrics_deserialize() {
        let json = r#"{"cpu_usage_pct": 50.0, "memory_used_bytes": 1024}"#;
        let m: JobMetrics = serde_json::from_str(json).unwrap();
        assert_eq!(m.cpu_usage_pct, Some(50.0));
    }

    #[test]
    fn test_scheduled_job_info_deserialize() {
        let json = r#"{"id": "sched1", "schedule": "@hourly", "suspended": false}"#;
        let sj: ScheduledJobInfo = serde_json::from_str(json).unwrap();
        assert_eq!(sj.id, "sched1");
        assert_eq!(sj.suspended, Some(false));
    }
}
