use std::process::Command;
use std::time::Duration;

pub struct CliRunner {
    bin: String,
    bin_path: Option<String>,
    token: Option<String>,
}

pub const VOLATILE_FIELDS: &[&str] = &[
    "downloads",
    "downloadsAllTime",
    "trendingScore",
    "lastModified",
    "likes",
    "sha",
    "trending_score",
    "downloads_all_time",
    "last_modified",
];

impl CliRunner {
    pub fn new(bin: &str) -> Self {
        Self {
            bin: bin.to_string(),
            bin_path: None,
            token: std::env::var("HF_TOKEN").ok(),
        }
    }

    pub fn hfrs() -> Self {
        Self {
            bin: "hfrs".to_string(),
            bin_path: Some(env!("CARGO_BIN_EXE_hfrs").to_string()),
            token: std::env::var("HF_TOKEN").ok(),
        }
    }

    pub fn is_available(&self) -> bool {
        if self.bin_path.is_some() {
            return true;
        }
        Command::new("which")
            .arg(&self.bin)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn build_command(&self, args: &[&str], extra_args: &[&str]) -> Command {
        let bin = self.bin_path.as_deref().unwrap_or(&self.bin);
        let mut cmd = Command::new(bin);
        cmd.args(args);
        cmd.args(extra_args);
        if let Some(ref token) = self.token {
            cmd.env("HF_TOKEN", token);
        }
        cmd
    }

    fn run_with_timeout(&self, mut cmd: Command, args: &[&str]) -> anyhow::Result<std::process::Output> {
        let mut child = cmd
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        let timeout = Duration::from_secs(60);
        let start = std::time::Instant::now();
        loop {
            match child.try_wait()? {
                Some(_status) => {
                    return Ok(child.wait_with_output()?);
                },
                None => {
                    if start.elapsed() > timeout {
                        let _ = child.kill();
                        anyhow::bail!("{} {:?} timed out after {}s", self.bin, args, timeout.as_secs());
                    }
                    std::thread::sleep(Duration::from_millis(100));
                },
            }
        }
    }

    pub fn run_json(&self, args: &[&str]) -> anyhow::Result<serde_json::Value> {
        let cmd = self.build_command(args, &["--format", "json"]);
        let output = self.run_with_timeout(cmd, args)?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("{} {:?} failed (exit {}): {}", self.bin, args, output.status, stderr);
        }
        let stdout = String::from_utf8(output.stdout)?;
        let value: serde_json::Value = serde_json::from_str(&stdout)?;
        Ok(value)
    }

    pub fn run_raw(&self, args: &[&str]) -> anyhow::Result<String> {
        let cmd = self.build_command(args, &[]);
        let output = self.run_with_timeout(cmd, args)?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("{} {:?} failed (exit {}): {}", self.bin, args, output.status, stderr);
        }
        Ok(String::from_utf8(output.stdout)?)
    }

    pub fn run_expecting_failure(&self, args: &[&str]) -> anyhow::Result<(i32, String)> {
        let cmd = self.build_command(args, &[]);
        let output = self.run_with_timeout(cmd, args)?;
        let stderr = String::from_utf8(output.stderr)?;
        let code = output.status.code().unwrap_or(-1);
        Ok((code, stderr))
    }
}

pub fn require_cli(runner: &CliRunner) {
    if !runner.is_available() {
        panic!("Required CLI '{}' not found on PATH. Install it before running integration tests.", runner.bin);
    }
}

pub fn require_token() {
    if std::env::var("HF_TOKEN").is_err() {
        panic!("HF_TOKEN environment variable is required for integration tests.");
    }
}

pub fn require_write() {
    if std::env::var("HF_TEST_WRITE").is_err() {
        panic!("HF_TEST_WRITE=1 is required for write operation tests.");
    }
}

pub fn assert_json_equivalent(actual: &serde_json::Value, expected: &serde_json::Value, ignore_fields: &[&str]) {
    assert_json_equivalent_at_path(actual, expected, ignore_fields, "");
}

fn assert_json_equivalent_at_path(
    actual: &serde_json::Value,
    expected: &serde_json::Value,
    ignore_fields: &[&str],
    path: &str,
) {
    match (actual, expected) {
        (serde_json::Value::Object(a), serde_json::Value::Object(e)) => {
            for (key, e_val) in e {
                if ignore_fields.contains(&key.as_str()) {
                    continue;
                }
                let current_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                match a.get(key) {
                    Some(a_val) => {
                        assert_json_equivalent_at_path(a_val, e_val, ignore_fields, &current_path);
                    },
                    None => {
                        panic!("Missing key at '{current_path}': expected {e_val}");
                    },
                }
            }
        },
        (serde_json::Value::Array(a), serde_json::Value::Array(e)) => {
            assert_eq!(
                a.len(),
                e.len(),
                "Array length mismatch at '{path}': actual {} vs expected {}",
                a.len(),
                e.len()
            );
            for (i, (a_item, e_item)) in a.iter().zip(e.iter()).enumerate() {
                let current_path = format!("{path}[{i}]");
                assert_json_equivalent_at_path(a_item, e_item, ignore_fields, &current_path);
            }
        },
        _ => {
            assert_eq!(actual, expected, "Value mismatch at '{path}': actual {actual} vs expected {expected}");
        },
    }
}
