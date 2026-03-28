# hfrs CLI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Rust CLI tool `hfrs` that mirrors the Python `hf` CLI, backed by the `huggingface-hub` crate.

**Architecture:** New `hfrs/` crate in the workspace using clap derive for arg parsing, with a centralized output system (table/json/quiet). Each command group gets a nested module directory. Integration tests shell out to `hf`, `hfjs`, and `hfrs` comparing JSON output.

**Tech Stack:** clap (derive), tokio, serde_json, comfy-table, anyhow, huggingface-hub (all features)

**Spec:** `docs/superpowers/specs/2026-03-27-hfrs-cli-design.md`

---

## File Structure

```
hfrs/
├── Cargo.toml
├── README.md
├── src/
│   ├── main.rs
│   ├── cli.rs
│   ├── output.rs
│   ├── util/
│   │   ├── mod.rs
│   │   └── token.rs
│   └── commands/
│       ├── mod.rs
│       ├── env.rs
│       ├── version.rs
│       ├── download.rs
│       ├── upload.rs
│       ├── auth/
│       │   ├── mod.rs
│       │   ├── login.rs
│       │   ├── logout.rs
│       │   ├── switch.rs
│       │   ├── list.rs
│       │   └── whoami.rs
│       ├── models/
│       │   ├── mod.rs
│       │   ├── info.rs
│       │   └── list.rs
│       ├── datasets/
│       │   ├── mod.rs
│       │   ├── info.rs
│       │   └── list.rs
│       ├── spaces/
│       │   ├── mod.rs
│       │   ├── info.rs
│       │   └── list.rs
│       ├── repos/
│       │   ├── mod.rs
│       │   ├── create.rs
│       │   ├── delete.rs
│       │   ├── move_repo.rs
│       │   ├── settings.rs
│       │   ├── delete_files.rs
│       │   ├── branch.rs
│       │   └── tag.rs
│       ├── discussions/
│       │   ├── mod.rs
│       │   ├── list.rs
│       │   ├── info.rs
│       │   ├── create.rs
│       │   ├── comment.rs
│       │   ├── merge.rs
│       │   ├── close.rs
│       │   ├── reopen.rs
│       │   ├── rename.rs
│       │   └── diff.rs
│       ├── collections/
│       │   ├── mod.rs
│       │   ├── info.rs
│       │   ├── list.rs
│       │   ├── create.rs
│       │   ├── delete.rs
│       │   ├── update.rs
│       │   ├── add_item.rs
│       │   ├── update_item.rs
│       │   └── delete_item.rs
│       ├── webhooks/
│       │   ├── mod.rs
│       │   ├── list.rs
│       │   ├── info.rs
│       │   ├── create.rs
│       │   ├── update.rs
│       │   ├── delete.rs
│       │   ├── enable.rs
│       │   └── disable.rs
│       ├── endpoints/
│       │   ├── mod.rs
│       │   ├── list.rs
│       │   ├── describe.rs
│       │   ├── deploy.rs
│       │   ├── delete.rs
│       │   ├── pause.rs
│       │   ├── resume.rs
│       │   ├── scale_to_zero.rs
│       │   └── update.rs
│       ├── jobs/
│       │   ├── mod.rs
│       │   ├── run.rs
│       │   ├── ps.rs
│       │   ├── inspect.rs
│       │   ├── cancel.rs
│       │   ├── logs.rs
│       │   ├── hardware.rs
│       │   ├── stats.rs
│       │   └── scheduled/
│       │       ├── mod.rs
│       │       ├── run.rs
│       │       ├── ps.rs
│       │       ├── inspect.rs
│       │       ├── delete.rs
│       │       ├── suspend.rs
│       │       └── resume.rs
│       ├── papers/
│       │   ├── mod.rs
│       │   ├── info.rs
│       │   ├── list.rs
│       │   └── search.rs
│       ├── likes/
│       │   ├── mod.rs
│       │   ├── like.rs
│       │   ├── unlike.rs
│       │   └── list.rs
│       ├── access_requests/
│       │   ├── mod.rs
│       │   ├── list.rs
│       │   ├── accept.rs
│       │   ├── reject.rs
│       │   ├── cancel.rs
│       │   └── grant.rs
│       └── cache/
│           ├── mod.rs
│           ├── list.rs
│           └── rm.rs
└── tests/
    ├── cli_comparison.rs
    └── helpers/
        └── mod.rs
```

---

### Task 1: Crate Foundation

**Files:**
- Create: `hfrs/Cargo.toml`
- Modify: `Cargo.toml` (workspace root)
- Create: `hfrs/src/main.rs`
- Create: `hfrs/src/cli.rs`
- Create: `hfrs/src/output.rs`
- Create: `hfrs/src/commands/mod.rs`
- Create: `hfrs/src/util/mod.rs`

- [ ] **Step 1: Create `hfrs/Cargo.toml`**

```toml
[package]
name = "hfrs"
version = "0.1.0"
edition = "2021"
description = "Hugging Face Hub CLI (Rust)"
license = "Apache-2.0"

[[bin]]
name = "hfrs"
path = "src/main.rs"

[dependencies]
huggingface-hub = { path = "../huggingface_hub", features = [
    "blocking",
    "spaces",
    "inference_endpoints",
    "collections",
    "discussions",
    "webhooks",
    "jobs",
    "access_requests",
    "likes",
    "papers",
] }
clap = { version = "4", features = ["derive", "env"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
comfy-table = "7"
dirs = "6"
anyhow = "1"
futures = "0.3"

[dev-dependencies]
assert_cmd = "2"
serde_json = "1"
```

- [ ] **Step 2: Add `hfrs` to workspace members in root `Cargo.toml`**

Change `members = ["huggingface_hub"]` to `members = ["huggingface_hub", "hfrs"]`.

- [ ] **Step 3: Create `hfrs/src/cli.rs`**

```rust
use clap::{Parser, Subcommand, ValueEnum};

use crate::commands;

#[derive(Parser)]
#[command(name = "hfrs", about = "Hugging Face Hub CLI (Rust)")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Authentication token (overrides stored token and HF_TOKEN env var)
    #[arg(long, global = true, env = "HF_TOKEN")]
    pub token: Option<String>,

    /// API endpoint override
    #[arg(long, global = true, env = "HF_ENDPOINT")]
    pub endpoint: Option<String>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Manage authentication (login, logout, etc.)
    Auth(commands::auth::AuthCommand),
    /// Manage local cache directory
    Cache(commands::cache::CacheCommand),
    /// Interact with collections on the Hub
    Collections(commands::collections::CollectionsCommand),
    /// Interact with datasets on the Hub
    Datasets(commands::datasets::DatasetsCommand),
    /// Manage discussions and pull requests on the Hub
    Discussions(commands::discussions::DiscussionsCommand),
    /// Download files from the Hub
    Download(commands::download::DownloadArgs),
    /// Manage Hugging Face Inference Endpoints
    Endpoints(commands::endpoints::EndpointsCommand),
    /// Run and manage Jobs on the Hub
    Jobs(commands::jobs::JobsCommand),
    /// Manage likes on the Hub
    Likes(commands::likes::LikesCommand),
    /// Interact with models on the Hub
    Models(commands::models::ModelsCommand),
    /// Interact with papers on the Hub
    Papers(commands::papers::PapersCommand),
    /// Manage repos on the Hub
    #[command(alias = "repo")]
    Repos(commands::repos::ReposCommand),
    /// Interact with spaces on the Hub
    Spaces(commands::spaces::SpacesCommand),
    /// Upload a file or folder to the Hub
    Upload(commands::upload::UploadArgs),
    /// Manage webhooks on the Hub
    Webhooks(commands::webhooks::WebhooksCommand),
    /// Manage gated repo access requests
    #[command(name = "access-requests")]
    AccessRequests(commands::access_requests::AccessRequestsCommand),
    /// Print information about the environment
    Env(commands::env::EnvArgs),
    /// Print the hfrs version
    Version(commands::version::VersionArgs),
}

#[derive(Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Table,
    Json,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum RepoTypeArg {
    Model,
    Dataset,
    Space,
}

impl From<RepoTypeArg> for huggingface_hub::RepoType {
    fn from(val: RepoTypeArg) -> Self {
        match val {
            RepoTypeArg::Model => huggingface_hub::RepoType::Model,
            RepoTypeArg::Dataset => huggingface_hub::RepoType::Dataset,
            RepoTypeArg::Space => huggingface_hub::RepoType::Space,
        }
    }
}
```

- [ ] **Step 4: Create `hfrs/src/output.rs`**

```rust
use comfy_table::{ContentArrangement, Table};

use crate::cli::OutputFormat;

pub enum CommandResult {
    Formatted {
        output: CommandOutput,
        format: OutputFormat,
        quiet: bool,
    },
    Raw(String),
    Silent,
}

pub struct CommandOutput {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub json_value: serde_json::Value,
    pub quiet_values: Vec<String>,
}

pub fn render(result: CommandResult) -> anyhow::Result<()> {
    match result {
        CommandResult::Silent => {}
        CommandResult::Raw(s) => {
            println!("{s}");
        }
        CommandResult::Formatted {
            output,
            format,
            quiet,
        } => {
            if quiet {
                for val in &output.quiet_values {
                    println!("{val}");
                }
            } else {
                match format {
                    OutputFormat::Json => {
                        println!("{}", serde_json::to_string_pretty(&output.json_value)?);
                    }
                    OutputFormat::Table => {
                        render_table(&output.headers, &output.rows);
                    }
                }
            }
        }
    }
    Ok(())
}

fn render_table(headers: &[String], rows: &[Vec<String>]) {
    if rows.is_empty() {
        return;
    }
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(headers);
    for row in rows {
        table.add_row(row);
    }
    println!("{table}");
}

impl CommandOutput {
    pub fn single_item(json_value: serde_json::Value) -> Self {
        let mut headers = vec!["Field".to_string(), "Value".to_string()];
        let mut rows = Vec::new();
        let mut quiet_values = Vec::new();

        if let serde_json::Value::Object(map) = &json_value {
            for (key, val) in map {
                let display = match val {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Null => "null".to_string(),
                    other => other.to_string(),
                };
                rows.push(vec![key.clone(), display]);
            }
            if let Some(id) = map.get("id").and_then(|v| v.as_str()) {
                quiet_values.push(id.to_string());
            }
        }

        // If no rows were added (not an object), use empty headers
        if rows.is_empty() {
            headers = Vec::new();
        }

        Self {
            headers,
            rows,
            json_value,
            quiet_values,
        }
    }
}
```

- [ ] **Step 5: Create `hfrs/src/commands/mod.rs` (stub modules)**

```rust
pub mod auth;
pub mod cache;
pub mod collections;
pub mod datasets;
pub mod discussions;
pub mod download;
pub mod endpoints;
pub mod env;
pub mod jobs;
pub mod likes;
pub mod models;
pub mod papers;
pub mod repos;
pub mod spaces;
pub mod upload;
pub mod version;
pub mod webhooks;
pub mod access_requests;
```

- [ ] **Step 6: Create `hfrs/src/util/mod.rs`**

```rust
pub mod token;
```

- [ ] **Step 7: Create `hfrs/src/main.rs`**

```rust
mod cli;
mod commands;
mod output;
mod util;

use clap::Parser;
use huggingface_hub::HfApi;

use cli::Cli;
use output::render;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let mut builder = HfApi::builder();
    if let Some(ref token) = cli.token {
        builder = builder.token(token);
    } else if let Some(token) = util::token::read_active_token() {
        builder = builder.token(token);
    }
    if let Some(ref endpoint) = cli.endpoint {
        builder = builder.endpoint(endpoint);
    }
    let api = builder.build()?;

    let result = match cli.command {
        cli::Command::Auth(cmd) => cmd.execute(&api).await?,
        cli::Command::Cache(cmd) => cmd.execute(&api).await?,
        cli::Command::Collections(cmd) => cmd.execute(&api).await?,
        cli::Command::Datasets(cmd) => cmd.execute(&api).await?,
        cli::Command::Discussions(cmd) => cmd.execute(&api).await?,
        cli::Command::Download(args) => args.execute(&api).await?,
        cli::Command::Endpoints(cmd) => cmd.execute(&api).await?,
        cli::Command::Jobs(cmd) => cmd.execute(&api).await?,
        cli::Command::Likes(cmd) => cmd.execute(&api).await?,
        cli::Command::Models(cmd) => cmd.execute(&api).await?,
        cli::Command::Papers(cmd) => cmd.execute(&api).await?,
        cli::Command::Repos(cmd) => cmd.execute(&api).await?,
        cli::Command::Spaces(cmd) => cmd.execute(&api).await?,
        cli::Command::Upload(args) => args.execute(&api).await?,
        cli::Command::Webhooks(cmd) => cmd.execute(&api).await?,
        cli::Command::AccessRequests(cmd) => cmd.execute(&api).await?,
        cli::Command::Env(args) => args.execute()?,
        cli::Command::Version(args) => args.execute()?,
    };

    render(result)?;

    Ok(())
}
```

- [ ] **Step 8: Create stub command modules so it compiles**

Create minimal stubs for every command module so the project compiles. Each stub just needs the struct with `clap::Args` derive and an `execute` method that returns `Ok(CommandResult::Silent)`. I'll list the pattern for auth; repeat for all others.

Create `hfrs/src/util/token.rs`:
```rust
pub fn read_active_token() -> Option<String> {
    None
}
```

Create `hfrs/src/commands/auth/mod.rs`:
```rust
pub mod list;
pub mod login;
pub mod logout;
pub mod switch;
pub mod whoami;

use clap::{Args, Subcommand};
use huggingface_hub::HfApi;

use crate::output::CommandResult;

#[derive(Args)]
#[command(about = "Manage authentication (login, logout, etc.)")]
pub struct AuthCommand {
    #[command(subcommand)]
    pub command: AuthSubcommand,
}

#[derive(Subcommand)]
pub enum AuthSubcommand {
    /// Login using a token from huggingface.co/settings/tokens
    Login(login::LoginArgs),
    /// Logout from a specific token
    Logout(logout::LogoutArgs),
    /// Switch between access tokens
    Switch(switch::SwitchArgs),
    /// List all stored access tokens
    #[command(alias = "ls")]
    List(list::ListArgs),
    /// Find out which huggingface.co account you are logged in as
    Whoami(whoami::WhoamiArgs),
}

impl AuthCommand {
    pub async fn execute(&self, api: &HfApi) -> anyhow::Result<CommandResult> {
        match &self.command {
            AuthSubcommand::Login(args) => args.execute(api).await,
            AuthSubcommand::Logout(args) => args.execute(api).await,
            AuthSubcommand::Switch(args) => args.execute(api).await,
            AuthSubcommand::List(args) => args.execute(api).await,
            AuthSubcommand::Whoami(args) => args.execute(api).await,
        }
    }
}
```

Create leaf stubs for each auth subcommand (`login.rs`, `logout.rs`, `switch.rs`, `list.rs`, `whoami.rs`):
```rust
// hfrs/src/commands/auth/login.rs (pattern for all leaf stubs)
use clap::Args;
use huggingface_hub::HfApi;

use crate::output::CommandResult;

#[derive(Args)]
#[command(about = "Login using a token from huggingface.co/settings/tokens")]
pub struct LoginArgs {}

impl LoginArgs {
    pub async fn execute(&self, _api: &HfApi) -> anyhow::Result<CommandResult> {
        Ok(CommandResult::Silent)
    }
}
```

Repeat this pattern for **every** command module and leaf listed in the file structure above. Each command group `mod.rs` follows the `AuthCommand` pattern (struct wrapping a subcommand enum, enum dispatching to leaf args). Each leaf file follows the `LoginArgs` pattern (empty struct with `execute` returning `Silent`).

For groups without subcommands (`download.rs`, `upload.rs`, `env.rs`, `version.rs`), the leaf file itself is the module:
```rust
// hfrs/src/commands/env.rs
use clap::Args;

use crate::output::CommandResult;

#[derive(Args)]
#[command(about = "Print information about the environment")]
pub struct EnvArgs {}

impl EnvArgs {
    pub fn execute(&self) -> anyhow::Result<CommandResult> {
        Ok(CommandResult::Silent)
    }
}
```

```rust
// hfrs/src/commands/version.rs
use clap::Args;

use crate::output::CommandResult;

#[derive(Args)]
#[command(about = "Print the hfrs version")]
pub struct VersionArgs {}

impl VersionArgs {
    pub fn execute(&self) -> anyhow::Result<CommandResult> {
        Ok(CommandResult::Silent)
    }
}
```

```rust
// hfrs/src/commands/download.rs
use clap::Args;
use huggingface_hub::HfApi;

use crate::output::CommandResult;

#[derive(Args)]
#[command(about = "Download files from the Hub")]
pub struct DownloadArgs {}

impl DownloadArgs {
    pub async fn execute(&self, _api: &HfApi) -> anyhow::Result<CommandResult> {
        Ok(CommandResult::Silent)
    }
}
```

```rust
// hfrs/src/commands/upload.rs
use clap::Args;
use huggingface_hub::HfApi;

use crate::output::CommandResult;

#[derive(Args)]
#[command(about = "Upload a file or folder to the Hub")]
pub struct UploadArgs {}

impl UploadArgs {
    pub async fn execute(&self, _api: &HfApi) -> anyhow::Result<CommandResult> {
        Ok(CommandResult::Silent)
    }
}
```

For `jobs/scheduled/` subgroup, the `jobs/mod.rs` has a nested subcommand:
```rust
// hfrs/src/commands/jobs/mod.rs
pub mod cancel;
pub mod hardware;
pub mod inspect;
pub mod logs;
pub mod ps;
pub mod run;
pub mod scheduled;
pub mod stats;

use clap::{Args, Subcommand};
use huggingface_hub::HfApi;

use crate::output::CommandResult;

#[derive(Args)]
#[command(about = "Run and manage Jobs on the Hub")]
pub struct JobsCommand {
    #[command(subcommand)]
    pub command: JobsSubcommand,
}

#[derive(Subcommand)]
pub enum JobsSubcommand {
    /// Run a Job on the Hub
    Run(run::RunArgs),
    /// List Jobs
    #[command(alias = "ls")]
    Ps(ps::PsArgs),
    /// Inspect one or more Jobs
    Inspect(inspect::InspectArgs),
    /// Cancel a running Job
    Cancel(cancel::CancelArgs),
    /// Fetch logs for a Job
    Logs(logs::LogsArgs),
    /// List available hardware options for Jobs
    Hardware(hardware::HardwareArgs),
    /// Show resource usage stats for a Job
    Stats(stats::StatsArgs),
    /// Manage scheduled Jobs
    Scheduled(scheduled::ScheduledCommand),
}

impl JobsCommand {
    pub async fn execute(&self, api: &HfApi) -> anyhow::Result<CommandResult> {
        match &self.command {
            JobsSubcommand::Run(args) => args.execute(api).await,
            JobsSubcommand::Ps(args) => args.execute(api).await,
            JobsSubcommand::Inspect(args) => args.execute(api).await,
            JobsSubcommand::Cancel(args) => args.execute(api).await,
            JobsSubcommand::Logs(args) => args.execute(api).await,
            JobsSubcommand::Hardware(args) => args.execute(api).await,
            JobsSubcommand::Stats(args) => args.execute(api).await,
            JobsSubcommand::Scheduled(cmd) => cmd.execute(api).await,
        }
    }
}
```

```rust
// hfrs/src/commands/jobs/scheduled/mod.rs
pub mod delete;
pub mod inspect;
pub mod ps;
pub mod resume;
pub mod run;
pub mod suspend;

use clap::{Args, Subcommand};
use huggingface_hub::HfApi;

use crate::output::CommandResult;

#[derive(Args)]
#[command(about = "Manage scheduled Jobs")]
pub struct ScheduledCommand {
    #[command(subcommand)]
    pub command: ScheduledSubcommand,
}

#[derive(Subcommand)]
pub enum ScheduledSubcommand {
    /// Create a scheduled Job
    Run(run::RunArgs),
    /// List scheduled Jobs
    #[command(alias = "ls")]
    Ps(ps::PsArgs),
    /// Inspect scheduled Jobs
    Inspect(inspect::InspectArgs),
    /// Delete a scheduled Job
    Delete(delete::DeleteArgs),
    /// Suspend a scheduled Job
    Suspend(suspend::SuspendArgs),
    /// Resume a suspended scheduled Job
    Resume(resume::ResumeArgs),
}

impl ScheduledCommand {
    pub async fn execute(&self, api: &HfApi) -> anyhow::Result<CommandResult> {
        match &self.command {
            ScheduledSubcommand::Run(args) => args.execute(api).await,
            ScheduledSubcommand::Ps(args) => args.execute(api).await,
            ScheduledSubcommand::Inspect(args) => args.execute(api).await,
            ScheduledSubcommand::Delete(args) => args.execute(api).await,
            ScheduledSubcommand::Suspend(args) => args.execute(api).await,
            ScheduledSubcommand::Resume(args) => args.execute(api).await,
        }
    }
}
```

- [ ] **Step 9: Verify it compiles**

Run: `cargo build -p hfrs`
Expected: Compiles with no errors (warnings about unused variables are OK for now)

- [ ] **Step 10: Verify `hfrs --help` shows all commands**

Run: `cargo run -p hfrs -- --help`
Expected: Shows all top-level commands (auth, cache, collections, datasets, discussions, download, endpoints, jobs, likes, models, papers, repos, spaces, upload, webhooks, access-requests, env, version)

- [ ] **Step 11: Format and lint**

Run: `cargo +nightly fmt && cargo clippy -p hfrs -- -D warnings`
Expected: No errors

- [ ] **Step 12: Commit**

```bash
git add hfrs/ Cargo.toml
git commit -m "feat(hfrs): scaffold CLI crate with all command stubs"
```

---

### Task 2: Token Utilities

**Files:**
- Create: `hfrs/src/util/token.rs` (replace stub)

- [ ] **Step 1: Investigate Python CLI token format**

Run: `python3 -c "from huggingface_hub import get_token; print(get_token())"` to verify token file location.
Run: `cat ~/.cache/huggingface/token` to see format.
Run: `ls ~/.cache/huggingface/stored_tokens` to check if multi-token file exists.
Run: `python3 -c "from huggingface_hub.utils._token import _get_token_from_file; help(_get_token_from_file)"` for details.

Document findings before implementing.

- [ ] **Step 2: Implement `hfrs/src/util/token.rs`**

```rust
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredTokens {
    #[serde(default)]
    tokens: HashMap<String, StoredToken>,
    #[serde(default)]
    active: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredToken {
    token: String,
}

#[derive(Debug, Clone)]
pub struct TokenEntry {
    pub name: String,
    pub token_masked: String,
    pub is_active: bool,
}

fn hf_home() -> PathBuf {
    if let Ok(path) = std::env::var("HF_HOME") {
        return PathBuf::from(path);
    }
    if let Ok(path) = std::env::var("XDG_CACHE_HOME") {
        return PathBuf::from(path).join("huggingface");
    }
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("~/.cache"))
        .join("huggingface")
}

fn token_file_path() -> PathBuf {
    if let Ok(path) = std::env::var("HF_TOKEN_PATH") {
        return PathBuf::from(path);
    }
    hf_home().join("token")
}

fn stored_tokens_path() -> PathBuf {
    hf_home().join("stored_tokens")
}

fn mask_token(token: &str) -> String {
    if token.len() <= 8 {
        return "*".repeat(token.len());
    }
    let prefix = &token[..4];
    let suffix = &token[token.len() - 4..];
    format!("{prefix}...{suffix}")
}

fn read_stored_tokens() -> StoredTokens {
    let path = stored_tokens_path();
    if !path.exists() {
        return StoredTokens {
            tokens: HashMap::new(),
            active: None,
        };
    }
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(StoredTokens {
            tokens: HashMap::new(),
            active: None,
        })
}

fn write_stored_tokens(stored: &StoredTokens) -> anyhow::Result<()> {
    let path = stored_tokens_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(stored)?;
    fs::write(&path, json)?;
    Ok(())
}

fn write_active_token_file(token: &str) -> anyhow::Result<()> {
    let path = token_file_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, token)?;
    Ok(())
}

pub fn read_active_token() -> Option<String> {
    // Check stored_tokens first for multi-token support
    let stored = read_stored_tokens();
    if let Some(active_name) = &stored.active {
        if let Some(entry) = stored.tokens.get(active_name) {
            return Some(entry.token.clone());
        }
    }
    // Fall back to legacy single-token file
    let path = token_file_path();
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

pub fn save_token(name: &str, token: &str) -> anyhow::Result<()> {
    let mut stored = read_stored_tokens();
    stored.tokens.insert(
        name.to_string(),
        StoredToken {
            token: token.to_string(),
        },
    );
    // If no active token, set this one as active
    if stored.active.is_none() {
        stored.active = Some(name.to_string());
    }
    write_stored_tokens(&stored)?;
    // Also write the active token to the legacy file if this is now active
    if stored.active.as_deref() == Some(name) {
        write_active_token_file(token)?;
    }
    Ok(())
}

pub fn delete_token(name: &str) -> anyhow::Result<()> {
    let mut stored = read_stored_tokens();
    stored.tokens.remove(name);
    if stored.active.as_deref() == Some(name) {
        stored.active = stored.tokens.keys().next().cloned();
        // Update legacy file
        if let Some(ref active_name) = stored.active {
            if let Some(entry) = stored.tokens.get(active_name) {
                write_active_token_file(&entry.token)?;
            }
        }
    }
    write_stored_tokens(&stored)?;
    Ok(())
}

pub fn switch_token(name: &str) -> anyhow::Result<()> {
    let mut stored = read_stored_tokens();
    if !stored.tokens.contains_key(name) {
        anyhow::bail!("Token '{}' not found. Use `hfrs auth list` to see stored tokens.", name);
    }
    stored.active = Some(name.to_string());
    let token = stored.tokens[name].token.clone();
    write_stored_tokens(&stored)?;
    write_active_token_file(&token)?;
    Ok(())
}

pub fn list_tokens() -> Vec<TokenEntry> {
    let stored = read_stored_tokens();
    let mut entries: Vec<TokenEntry> = stored
        .tokens
        .iter()
        .map(|(name, entry)| TokenEntry {
            name: name.clone(),
            token_masked: mask_token(&entry.token),
            is_active: stored.active.as_deref() == Some(name.as_str()),
        })
        .collect();
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build -p hfrs`
Expected: Compiles

- [ ] **Step 4: Format and lint**

Run: `cargo +nightly fmt && cargo clippy -p hfrs -- -D warnings`
Expected: No errors

- [ ] **Step 5: Commit**

```bash
git add hfrs/src/util/
git commit -m "feat(hfrs): implement token file read/write/switch/list"
```

---

### Task 3: Env + Version Commands

**Files:**
- Modify: `hfrs/src/commands/env.rs`
- Modify: `hfrs/src/commands/version.rs`

- [ ] **Step 1: Implement `env.rs`**

```rust
use clap::Args;

use crate::output::CommandResult;

#[derive(Args)]
#[command(about = "Print information about the environment")]
pub struct EnvArgs {}

impl EnvArgs {
    pub fn execute(&self) -> anyhow::Result<CommandResult> {
        let mut lines = Vec::new();
        lines.push(format!("hfrs version: {}", env!("CARGO_PKG_VERSION")));
        lines.push(format!(
            "Platform: {} {}",
            std::env::consts::OS,
            std::env::consts::ARCH
        ));
        if let Ok(endpoint) = std::env::var("HF_ENDPOINT") {
            lines.push(format!("HF_ENDPOINT: {endpoint}"));
        }
        if std::env::var("HF_TOKEN").is_ok() {
            lines.push("HF_TOKEN: set".to_string());
        } else {
            lines.push("HF_TOKEN: not set".to_string());
        }
        if let Ok(home) = std::env::var("HF_HOME") {
            lines.push(format!("HF_HOME: {home}"));
        }
        if let Ok(cache) = std::env::var("HF_HUB_CACHE") {
            lines.push(format!("HF_HUB_CACHE: {cache}"));
        }
        Ok(CommandResult::Raw(lines.join("\n")))
    }
}
```

- [ ] **Step 2: Implement `version.rs`**

```rust
use clap::Args;

use crate::output::CommandResult;

#[derive(Args)]
#[command(about = "Print the hfrs version")]
pub struct VersionArgs {}

impl VersionArgs {
    pub fn execute(&self) -> anyhow::Result<CommandResult> {
        Ok(CommandResult::Raw(format!(
            "hfrs {}",
            env!("CARGO_PKG_VERSION")
        )))
    }
}
```

- [ ] **Step 3: Verify**

Run: `cargo run -p hfrs -- version`
Expected: `hfrs 0.1.0`

Run: `cargo run -p hfrs -- env`
Expected: Shows platform, env vars

- [ ] **Step 4: Format and lint**

Run: `cargo +nightly fmt && cargo clippy -p hfrs -- -D warnings`

- [ ] **Step 5: Commit**

```bash
git add hfrs/src/commands/env.rs hfrs/src/commands/version.rs
git commit -m "feat(hfrs): implement env and version commands"
```

---

### Task 4: Auth Commands

**Files:**
- Modify: `hfrs/src/commands/auth/login.rs`
- Modify: `hfrs/src/commands/auth/logout.rs`
- Modify: `hfrs/src/commands/auth/switch.rs`
- Modify: `hfrs/src/commands/auth/list.rs`
- Modify: `hfrs/src/commands/auth/whoami.rs`

- [ ] **Step 1: Implement `auth/whoami.rs`**

```rust
use clap::Args;
use huggingface_hub::HfApi;

use crate::cli::OutputFormat;
use crate::output::{CommandOutput, CommandResult};

#[derive(Args)]
#[command(about = "Find out which huggingface.co account you are logged in as")]
pub struct WhoamiArgs {
    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    format: OutputFormat,
}

impl WhoamiArgs {
    pub async fn execute(&self, api: &HfApi) -> anyhow::Result<CommandResult> {
        let user = api.whoami().await?;
        let json_value = serde_json::to_value(&user)?;

        let headers = vec![
            "Username".to_string(),
            "Full Name".to_string(),
            "Email".to_string(),
        ];
        let rows = vec![vec![
            user.username.clone(),
            user.fullname.clone().unwrap_or_default(),
            user.email.clone().unwrap_or_default(),
        ]];
        let quiet_values = vec![user.username.clone()];

        Ok(CommandResult::Formatted {
            output: CommandOutput {
                headers,
                rows,
                json_value,
                quiet_values,
            },
            format: self.format,
            quiet: false,
        })
    }
}
```

- [ ] **Step 2: Implement `auth/login.rs`**

```rust
use clap::Args;
use huggingface_hub::HfApi;

use crate::output::CommandResult;
use crate::util::token;

#[derive(Args)]
#[command(about = "Login using a token from huggingface.co/settings/tokens")]
pub struct LoginArgs {
    /// A User Access Token
    #[arg(long)]
    token_value: Option<String>,

    /// Name for this token (used to identify it in `auth list`)
    #[arg(long, default_value = "default")]
    token_name: String,
}

impl LoginArgs {
    pub async fn execute(&self, _api: &HfApi) -> anyhow::Result<CommandResult> {
        let token_value = match &self.token_value {
            Some(t) => t.clone(),
            None => {
                anyhow::bail!(
                    "Please provide a token with --token-value. \
                     Get one at https://huggingface.co/settings/tokens"
                );
            }
        };

        token::save_token(&self.token_name, &token_value)?;
        Ok(CommandResult::Raw(format!(
            "Token saved as '{}'.",
            self.token_name
        )))
    }
}
```

- [ ] **Step 3: Implement `auth/logout.rs`**

```rust
use clap::Args;
use huggingface_hub::HfApi;

use crate::output::CommandResult;
use crate::util::token;

#[derive(Args)]
#[command(about = "Logout from a specific token")]
pub struct LogoutArgs {
    /// Name of the token to remove
    #[arg(long)]
    token_name: Option<String>,
}

impl LogoutArgs {
    pub async fn execute(&self, _api: &HfApi) -> anyhow::Result<CommandResult> {
        let name = self.token_name.as_deref().unwrap_or("default");
        token::delete_token(name)?;
        Ok(CommandResult::Raw(format!("Token '{name}' removed.")))
    }
}
```

- [ ] **Step 4: Implement `auth/switch.rs`**

```rust
use clap::Args;
use huggingface_hub::HfApi;

use crate::output::CommandResult;
use crate::util::token;

#[derive(Args)]
#[command(about = "Switch between access tokens")]
pub struct SwitchArgs {
    /// Name of the token to switch to
    #[arg(long)]
    token_name: String,
}

impl SwitchArgs {
    pub async fn execute(&self, _api: &HfApi) -> anyhow::Result<CommandResult> {
        token::switch_token(&self.token_name)?;
        Ok(CommandResult::Raw(format!(
            "Switched to token '{}'.",
            self.token_name
        )))
    }
}
```

- [ ] **Step 5: Implement `auth/list.rs`**

```rust
use clap::Args;
use huggingface_hub::HfApi;

use crate::output::CommandResult;
use crate::util::token;

#[derive(Args)]
#[command(about = "List all stored access tokens")]
pub struct ListArgs {}

impl ListArgs {
    pub async fn execute(&self, _api: &HfApi) -> anyhow::Result<CommandResult> {
        let tokens = token::list_tokens();
        if tokens.is_empty() {
            return Ok(CommandResult::Raw(
                "No tokens stored. Run `hfrs auth login` to add one.".to_string(),
            ));
        }
        let mut lines = Vec::new();
        for entry in &tokens {
            let active = if entry.is_active { " (active)" } else { "" };
            lines.push(format!(
                "  {} — {}{}",
                entry.name, entry.token_masked, active
            ));
        }
        Ok(CommandResult::Raw(lines.join("\n")))
    }
}
```

- [ ] **Step 6: Verify**

Run: `cargo run -p hfrs -- auth --help`
Expected: Shows login, logout, switch, list, whoami subcommands

Run: `cargo run -p hfrs -- auth list`
Expected: Shows stored tokens or "No tokens stored"

- [ ] **Step 7: Format and lint**

Run: `cargo +nightly fmt && cargo clippy -p hfrs -- -D warnings`

- [ ] **Step 8: Commit**

```bash
git add hfrs/src/commands/auth/
git commit -m "feat(hfrs): implement auth commands (login, logout, switch, list, whoami)"
```

---

### Task 5: Models Commands (info, list)

This establishes the core pattern for all info/list command groups.

**Files:**
- Modify: `hfrs/src/commands/models/mod.rs`
- Modify: `hfrs/src/commands/models/info.rs`
- Modify: `hfrs/src/commands/models/list.rs`

- [ ] **Step 1: Implement `models/mod.rs`**

```rust
pub mod info;
pub mod list;

use clap::{Args, Subcommand};
use huggingface_hub::HfApi;

use crate::output::CommandResult;

#[derive(Args)]
#[command(about = "Interact with models on the Hub")]
pub struct ModelsCommand {
    #[command(subcommand)]
    pub command: ModelsSubcommand,
}

#[derive(Subcommand)]
pub enum ModelsSubcommand {
    /// Get info about a model
    Info(info::InfoArgs),
    /// List models on the Hub
    #[command(alias = "ls")]
    List(list::ListArgs),
}

impl ModelsCommand {
    pub async fn execute(&self, api: &HfApi) -> anyhow::Result<CommandResult> {
        match &self.command {
            ModelsSubcommand::Info(args) => args.execute(api).await,
            ModelsSubcommand::List(args) => args.execute(api).await,
        }
    }
}
```

- [ ] **Step 2: Implement `models/info.rs`**

```rust
use clap::Args;
use huggingface_hub::{HfApi, ModelInfoParams};

use crate::cli::OutputFormat;
use crate::output::{CommandOutput, CommandResult};

#[derive(Args)]
#[command(about = "Get info about a model")]
pub struct InfoArgs {
    /// Model ID (e.g. 'gpt2' or 'meta-llama/Llama-3.2-1B-Instruct')
    pub model_id: String,

    /// Revision (branch, tag, or commit hash)
    #[arg(long)]
    pub revision: Option<String>,

    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    pub format: OutputFormat,
}

impl InfoArgs {
    pub async fn execute(&self, api: &HfApi) -> anyhow::Result<CommandResult> {
        let mut builder = ModelInfoParams::builder().repo_id(&self.model_id);
        if let Some(ref rev) = self.revision {
            builder = builder.revision(rev);
        }
        let params = builder.build();
        let info = api.model_info(&params).await?;
        let json_value = serde_json::to_value(&info)?;

        Ok(CommandResult::Formatted {
            output: CommandOutput::single_item(json_value),
            format: self.format,
            quiet: false,
        })
    }
}
```

- [ ] **Step 3: Implement `models/list.rs`**

```rust
use clap::Args;
use futures::StreamExt;
use huggingface_hub::{HfApi, ListModelsParams};

use crate::cli::OutputFormat;
use crate::output::{CommandOutput, CommandResult};

#[derive(Args)]
#[command(about = "List models on the Hub")]
pub struct ListArgs {
    /// Filter by search query
    #[arg(long)]
    pub search: Option<String>,

    /// Filter by model author or organization
    #[arg(long)]
    pub author: Option<String>,

    /// Filter by tags. Can be specified multiple times
    #[arg(long)]
    pub filter: Vec<String>,

    /// Sort order for results
    #[arg(long)]
    pub sort: Option<String>,

    /// Maximum number of results to return
    #[arg(long, default_value = "10")]
    pub limit: usize,

    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    pub format: OutputFormat,

    /// Only print model IDs, one per line
    #[arg(short, long)]
    pub quiet: bool,
}

impl ListArgs {
    pub async fn execute(&self, api: &HfApi) -> anyhow::Result<CommandResult> {
        let filter_str = if self.filter.is_empty() {
            None
        } else {
            Some(self.filter.join(","))
        };

        let mut builder = ListModelsParams::builder().limit(self.limit);
        if let Some(ref search) = self.search {
            builder = builder.search(search);
        }
        if let Some(ref author) = self.author {
            builder = builder.author(author);
        }
        if let Some(ref filter) = filter_str {
            builder = builder.filter(filter);
        }
        if let Some(ref sort) = self.sort {
            builder = builder.sort(sort);
        }
        let params = builder.build();

        let stream = api.list_models(&params);
        futures::pin_mut!(stream);
        let mut models = Vec::new();
        while let Some(item) = stream.next().await {
            models.push(item?);
        }

        let json_value = serde_json::to_value(&models)?;

        let headers = vec![
            "ID".to_string(),
            "Author".to_string(),
            "Downloads".to_string(),
            "Likes".to_string(),
            "Pipeline".to_string(),
        ];

        let rows: Vec<Vec<String>> = models
            .iter()
            .map(|m| {
                vec![
                    m.id.clone(),
                    m.author.clone().unwrap_or_default(),
                    m.downloads.map(|d| d.to_string()).unwrap_or_default(),
                    m.likes.map(|l| l.to_string()).unwrap_or_default(),
                    m.pipeline_tag.clone().unwrap_or_default(),
                ]
            })
            .collect();

        let quiet_values: Vec<String> = models.iter().map(|m| m.id.clone()).collect();

        Ok(CommandResult::Formatted {
            output: CommandOutput {
                headers,
                rows,
                json_value,
                quiet_values,
            },
            format: self.format,
            quiet: self.quiet,
        })
    }
}
```

- [ ] **Step 4: Verify**

Run: `cargo run -p hfrs -- models list --limit 3 --format json`
Expected: JSON array of 3 models

Run: `cargo run -p hfrs -- models info gpt2 --format json`
Expected: JSON object with model info

- [ ] **Step 5: Format and lint**

Run: `cargo +nightly fmt && cargo clippy -p hfrs -- -D warnings`

- [ ] **Step 6: Commit**

```bash
git add hfrs/src/commands/models/
git commit -m "feat(hfrs): implement models info and list commands"
```

---

### Task 6: Datasets Commands

**Files:**
- Modify: `hfrs/src/commands/datasets/mod.rs`
- Modify: `hfrs/src/commands/datasets/info.rs`
- Modify: `hfrs/src/commands/datasets/list.rs`

Follow the same pattern as Task 5 (models), using `DatasetInfoParams`, `ListDatasetsParams`, `dataset_info()`, `list_datasets()`. Table headers: `ID`, `Author`, `Downloads`, `Likes`.

- [ ] **Step 1: Implement `datasets/mod.rs`**

```rust
pub mod info;
pub mod list;

use clap::{Args, Subcommand};
use huggingface_hub::HfApi;

use crate::output::CommandResult;

#[derive(Args)]
#[command(about = "Interact with datasets on the Hub")]
pub struct DatasetsCommand {
    #[command(subcommand)]
    pub command: DatasetsSubcommand,
}

#[derive(Subcommand)]
pub enum DatasetsSubcommand {
    /// Get info about a dataset
    Info(info::InfoArgs),
    /// List datasets on the Hub
    #[command(alias = "ls")]
    List(list::ListArgs),
}

impl DatasetsCommand {
    pub async fn execute(&self, api: &HfApi) -> anyhow::Result<CommandResult> {
        match &self.command {
            DatasetsSubcommand::Info(args) => args.execute(api).await,
            DatasetsSubcommand::List(args) => args.execute(api).await,
        }
    }
}
```

- [ ] **Step 2: Implement `datasets/info.rs`**

```rust
use clap::Args;
use huggingface_hub::{DatasetInfoParams, HfApi};

use crate::cli::OutputFormat;
use crate::output::{CommandOutput, CommandResult};

#[derive(Args)]
#[command(about = "Get info about a dataset")]
pub struct InfoArgs {
    /// Dataset ID (e.g. 'squad' or 'lmsys/chatbot_arena_conversations')
    pub dataset_id: String,

    /// Revision (branch, tag, or commit hash)
    #[arg(long)]
    pub revision: Option<String>,

    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    pub format: OutputFormat,
}

impl InfoArgs {
    pub async fn execute(&self, api: &HfApi) -> anyhow::Result<CommandResult> {
        let mut builder = DatasetInfoParams::builder().repo_id(&self.dataset_id);
        if let Some(ref rev) = self.revision {
            builder = builder.revision(rev);
        }
        let params = builder.build();
        let info = api.dataset_info(&params).await?;
        let json_value = serde_json::to_value(&info)?;

        Ok(CommandResult::Formatted {
            output: CommandOutput::single_item(json_value),
            format: self.format,
            quiet: false,
        })
    }
}
```

- [ ] **Step 3: Implement `datasets/list.rs`**

```rust
use clap::Args;
use futures::StreamExt;
use huggingface_hub::{HfApi, ListDatasetsParams};

use crate::cli::OutputFormat;
use crate::output::{CommandOutput, CommandResult};

#[derive(Args)]
#[command(about = "List datasets on the Hub")]
pub struct ListArgs {
    /// Filter by search query
    #[arg(long)]
    pub search: Option<String>,

    /// Filter by dataset author or organization
    #[arg(long)]
    pub author: Option<String>,

    /// Filter by tags. Can be specified multiple times
    #[arg(long)]
    pub filter: Vec<String>,

    /// Sort order for results
    #[arg(long)]
    pub sort: Option<String>,

    /// Maximum number of results to return
    #[arg(long, default_value = "10")]
    pub limit: usize,

    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    pub format: OutputFormat,

    /// Only print dataset IDs, one per line
    #[arg(short, long)]
    pub quiet: bool,
}

impl ListArgs {
    pub async fn execute(&self, api: &HfApi) -> anyhow::Result<CommandResult> {
        let filter_str = if self.filter.is_empty() {
            None
        } else {
            Some(self.filter.join(","))
        };

        let mut builder = ListDatasetsParams::builder().limit(self.limit);
        if let Some(ref search) = self.search {
            builder = builder.search(search);
        }
        if let Some(ref author) = self.author {
            builder = builder.author(author);
        }
        if let Some(ref filter) = filter_str {
            builder = builder.filter(filter);
        }
        if let Some(ref sort) = self.sort {
            builder = builder.sort(sort);
        }
        let params = builder.build();

        let stream = api.list_datasets(&params);
        futures::pin_mut!(stream);
        let mut datasets = Vec::new();
        while let Some(item) = stream.next().await {
            datasets.push(item?);
        }

        let json_value = serde_json::to_value(&datasets)?;

        let headers = vec![
            "ID".to_string(),
            "Author".to_string(),
            "Downloads".to_string(),
            "Likes".to_string(),
        ];

        let rows: Vec<Vec<String>> = datasets
            .iter()
            .map(|d| {
                vec![
                    d.id.clone(),
                    d.author.clone().unwrap_or_default(),
                    d.downloads.map(|v| v.to_string()).unwrap_or_default(),
                    d.likes.map(|v| v.to_string()).unwrap_or_default(),
                ]
            })
            .collect();

        let quiet_values: Vec<String> = datasets.iter().map(|d| d.id.clone()).collect();

        Ok(CommandResult::Formatted {
            output: CommandOutput {
                headers,
                rows,
                json_value,
                quiet_values,
            },
            format: self.format,
            quiet: self.quiet,
        })
    }
}
```

- [ ] **Step 4: Verify, format, lint, commit**

Run: `cargo run -p hfrs -- datasets list --limit 3 --format json`
Run: `cargo +nightly fmt && cargo clippy -p hfrs -- -D warnings`

```bash
git add hfrs/src/commands/datasets/
git commit -m "feat(hfrs): implement datasets info and list commands"
```

---

### Task 7: Spaces Commands

**Files:**
- Modify: `hfrs/src/commands/spaces/mod.rs`
- Modify: `hfrs/src/commands/spaces/info.rs`
- Modify: `hfrs/src/commands/spaces/list.rs`

Same pattern as models/datasets, using `SpaceInfoParams`, `ListSpacesParams`, `space_info()`, `list_spaces()`. Table headers: `ID`, `Author`, `SDK`, `Likes`.

- [ ] **Step 1: Implement `spaces/mod.rs`**

```rust
pub mod info;
pub mod list;

use clap::{Args, Subcommand};
use huggingface_hub::HfApi;

use crate::output::CommandResult;

#[derive(Args)]
#[command(about = "Interact with spaces on the Hub")]
pub struct SpacesCommand {
    #[command(subcommand)]
    pub command: SpacesSubcommand,
}

#[derive(Subcommand)]
pub enum SpacesSubcommand {
    /// Get info about a space
    Info(info::InfoArgs),
    /// List spaces on the Hub
    #[command(alias = "ls")]
    List(list::ListArgs),
}

impl SpacesCommand {
    pub async fn execute(&self, api: &HfApi) -> anyhow::Result<CommandResult> {
        match &self.command {
            SpacesSubcommand::Info(args) => args.execute(api).await,
            SpacesSubcommand::List(args) => args.execute(api).await,
        }
    }
}
```

- [ ] **Step 2: Implement `spaces/info.rs`**

```rust
use clap::Args;
use huggingface_hub::{HfApi, SpaceInfoParams};

use crate::cli::OutputFormat;
use crate::output::{CommandOutput, CommandResult};

#[derive(Args)]
#[command(about = "Get info about a space")]
pub struct InfoArgs {
    /// Space ID (e.g. 'gradio/hello_world')
    pub space_id: String,

    /// Revision (branch, tag, or commit hash)
    #[arg(long)]
    pub revision: Option<String>,

    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    pub format: OutputFormat,
}

impl InfoArgs {
    pub async fn execute(&self, api: &HfApi) -> anyhow::Result<CommandResult> {
        let mut builder = SpaceInfoParams::builder().repo_id(&self.space_id);
        if let Some(ref rev) = self.revision {
            builder = builder.revision(rev);
        }
        let params = builder.build();
        let info = api.space_info(&params).await?;
        let json_value = serde_json::to_value(&info)?;

        Ok(CommandResult::Formatted {
            output: CommandOutput::single_item(json_value),
            format: self.format,
            quiet: false,
        })
    }
}
```

- [ ] **Step 3: Implement `spaces/list.rs`**

```rust
use clap::Args;
use futures::StreamExt;
use huggingface_hub::{HfApi, ListSpacesParams};

use crate::cli::OutputFormat;
use crate::output::{CommandOutput, CommandResult};

#[derive(Args)]
#[command(about = "List spaces on the Hub")]
pub struct ListArgs {
    /// Filter by search query
    #[arg(long)]
    pub search: Option<String>,

    /// Filter by space author or organization
    #[arg(long)]
    pub author: Option<String>,

    /// Filter by tags. Can be specified multiple times
    #[arg(long)]
    pub filter: Vec<String>,

    /// Sort order for results
    #[arg(long)]
    pub sort: Option<String>,

    /// Maximum number of results to return
    #[arg(long, default_value = "10")]
    pub limit: usize,

    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    pub format: OutputFormat,

    /// Only print space IDs, one per line
    #[arg(short, long)]
    pub quiet: bool,
}

impl ListArgs {
    pub async fn execute(&self, api: &HfApi) -> anyhow::Result<CommandResult> {
        let filter_str = if self.filter.is_empty() {
            None
        } else {
            Some(self.filter.join(","))
        };

        let mut builder = ListSpacesParams::builder().limit(self.limit);
        if let Some(ref search) = self.search {
            builder = builder.search(search);
        }
        if let Some(ref author) = self.author {
            builder = builder.author(author);
        }
        if let Some(ref filter) = filter_str {
            builder = builder.filter(filter);
        }
        if let Some(ref sort) = self.sort {
            builder = builder.sort(sort);
        }
        let params = builder.build();

        let stream = api.list_spaces(&params);
        futures::pin_mut!(stream);
        let mut spaces = Vec::new();
        while let Some(item) = stream.next().await {
            spaces.push(item?);
        }

        let json_value = serde_json::to_value(&spaces)?;

        let headers = vec![
            "ID".to_string(),
            "Author".to_string(),
            "SDK".to_string(),
            "Likes".to_string(),
        ];

        let rows: Vec<Vec<String>> = spaces
            .iter()
            .map(|s| {
                vec![
                    s.id.clone(),
                    s.author.clone().unwrap_or_default(),
                    s.sdk.clone().unwrap_or_default(),
                    s.likes.map(|v| v.to_string()).unwrap_or_default(),
                ]
            })
            .collect();

        let quiet_values: Vec<String> = spaces.iter().map(|s| s.id.clone()).collect();

        Ok(CommandResult::Formatted {
            output: CommandOutput {
                headers,
                rows,
                json_value,
                quiet_values,
            },
            format: self.format,
            quiet: self.quiet,
        })
    }
}
```

- [ ] **Step 4: Verify, format, lint, commit**

Run: `cargo run -p hfrs -- spaces list --limit 3 --format json`
Run: `cargo +nightly fmt && cargo clippy -p hfrs -- -D warnings`

```bash
git add hfrs/src/commands/spaces/
git commit -m "feat(hfrs): implement spaces info and list commands"
```

---

### Task 8: Repos Commands

**Files:**
- Modify: `hfrs/src/commands/repos/mod.rs`
- Modify: `hfrs/src/commands/repos/create.rs`
- Modify: `hfrs/src/commands/repos/delete.rs`
- Modify: `hfrs/src/commands/repos/move_repo.rs`
- Modify: `hfrs/src/commands/repos/settings.rs`
- Modify: `hfrs/src/commands/repos/delete_files.rs`
- Modify: `hfrs/src/commands/repos/branch.rs`
- Modify: `hfrs/src/commands/repos/tag.rs`

- [ ] **Step 1: Implement `repos/mod.rs`**

```rust
pub mod branch;
pub mod create;
pub mod delete;
pub mod delete_files;
pub mod move_repo;
pub mod settings;
pub mod tag;

use clap::{Args, Subcommand};
use huggingface_hub::HfApi;

use crate::output::CommandResult;

#[derive(Args)]
#[command(about = "Manage repos on the Hub")]
pub struct ReposCommand {
    #[command(subcommand)]
    pub command: ReposSubcommand,
}

#[derive(Subcommand)]
pub enum ReposSubcommand {
    /// Create a new repository on the Hub
    Create(create::CreateArgs),
    /// Delete a repository from the Hub
    Delete(delete::DeleteArgs),
    /// Move (rename) a repository
    Move(move_repo::MoveArgs),
    /// Update repository settings (visibility, gating)
    Settings(settings::SettingsArgs),
    /// Delete files from a repository via commit
    DeleteFiles(delete_files::DeleteFilesArgs),
    /// Manage branches
    Branch(branch::BranchCommand),
    /// Manage tags
    Tag(tag::TagCommand),
}

impl ReposCommand {
    pub async fn execute(&self, api: &HfApi) -> anyhow::Result<CommandResult> {
        match &self.command {
            ReposSubcommand::Create(args) => args.execute(api).await,
            ReposSubcommand::Delete(args) => args.execute(api).await,
            ReposSubcommand::Move(args) => args.execute(api).await,
            ReposSubcommand::Settings(args) => args.execute(api).await,
            ReposSubcommand::DeleteFiles(args) => args.execute(api).await,
            ReposSubcommand::Branch(cmd) => cmd.execute(api).await,
            ReposSubcommand::Tag(cmd) => cmd.execute(api).await,
        }
    }
}
```

- [ ] **Step 2: Implement `repos/create.rs`**

```rust
use clap::Args;
use huggingface_hub::{CreateRepoParams, HfApi, RepoType};

use crate::cli::RepoTypeArg;
use crate::output::CommandResult;

#[derive(Args)]
#[command(about = "Create a new repository on the Hub")]
pub struct CreateArgs {
    /// Repository ID (e.g. 'username/repo-name')
    pub repo_id: String,

    /// Repository type
    #[arg(long, value_enum, default_value_t = RepoTypeArg::Model)]
    pub r#type: RepoTypeArg,

    /// Make the repository private
    #[arg(long)]
    pub private: bool,

    /// Do not error if the repo already exists
    #[arg(long)]
    pub exist_ok: bool,

    /// Space SDK (required when type is space)
    #[arg(long)]
    pub space_sdk: Option<String>,
}

impl CreateArgs {
    pub async fn execute(&self, api: &HfApi) -> anyhow::Result<CommandResult> {
        let repo_type: RepoType = self.r#type.into();
        let mut builder = CreateRepoParams::builder()
            .repo_id(&self.repo_id)
            .repo_type(repo_type)
            .exist_ok(self.exist_ok);

        if self.private {
            builder = builder.private(true);
        }
        if let Some(ref sdk) = self.space_sdk {
            builder = builder.space_sdk(sdk);
        }

        let params = builder.build();
        let url = api.create_repo(&params).await?;
        Ok(CommandResult::Raw(format!("Created: {}", url.url)))
    }
}
```

- [ ] **Step 3: Implement `repos/delete.rs`**

```rust
use clap::Args;
use huggingface_hub::{DeleteRepoParams, HfApi, RepoType};

use crate::cli::RepoTypeArg;
use crate::output::CommandResult;

#[derive(Args)]
#[command(about = "Delete a repository from the Hub")]
pub struct DeleteArgs {
    /// Repository ID
    pub repo_id: String,

    /// Repository type
    #[arg(long, value_enum, default_value_t = RepoTypeArg::Model)]
    pub r#type: RepoTypeArg,

    /// Do not error if the repo does not exist
    #[arg(long)]
    pub missing_ok: bool,
}

impl DeleteArgs {
    pub async fn execute(&self, api: &HfApi) -> anyhow::Result<CommandResult> {
        let repo_type: RepoType = self.r#type.into();
        let params = DeleteRepoParams::builder()
            .repo_id(&self.repo_id)
            .repo_type(repo_type)
            .missing_ok(self.missing_ok)
            .build();
        api.delete_repo(&params).await?;
        Ok(CommandResult::Silent)
    }
}
```

- [ ] **Step 4: Implement `repos/move_repo.rs`**

```rust
use clap::Args;
use huggingface_hub::{HfApi, MoveRepoParams, RepoType};

use crate::cli::RepoTypeArg;
use crate::output::CommandResult;

#[derive(Args)]
#[command(about = "Move (rename) a repository")]
pub struct MoveArgs {
    /// Source repository ID
    pub from_id: String,

    /// Destination repository ID
    pub to_id: String,

    /// Repository type
    #[arg(long, value_enum, default_value_t = RepoTypeArg::Model)]
    pub r#type: RepoTypeArg,
}

impl MoveArgs {
    pub async fn execute(&self, api: &HfApi) -> anyhow::Result<CommandResult> {
        let repo_type: RepoType = self.r#type.into();
        let params = MoveRepoParams::builder()
            .from_id(&self.from_id)
            .to_id(&self.to_id)
            .repo_type(repo_type)
            .build();
        let url = api.move_repo(&params).await?;
        Ok(CommandResult::Raw(format!("Moved to: {}", url.url)))
    }
}
```

- [ ] **Step 5: Implement `repos/settings.rs`**

```rust
use clap::Args;
use huggingface_hub::{HfApi, RepoType, UpdateRepoParams};

use crate::cli::RepoTypeArg;
use crate::output::CommandResult;

#[derive(Args)]
#[command(about = "Update repository settings (visibility, gating)")]
pub struct SettingsArgs {
    /// Repository ID
    pub repo_id: String,

    /// Set gating mode
    #[arg(long)]
    pub gated: Option<String>,

    /// Make the repository private
    #[arg(long)]
    pub private: Option<bool>,

    /// Repository type
    #[arg(long, value_enum, default_value_t = RepoTypeArg::Model)]
    pub r#type: RepoTypeArg,
}

impl SettingsArgs {
    pub async fn execute(&self, api: &HfApi) -> anyhow::Result<CommandResult> {
        let repo_type: RepoType = self.r#type.into();
        let mut builder = UpdateRepoParams::builder()
            .repo_id(&self.repo_id)
            .repo_type(repo_type);

        if let Some(ref gated) = self.gated {
            builder = builder.gated(gated);
        }
        if let Some(private) = self.private {
            builder = builder.private(private);
        }

        let params = builder.build();
        api.update_repo_settings(&params).await?;
        Ok(CommandResult::Silent)
    }
}
```

- [ ] **Step 6: Implement `repos/delete_files.rs`**

```rust
use clap::Args;
use huggingface_hub::{CommitOperation, CreateCommitParams, HfApi, RepoType};

use crate::cli::RepoTypeArg;
use crate::output::CommandResult;

#[derive(Args)]
#[command(about = "Delete files from a repository via commit")]
pub struct DeleteFilesArgs {
    /// Repository ID
    pub repo_id: String,

    /// File paths or glob patterns to delete
    #[arg(required = true)]
    pub patterns: Vec<String>,

    /// Repository type
    #[arg(long, value_enum, default_value_t = RepoTypeArg::Model)]
    pub r#type: RepoTypeArg,

    /// Revision (branch)
    #[arg(long)]
    pub revision: Option<String>,

    /// Commit message
    #[arg(long)]
    pub commit_message: Option<String>,

    /// Commit description
    #[arg(long)]
    pub commit_description: Option<String>,

    /// Create a Pull Request instead of committing directly
    #[arg(long)]
    pub create_pr: bool,
}

impl DeleteFilesArgs {
    pub async fn execute(&self, api: &HfApi) -> anyhow::Result<CommandResult> {
        let repo_type: RepoType = self.r#type.into();
        let operations: Vec<CommitOperation> = self
            .patterns
            .iter()
            .map(|p| CommitOperation::Delete {
                path_in_repo: p.clone(),
            })
            .collect();

        let message = self
            .commit_message
            .clone()
            .unwrap_or_else(|| format!("Delete files via hfrs"));

        let mut builder = CreateCommitParams::builder()
            .repo_id(&self.repo_id)
            .operations(operations)
            .commit_message(message)
            .repo_type(repo_type);

        if let Some(ref desc) = self.commit_description {
            builder = builder.commit_description(desc);
        }
        if let Some(ref rev) = self.revision {
            builder = builder.revision(rev);
        }
        if self.create_pr {
            builder = builder.create_pr(true);
        }

        let params = builder.build();
        let info = api.create_commit(&params).await?;
        if let Some(url) = &info.commit_url {
            Ok(CommandResult::Raw(format!("Committed: {url}")))
        } else {
            Ok(CommandResult::Silent)
        }
    }
}
```

- [ ] **Step 7: Implement `repos/branch.rs`**

```rust
use clap::{Args, Subcommand};
use huggingface_hub::{CreateBranchParams, DeleteBranchParams, HfApi, RepoType};

use crate::cli::RepoTypeArg;
use crate::output::CommandResult;

#[derive(Args)]
#[command(about = "Manage branches")]
pub struct BranchCommand {
    #[command(subcommand)]
    pub command: BranchSubcommand,
}

#[derive(Subcommand)]
pub enum BranchSubcommand {
    /// Create a new branch
    Create(CreateBranchArgs),
    /// Delete a branch
    Delete(DeleteBranchArgs),
}

impl BranchCommand {
    pub async fn execute(&self, api: &HfApi) -> anyhow::Result<CommandResult> {
        match &self.command {
            BranchSubcommand::Create(args) => args.execute(api).await,
            BranchSubcommand::Delete(args) => args.execute(api).await,
        }
    }
}

#[derive(Args)]
#[command(about = "Create a new branch")]
pub struct CreateBranchArgs {
    /// Repository ID
    pub repo_id: String,

    /// Branch name
    pub branch: String,

    /// Source revision to branch from
    #[arg(long)]
    pub revision: Option<String>,

    /// Repository type
    #[arg(long, value_enum, default_value_t = RepoTypeArg::Model)]
    pub r#type: RepoTypeArg,
}

impl CreateBranchArgs {
    pub async fn execute(&self, api: &HfApi) -> anyhow::Result<CommandResult> {
        let repo_type: RepoType = self.r#type.into();
        let mut builder = CreateBranchParams::builder()
            .repo_id(&self.repo_id)
            .branch(&self.branch)
            .repo_type(repo_type);
        if let Some(ref rev) = self.revision {
            builder = builder.revision(rev);
        }
        let params = builder.build();
        api.create_branch(&params).await?;
        Ok(CommandResult::Raw(format!(
            "Branch '{}' created.",
            self.branch
        )))
    }
}

#[derive(Args)]
#[command(about = "Delete a branch")]
pub struct DeleteBranchArgs {
    /// Repository ID
    pub repo_id: String,

    /// Branch name
    pub branch: String,

    /// Repository type
    #[arg(long, value_enum, default_value_t = RepoTypeArg::Model)]
    pub r#type: RepoTypeArg,
}

impl DeleteBranchArgs {
    pub async fn execute(&self, api: &HfApi) -> anyhow::Result<CommandResult> {
        let repo_type: RepoType = self.r#type.into();
        let params = DeleteBranchParams::builder()
            .repo_id(&self.repo_id)
            .branch(&self.branch)
            .repo_type(repo_type)
            .build();
        api.delete_branch(&params).await?;
        Ok(CommandResult::Silent)
    }
}
```

- [ ] **Step 8: Implement `repos/tag.rs`**

```rust
use clap::{Args, Subcommand};
use huggingface_hub::{
    CreateTagParams, DeleteTagParams, HfApi, ListRepoRefsParams, RepoType,
};

use crate::cli::{OutputFormat, RepoTypeArg};
use crate::output::{CommandOutput, CommandResult};

#[derive(Args)]
#[command(about = "Manage tags")]
pub struct TagCommand {
    #[command(subcommand)]
    pub command: TagSubcommand,
}

#[derive(Subcommand)]
pub enum TagSubcommand {
    /// Create a new tag
    Create(CreateTagArgs),
    /// Delete a tag
    Delete(DeleteTagArgs),
    /// List tags
    #[command(alias = "ls")]
    List(ListTagArgs),
}

impl TagCommand {
    pub async fn execute(&self, api: &HfApi) -> anyhow::Result<CommandResult> {
        match &self.command {
            TagSubcommand::Create(args) => args.execute(api).await,
            TagSubcommand::Delete(args) => args.execute(api).await,
            TagSubcommand::List(args) => args.execute(api).await,
        }
    }
}

#[derive(Args)]
#[command(about = "Create a new tag")]
pub struct CreateTagArgs {
    /// Repository ID
    pub repo_id: String,

    /// Tag name
    pub tag: String,

    /// Tag message
    #[arg(short, long)]
    pub message: Option<String>,

    /// Source revision to tag
    #[arg(long)]
    pub revision: Option<String>,

    /// Repository type
    #[arg(long, value_enum, default_value_t = RepoTypeArg::Model)]
    pub r#type: RepoTypeArg,
}

impl CreateTagArgs {
    pub async fn execute(&self, api: &HfApi) -> anyhow::Result<CommandResult> {
        let repo_type: RepoType = self.r#type.into();
        let mut builder = CreateTagParams::builder()
            .repo_id(&self.repo_id)
            .tag(&self.tag)
            .repo_type(repo_type);
        if let Some(ref msg) = self.message {
            builder = builder.message(msg);
        }
        if let Some(ref rev) = self.revision {
            builder = builder.revision(rev);
        }
        let params = builder.build();
        api.create_tag(&params).await?;
        Ok(CommandResult::Raw(format!("Tag '{}' created.", self.tag)))
    }
}

#[derive(Args)]
#[command(about = "Delete a tag")]
pub struct DeleteTagArgs {
    /// Repository ID
    pub repo_id: String,

    /// Tag name
    pub tag: String,

    /// Repository type
    #[arg(long, value_enum, default_value_t = RepoTypeArg::Model)]
    pub r#type: RepoTypeArg,
}

impl DeleteTagArgs {
    pub async fn execute(&self, api: &HfApi) -> anyhow::Result<CommandResult> {
        let repo_type: RepoType = self.r#type.into();
        let params = DeleteTagParams::builder()
            .repo_id(&self.repo_id)
            .tag(&self.tag)
            .repo_type(repo_type)
            .build();
        api.delete_tag(&params).await?;
        Ok(CommandResult::Silent)
    }
}

#[derive(Args)]
#[command(about = "List tags for a repository")]
pub struct ListTagArgs {
    /// Repository ID
    pub repo_id: String,

    /// Repository type
    #[arg(long, value_enum, default_value_t = RepoTypeArg::Model)]
    pub r#type: RepoTypeArg,

    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    pub format: OutputFormat,
}

impl ListTagArgs {
    pub async fn execute(&self, api: &HfApi) -> anyhow::Result<CommandResult> {
        let repo_type: RepoType = self.r#type.into();
        let params = ListRepoRefsParams::builder()
            .repo_id(&self.repo_id)
            .repo_type(repo_type)
            .build();
        let refs = api.list_repo_refs(&params).await?;

        let json_value = serde_json::to_value(&refs.tags)?;

        let headers = vec![
            "Name".to_string(),
            "Ref".to_string(),
            "Commit".to_string(),
        ];
        let rows: Vec<Vec<String>> = refs
            .tags
            .iter()
            .map(|t| {
                vec![
                    t.name.clone(),
                    t.git_ref.clone(),
                    t.target_commit.clone(),
                ]
            })
            .collect();
        let quiet_values: Vec<String> = refs.tags.iter().map(|t| t.name.clone()).collect();

        Ok(CommandResult::Formatted {
            output: CommandOutput {
                headers,
                rows,
                json_value,
                quiet_values,
            },
            format: self.format,
            quiet: false,
        })
    }
}
```

- [ ] **Step 9: Verify, format, lint, commit**

Run: `cargo build -p hfrs`
Run: `cargo run -p hfrs -- repos --help`
Run: `cargo +nightly fmt && cargo clippy -p hfrs -- -D warnings`

```bash
git add hfrs/src/commands/repos/
git commit -m "feat(hfrs): implement repos commands (create, delete, move, settings, delete-files, branch, tag)"
```

---

### Task 9: Download + Upload Commands

**Files:**
- Modify: `hfrs/src/commands/download.rs`
- Modify: `hfrs/src/commands/upload.rs`

- [ ] **Step 1: Implement `download.rs`**

```rust
use std::path::PathBuf;

use clap::Args;
use huggingface_hub::{DownloadFileParams, HfApi, RepoType, SnapshotDownloadParams};

use crate::cli::RepoTypeArg;
use crate::output::CommandResult;

#[derive(Args)]
#[command(about = "Download files from the Hub")]
pub struct DownloadArgs {
    /// Repository ID (e.g. 'meta-llama/Llama-3.2-1B-Instruct')
    pub repo_id: String,

    /// Specific files to download (if empty, downloads entire repo)
    pub filenames: Vec<String>,

    /// Repository type
    #[arg(long, value_enum, default_value_t = RepoTypeArg::Model)]
    pub r#type: RepoTypeArg,

    /// Revision (branch, tag, or commit hash)
    #[arg(long)]
    pub revision: Option<String>,

    /// Glob patterns to include
    #[arg(long)]
    pub include: Vec<String>,

    /// Glob patterns to exclude
    #[arg(long)]
    pub exclude: Vec<String>,

    /// Cache directory
    #[arg(long)]
    pub cache_dir: Option<PathBuf>,

    /// Download to a local directory instead of cache
    #[arg(long)]
    pub local_dir: Option<PathBuf>,

    /// Force re-download even if files are cached
    #[arg(long)]
    pub force_download: bool,

    /// Suppress output
    #[arg(long)]
    pub quiet: bool,
}

impl DownloadArgs {
    pub async fn execute(&self, api: &HfApi) -> anyhow::Result<CommandResult> {
        let repo_type: RepoType = self.r#type.into();

        if self.filenames.len() == 1 && self.include.is_empty() && self.exclude.is_empty() {
            // Single file download
            let mut builder = DownloadFileParams::builder()
                .repo_id(&self.repo_id)
                .filename(&self.filenames[0])
                .repo_type(repo_type);
            if let Some(ref rev) = self.revision {
                builder = builder.revision(rev);
            }
            if let Some(ref local_dir) = self.local_dir {
                builder = builder.local_dir(local_dir.clone());
            }
            if self.force_download {
                builder = builder.force_download(true);
            }
            let params = builder.build();
            let path = api.download_file(&params).await?;
            if self.quiet {
                Ok(CommandResult::Silent)
            } else {
                Ok(CommandResult::Raw(path.display().to_string()))
            }
        } else {
            // Snapshot download (multiple files or patterns)
            let allow = if !self.filenames.is_empty() {
                Some(self.filenames.clone())
            } else if !self.include.is_empty() {
                Some(self.include.clone())
            } else {
                None
            };
            let ignore = if !self.exclude.is_empty() {
                Some(self.exclude.clone())
            } else {
                None
            };

            let mut builder = SnapshotDownloadParams::builder()
                .repo_id(&self.repo_id)
                .repo_type(repo_type);
            if let Some(ref rev) = self.revision {
                builder = builder.revision(rev);
            }
            if let Some(allow_patterns) = allow {
                builder = builder.allow_patterns(allow_patterns);
            }
            if let Some(ignore_patterns) = ignore {
                builder = builder.ignore_patterns(ignore_patterns);
            }
            if let Some(ref local_dir) = self.local_dir {
                builder = builder.local_dir(local_dir.clone());
            }
            if self.force_download {
                builder = builder.force_download(true);
            }
            let params = builder.build();
            let path = api.snapshot_download(&params).await?;
            if self.quiet {
                Ok(CommandResult::Silent)
            } else {
                Ok(CommandResult::Raw(path.display().to_string()))
            }
        }
    }
}
```

- [ ] **Step 2: Implement `upload.rs`**

```rust
use std::path::PathBuf;

use clap::Args;
use huggingface_hub::{HfApi, RepoType, UploadFileParams, UploadFolderParams, AddSource};

use crate::cli::RepoTypeArg;
use crate::output::CommandResult;

#[derive(Args)]
#[command(about = "Upload a file or folder to the Hub")]
pub struct UploadArgs {
    /// Repository ID
    pub repo_id: String,

    /// Local path to upload (file or directory)
    pub local_path: Option<PathBuf>,

    /// Path in the repository to upload to
    pub path_in_repo: Option<String>,

    /// Repository type
    #[arg(long, value_enum, default_value_t = RepoTypeArg::Model)]
    pub r#type: RepoTypeArg,

    /// Revision (branch)
    #[arg(long)]
    pub revision: Option<String>,

    /// Make the repo private (if it needs to be created)
    #[arg(long)]
    pub private: bool,

    /// Glob patterns to include
    #[arg(long)]
    pub include: Vec<String>,

    /// Glob patterns to exclude
    #[arg(long)]
    pub exclude: Vec<String>,

    /// Glob patterns for files to delete from repo while committing
    #[arg(long)]
    pub delete: Vec<String>,

    /// Commit message
    #[arg(long)]
    pub commit_message: Option<String>,

    /// Commit description
    #[arg(long)]
    pub commit_description: Option<String>,

    /// Create a Pull Request instead of committing directly
    #[arg(long)]
    pub create_pr: bool,

    /// Suppress output
    #[arg(long)]
    pub quiet: bool,
}

impl UploadArgs {
    pub async fn execute(&self, api: &HfApi) -> anyhow::Result<CommandResult> {
        let repo_type: RepoType = self.r#type.into();
        let local_path = self
            .local_path
            .clone()
            .unwrap_or_else(|| PathBuf::from("."));

        if local_path.is_file() {
            let path_in_repo = self
                .path_in_repo
                .clone()
                .unwrap_or_else(|| {
                    local_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string()
                });

            let mut builder = UploadFileParams::builder()
                .repo_id(&self.repo_id)
                .source(AddSource::File(local_path.clone()))
                .path_in_repo(path_in_repo)
                .repo_type(repo_type);
            if let Some(ref rev) = self.revision {
                builder = builder.revision(rev);
            }
            if let Some(ref msg) = self.commit_message {
                builder = builder.commit_message(msg);
            }
            if let Some(ref desc) = self.commit_description {
                builder = builder.commit_description(desc);
            }
            if self.create_pr {
                builder = builder.create_pr(true);
            }
            let params = builder.build();
            let info = api.upload_file(&params).await?;
            if self.quiet {
                Ok(CommandResult::Silent)
            } else {
                Ok(CommandResult::Raw(
                    info.commit_url.unwrap_or_else(|| "Uploaded.".to_string()),
                ))
            }
        } else if local_path.is_dir() {
            let mut builder = UploadFolderParams::builder()
                .repo_id(&self.repo_id)
                .folder_path(local_path.clone())
                .repo_type(repo_type);
            if let Some(ref pir) = self.path_in_repo {
                builder = builder.path_in_repo(pir);
            }
            if let Some(ref rev) = self.revision {
                builder = builder.revision(rev);
            }
            if let Some(ref msg) = self.commit_message {
                builder = builder.commit_message(msg);
            }
            if let Some(ref desc) = self.commit_description {
                builder = builder.commit_description(desc);
            }
            if self.create_pr {
                builder = builder.create_pr(true);
            }
            if !self.include.is_empty() {
                builder = builder.allow_patterns(self.include.clone());
            }
            if !self.exclude.is_empty() {
                builder = builder.ignore_patterns(self.exclude.clone());
            }
            if !self.delete.is_empty() {
                builder = builder.delete_patterns(self.delete.clone());
            }
            let params = builder.build();
            let info = api.upload_folder(&params).await?;
            if self.quiet {
                Ok(CommandResult::Silent)
            } else {
                Ok(CommandResult::Raw(
                    info.commit_url.unwrap_or_else(|| "Uploaded.".to_string()),
                ))
            }
        } else {
            anyhow::bail!(
                "Path '{}' does not exist or is not a file/directory.",
                local_path.display()
            );
        }
    }
}
```

- [ ] **Step 3: Verify, format, lint, commit**

Run: `cargo build -p hfrs`
Run: `cargo run -p hfrs -- download --help`
Run: `cargo run -p hfrs -- upload --help`
Run: `cargo +nightly fmt && cargo clippy -p hfrs -- -D warnings`

```bash
git add hfrs/src/commands/download.rs hfrs/src/commands/upload.rs
git commit -m "feat(hfrs): implement download and upload commands"
```

---

### Task 10: Discussions Commands

**Files:**
- Modify: all files in `hfrs/src/commands/discussions/`

- [ ] **Step 1: Implement `discussions/mod.rs`**

```rust
pub mod close;
pub mod comment;
pub mod create;
pub mod diff;
pub mod info;
pub mod list;
pub mod merge;
pub mod rename;
pub mod reopen;

use clap::{Args, Subcommand};
use huggingface_hub::HfApi;

use crate::output::CommandResult;

#[derive(Args)]
#[command(about = "Manage discussions and pull requests on the Hub")]
pub struct DiscussionsCommand {
    #[command(subcommand)]
    pub command: DiscussionsSubcommand,
}

#[derive(Subcommand)]
pub enum DiscussionsSubcommand {
    /// List discussions for a repository
    #[command(alias = "ls")]
    List(list::ListArgs),
    /// Get details about a discussion
    Info(info::InfoArgs),
    /// Create a new discussion or pull request
    Create(create::CreateArgs),
    /// Add a comment to a discussion
    Comment(comment::CommentArgs),
    /// Merge a pull request
    Merge(merge::MergeArgs),
    /// Close a discussion
    Close(close::CloseArgs),
    /// Reopen a closed discussion
    Reopen(reopen::ReopenArgs),
    /// Rename a discussion
    Rename(rename::RenameArgs),
    /// Show diff for a pull request
    Diff(diff::DiffArgs),
}

impl DiscussionsCommand {
    pub async fn execute(&self, api: &HfApi) -> anyhow::Result<CommandResult> {
        match &self.command {
            DiscussionsSubcommand::List(args) => args.execute(api).await,
            DiscussionsSubcommand::Info(args) => args.execute(api).await,
            DiscussionsSubcommand::Create(args) => args.execute(api).await,
            DiscussionsSubcommand::Comment(args) => args.execute(api).await,
            DiscussionsSubcommand::Merge(args) => args.execute(api).await,
            DiscussionsSubcommand::Close(args) => args.execute(api).await,
            DiscussionsSubcommand::Reopen(args) => args.execute(api).await,
            DiscussionsSubcommand::Rename(args) => args.execute(api).await,
            DiscussionsSubcommand::Diff(args) => args.execute(api).await,
        }
    }
}
```

- [ ] **Step 2: Implement all discussion leaf commands**

For each leaf, implement the clap Args struct with appropriate fields matching the `hf` CLI flags, and an `execute` method that maps to the corresponding `HfApi` method. Key mappings:

- `list.rs`: `GetRepoDiscussionsParams` → `api.get_repo_discussions()`, display as table with Num/Title/Status/Author columns
- `info.rs`: `GetDiscussionDetailsParams` → `api.get_discussion_details()`, display as single item JSON
- `create.rs`: `CreateDiscussionParams` or `CreatePullRequestParams` (based on `--pull-request` flag) → `api.create_discussion()` or `api.create_pull_request()`
- `comment.rs`: `CommentDiscussionParams` → `api.comment_discussion()`
- `merge.rs`: `MergePullRequestParams` → `api.merge_pull_request()`
- `close.rs`: `ChangeDiscussionStatusParams` with `new_status("closed")` → `api.change_discussion_status()`
- `reopen.rs`: `ChangeDiscussionStatusParams` with `new_status("open")` → `api.change_discussion_status()`
- `rename.rs`: `RenameDiscussionParams` → `api.rename_discussion()`
- `diff.rs`: `GetDiscussionDetailsParams` → `api.get_discussion_details()`, extract `diff` field, return as `CommandResult::Raw`

Each file follows the established leaf command pattern. Example for `discussions/list.rs`:

```rust
use clap::Args;
use huggingface_hub::{GetRepoDiscussionsParams, HfApi, RepoType};

use crate::cli::{OutputFormat, RepoTypeArg};
use crate::output::{CommandOutput, CommandResult};

#[derive(Args)]
#[command(about = "List discussions for a repository")]
pub struct ListArgs {
    /// Repository ID
    pub repo_id: String,

    /// Filter by status
    #[arg(short, long)]
    pub status: Option<String>,

    /// Filter by kind (discussion or pull_request)
    #[arg(short, long)]
    pub kind: Option<String>,

    /// Filter by author
    #[arg(long)]
    pub author: Option<String>,

    /// Repository type
    #[arg(long, value_enum, default_value_t = RepoTypeArg::Model)]
    pub r#type: RepoTypeArg,

    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    pub format: OutputFormat,

    /// Only print discussion numbers, one per line
    #[arg(short, long)]
    pub quiet: bool,
}

impl ListArgs {
    pub async fn execute(&self, api: &HfApi) -> anyhow::Result<CommandResult> {
        let repo_type: RepoType = self.r#type.into();
        let mut builder = GetRepoDiscussionsParams::builder()
            .repo_id(&self.repo_id)
            .repo_type(repo_type);
        if let Some(ref author) = self.author {
            builder = builder.author(author);
        }
        if let Some(ref kind) = self.kind {
            builder = builder.discussion_type(kind);
        }
        if let Some(ref status) = self.status {
            builder = builder.discussion_status(status);
        }
        let params = builder.build();
        let response = api.get_repo_discussions(&params).await?;

        let json_value = serde_json::to_value(&response.discussions)?;

        let headers = vec![
            "Num".to_string(),
            "Title".to_string(),
            "Status".to_string(),
            "PR".to_string(),
        ];
        let rows: Vec<Vec<String>> = response
            .discussions
            .iter()
            .map(|d| {
                vec![
                    d.num.to_string(),
                    d.title.clone().unwrap_or_default(),
                    d.status.clone().unwrap_or_default(),
                    d.is_pull_request
                        .map(|b| if b { "yes" } else { "no" })
                        .unwrap_or("")
                        .to_string(),
                ]
            })
            .collect();
        let quiet_values: Vec<String> =
            response.discussions.iter().map(|d| d.num.to_string()).collect();

        Ok(CommandResult::Formatted {
            output: CommandOutput {
                headers,
                rows,
                json_value,
                quiet_values,
            },
            format: self.format,
            quiet: self.quiet,
        })
    }
}
```

Example for `discussions/close.rs`:

```rust
use clap::Args;
use huggingface_hub::{ChangeDiscussionStatusParams, HfApi, RepoType};

use crate::cli::RepoTypeArg;
use crate::output::CommandResult;

#[derive(Args)]
#[command(about = "Close a discussion")]
pub struct CloseArgs {
    /// Repository ID
    pub repo_id: String,

    /// Discussion number
    pub num: u64,

    /// Repository type
    #[arg(long, value_enum, default_value_t = RepoTypeArg::Model)]
    pub r#type: RepoTypeArg,
}

impl CloseArgs {
    pub async fn execute(&self, api: &HfApi) -> anyhow::Result<CommandResult> {
        let repo_type: RepoType = self.r#type.into();
        let params = ChangeDiscussionStatusParams::builder()
            .repo_id(&self.repo_id)
            .discussion_num(self.num)
            .new_status("closed")
            .repo_type(repo_type)
            .build();
        api.change_discussion_status(&params).await?;
        Ok(CommandResult::Silent)
    }
}
```

Example for `discussions/diff.rs`:

```rust
use clap::Args;
use huggingface_hub::{GetDiscussionDetailsParams, HfApi, RepoType};

use crate::cli::RepoTypeArg;
use crate::output::CommandResult;

#[derive(Args)]
#[command(about = "Show diff for a pull request")]
pub struct DiffArgs {
    /// Repository ID
    pub repo_id: String,

    /// Discussion/PR number
    pub num: u64,

    /// Repository type
    #[arg(long, value_enum, default_value_t = RepoTypeArg::Model)]
    pub r#type: RepoTypeArg,
}

impl DiffArgs {
    pub async fn execute(&self, api: &HfApi) -> anyhow::Result<CommandResult> {
        let repo_type: RepoType = self.r#type.into();
        let params = GetDiscussionDetailsParams::builder()
            .repo_id(&self.repo_id)
            .discussion_num(self.num)
            .repo_type(repo_type)
            .build();
        let details = api.get_discussion_details(&params).await?;
        let diff = details.diff.unwrap_or_else(|| "No diff available.".to_string());
        Ok(CommandResult::Raw(diff))
    }
}
```

Implement the remaining leaf files (`info.rs`, `create.rs`, `comment.rs`, `merge.rs`, `reopen.rs`, `rename.rs`) following the same patterns. Each maps 1:1 to an `HfApi` method.

- [ ] **Step 3: Verify, format, lint, commit**

Run: `cargo build -p hfrs`
Run: `cargo run -p hfrs -- discussions --help`
Run: `cargo +nightly fmt && cargo clippy -p hfrs -- -D warnings`

```bash
git add hfrs/src/commands/discussions/
git commit -m "feat(hfrs): implement discussions commands"
```

---

### Task 11: Collections Commands

**Files:**
- Modify: all files in `hfrs/src/commands/collections/`

- [ ] **Step 1: Implement all collections files**

`collections/mod.rs` with subcommands: Info, List, Create, Delete, Update, AddItem, UpdateItem, DeleteItem. Each leaf maps to its corresponding `HfApi` method:

- `info.rs`: `GetCollectionParams` → `api.get_collection()` → single item display
- `list.rs`: `ListCollectionsParams` → `api.list_collections()` → table with Slug/Title/Items/Upvotes
- `create.rs`: `CreateCollectionParams` → `api.create_collection()` → show slug
- `delete.rs`: `DeleteCollectionParams` → `api.delete_collection()` → Silent
- `update.rs`: `UpdateCollectionMetadataParams` → `api.update_collection_metadata()` → Silent
- `add_item.rs`: `AddCollectionItemParams` → `api.add_collection_item()` → show slug
- `update_item.rs`: `UpdateCollectionItemParams` → `api.update_collection_item()` → Silent
- `delete_item.rs`: `DeleteCollectionItemParams` → `api.delete_collection_item()` → Silent

Follow the same patterns established in Tasks 5 and 10.

- [ ] **Step 2: Verify, format, lint, commit**

```bash
cargo build -p hfrs && cargo +nightly fmt && cargo clippy -p hfrs -- -D warnings
git add hfrs/src/commands/collections/
git commit -m "feat(hfrs): implement collections commands"
```

---

### Task 12: Webhooks Commands

**Files:**
- Modify: all files in `hfrs/src/commands/webhooks/`

- [ ] **Step 1: Implement all webhooks files**

Subcommands: List, Info, Create, Update, Delete, Enable, Disable.

- `list.rs`: `api.list_webhooks()` → table with ID/URL/Domains
- `info.rs`: `api.get_webhook(webhook_id)` → single item
- `create.rs`: `CreateWebhookParams` → `api.create_webhook()` → show ID
- `update.rs`: `UpdateWebhookParams` → `api.update_webhook()` → show ID
- `delete.rs`: `api.delete_webhook(webhook_id)` → Silent
- `enable.rs`: `api.enable_webhook(webhook_id)` → Silent
- `disable.rs`: `api.disable_webhook(webhook_id)` → Silent

- [ ] **Step 2: Verify, format, lint, commit**

```bash
cargo build -p hfrs && cargo +nightly fmt && cargo clippy -p hfrs -- -D warnings
git add hfrs/src/commands/webhooks/
git commit -m "feat(hfrs): implement webhooks commands"
```

---

### Task 13: Inference Endpoints Commands

**Files:**
- Modify: all files in `hfrs/src/commands/endpoints/`

- [ ] **Step 1: Implement all endpoints files**

Subcommands: List, Describe, Deploy, Delete, Pause, Resume, ScaleToZero, Update.

- `list.rs`: `ListInferenceEndpointsParams` → `api.list_inference_endpoints()` → table with Name/Status/URL
- `describe.rs`: `GetInferenceEndpointParams` → `api.get_inference_endpoint()` → single item
- `deploy.rs`: `CreateInferenceEndpointParams` (all required fields: name, repository, framework, accelerator, instance_size, instance_type, region, vendor) → `api.create_inference_endpoint()`
- `delete.rs`: `DeleteInferenceEndpointParams` → `api.delete_inference_endpoint()` → Silent
- `pause.rs`: `PauseInferenceEndpointParams` → `api.pause_inference_endpoint()`
- `resume.rs`: `ResumeInferenceEndpointParams` → `api.resume_inference_endpoint()`
- `scale_to_zero.rs`: `ScaleToZeroInferenceEndpointParams` → `api.scale_to_zero_inference_endpoint()`
- `update.rs`: `UpdateInferenceEndpointParams` (all optional fields) → `api.update_inference_endpoint()`

- [ ] **Step 2: Verify, format, lint, commit**

```bash
cargo build -p hfrs && cargo +nightly fmt && cargo clippy -p hfrs -- -D warnings
git add hfrs/src/commands/endpoints/
git commit -m "feat(hfrs): implement inference endpoints commands"
```

---

### Task 14: Jobs Commands

**Files:**
- Modify: all files in `hfrs/src/commands/jobs/` and `hfrs/src/commands/jobs/scheduled/`

- [ ] **Step 1: Implement jobs leaf commands**

- `run.rs`: `RunJobParams` (image, command as positional args; --flavor, --env, --secrets, --timeout, --namespace) → `api.run_job()`
- `ps.rs`: `ListJobsParams` → `api.list_jobs()` → table with ID/Image/Status/Created
- `inspect.rs`: positional `job_id` → `api.inspect_job(job_id, namespace)` → single item
- `cancel.rs`: positional `job_id` → `api.cancel_job(job_id, namespace)` → Silent
- `logs.rs`: positional `job_id` → `api.fetch_job_logs(job_id, namespace)` → Raw text (each log entry as timestamp + data)
- `hardware.rs`: `api.list_job_hardware()` → table with Name/CPU/RAM
- `stats.rs`: positional `job_id` → `api.fetch_job_metrics(job_id, namespace)` → single item

- [ ] **Step 2: Implement scheduled jobs leaf commands**

- `scheduled/run.rs`: `CreateScheduledJobParams` (schedule, image, command as positional; --flavor, --env, --suspend, --concurrency, etc.) → `api.create_scheduled_job()`
- `scheduled/ps.rs`: `api.list_scheduled_jobs()` → table with ID/Schedule/Image/Suspended
- `scheduled/inspect.rs`: `api.inspect_scheduled_job(id)` → single item
- `scheduled/delete.rs`: `api.delete_scheduled_job(id)` → Silent
- `scheduled/suspend.rs`: `api.suspend_scheduled_job(id)` → Silent
- `scheduled/resume.rs`: `api.resume_scheduled_job(id)` → Silent

- [ ] **Step 3: Verify, format, lint, commit**

```bash
cargo build -p hfrs && cargo +nightly fmt && cargo clippy -p hfrs -- -D warnings
git add hfrs/src/commands/jobs/
git commit -m "feat(hfrs): implement jobs and scheduled jobs commands"
```

---

### Task 15: Papers Commands

**Files:**
- Modify: all files in `hfrs/src/commands/papers/`

- [ ] **Step 1: Implement papers files**

- `info.rs`: `PaperInfoParams` → `api.paper_info()` → single item
- `list.rs`: `ListDailyPapersParams` (--date, --week, --month, --submitter, --sort, --limit) → `api.list_daily_papers()` → table with Title/ID/Upvotes
- `search.rs`: `ListPapersParams` (query as positional, --limit) → `api.list_papers()` → table with Title/ID

- [ ] **Step 2: Verify, format, lint, commit**

```bash
cargo build -p hfrs && cargo +nightly fmt && cargo clippy -p hfrs -- -D warnings
git add hfrs/src/commands/papers/
git commit -m "feat(hfrs): implement papers commands"
```

---

### Task 16: Likes Commands

**Files:**
- Modify: all files in `hfrs/src/commands/likes/`

- [ ] **Step 1: Implement likes files**

- `like.rs`: `LikeParams` (repo_id, --type) → `api.like()` → Silent
- `unlike.rs`: `LikeParams` (repo_id, --type) → `api.unlike()` → Silent
- `list.rs`: `ListLikedReposParams` (username) → `api.list_liked_repos()` → table with Repo/Type/Created

- [ ] **Step 2: Verify, format, lint, commit**

```bash
cargo build -p hfrs && cargo +nightly fmt && cargo clippy -p hfrs -- -D warnings
git add hfrs/src/commands/likes/
git commit -m "feat(hfrs): implement likes commands"
```

---

### Task 17: Access Requests Commands

**Files:**
- Modify: all files in `hfrs/src/commands/access_requests/`

- [ ] **Step 1: Implement access_requests files**

- `list.rs`: Takes `repo_id`, `--status` (pending/accepted/rejected), `--type`. Routes to `list_pending_access_requests`, `list_accepted_access_requests`, or `list_rejected_access_requests` based on status. Table: Username/Email/Status/Timestamp
- `accept.rs`: `HandleAccessRequestParams` (repo_id, user) → `api.accept_access_request()` → Silent
- `reject.rs`: `HandleAccessRequestParams` → `api.reject_access_request()` → Silent
- `cancel.rs`: `HandleAccessRequestParams` → `api.cancel_access_request()` → Silent
- `grant.rs`: `GrantAccessParams` (repo_id, user) → `api.grant_access()` → Silent

- [ ] **Step 2: Verify, format, lint, commit**

```bash
cargo build -p hfrs && cargo +nightly fmt && cargo clippy -p hfrs -- -D warnings
git add hfrs/src/commands/access_requests/
git commit -m "feat(hfrs): implement access requests commands"
```

---

### Task 18: Cache Commands

**Files:**
- Modify: `hfrs/src/commands/cache/mod.rs`
- Modify: `hfrs/src/commands/cache/list.rs`
- Modify: `hfrs/src/commands/cache/rm.rs`

- [ ] **Step 1: Implement cache commands**

`cache/mod.rs`: Subcommands List and Rm.

`cache/list.rs`: Call `api.scan_cache()` (check the exact method name — it may be `scan_cache_dir` or accessed via `HfCacheInfo`). Display table with Repo/Type/Size/Revisions/Last Accessed.

`cache/rm.rs`: Takes target repo IDs or revision hashes as positional args. Call `api.delete_cache_revisions()` or the equivalent method. Check the API for `DeleteCacheRevision` type.

Note: The cache API methods may differ from other API patterns since they operate on local filesystem. Read `huggingface_hub/src/api/cache.rs` during implementation to get exact signatures.

- [ ] **Step 2: Verify, format, lint, commit**

```bash
cargo build -p hfrs && cargo +nightly fmt && cargo clippy -p hfrs -- -D warnings
git add hfrs/src/commands/cache/
git commit -m "feat(hfrs): implement cache list and rm commands"
```

---

### Task 19: Integration Test Harness

**Files:**
- Create: `hfrs/tests/helpers/mod.rs`
- Create: `hfrs/tests/cli_comparison.rs`

- [ ] **Step 1: Implement `tests/helpers/mod.rs`**

```rust
use std::process::Command;

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

    pub fn run_json(&self, args: &[&str]) -> anyhow::Result<serde_json::Value> {
        let bin = self.bin_path.as_deref().unwrap_or(&self.bin);
        let mut cmd = Command::new(bin);
        cmd.args(args);
        cmd.arg("--format").arg("json");
        if let Some(ref token) = self.token {
            cmd.arg("--token").arg(token);
        }
        let output = cmd.output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(
                "{} {:?} failed (exit {}): {}",
                self.bin,
                args,
                output.status,
                stderr
            );
        }
        let stdout = String::from_utf8(output.stdout)?;
        let value: serde_json::Value = serde_json::from_str(&stdout)?;
        Ok(value)
    }
}

pub fn require_cli(runner: &CliRunner) {
    if !runner.is_available() {
        panic!(
            "Required CLI '{}' not found on PATH. Install it before running integration tests.",
            runner.bin
        );
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

pub fn assert_json_equivalent(
    actual: &serde_json::Value,
    expected: &serde_json::Value,
    ignore_fields: &[&str],
) {
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
                    }
                    None => {
                        panic!("Missing key at '{current_path}': expected {e_val}");
                    }
                }
            }
        }
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
        }
        _ => {
            assert_eq!(
                actual, expected,
                "Value mismatch at '{path}': actual {actual} vs expected {expected}"
            );
        }
    }
}
```

- [ ] **Step 2: Implement `tests/cli_comparison.rs`**

```rust
mod helpers;

use helpers::{assert_json_equivalent, require_cli, require_token, CliRunner, VOLATILE_FIELDS};

#[test]
fn models_list_matches_hf() {
    require_token();
    let hfrs = CliRunner::hfrs();
    let hf = CliRunner::new("hf");
    require_cli(&hf);

    let hfrs_out = hfrs
        .run_json(&["models", "list", "--limit", "3"])
        .unwrap();
    let hf_out = hf.run_json(&["models", "list", "--limit", "3"]).unwrap();

    // Both should be arrays
    assert!(hfrs_out.is_array(), "hfrs output should be an array");
    assert!(hf_out.is_array(), "hf output should be an array");
    assert_eq!(
        hfrs_out.as_array().unwrap().len(),
        hf_out.as_array().unwrap().len(),
        "Should return same number of models"
    );
}

#[test]
fn models_info_matches_hf() {
    require_token();
    let hfrs = CliRunner::hfrs();
    let hf = CliRunner::new("hf");
    require_cli(&hf);

    let hfrs_out = hfrs
        .run_json(&["models", "info", "gpt2"])
        .unwrap();
    let hf_out = hf.run_json(&["models", "info", "gpt2"]).unwrap();

    assert_json_equivalent(&hfrs_out, &hf_out, VOLATILE_FIELDS);
}

#[test]
fn datasets_list_matches_hf() {
    require_token();
    let hfrs = CliRunner::hfrs();
    let hf = CliRunner::new("hf");
    require_cli(&hf);

    let hfrs_out = hfrs
        .run_json(&["datasets", "list", "--limit", "3"])
        .unwrap();
    let hf_out = hf
        .run_json(&["datasets", "list", "--limit", "3"])
        .unwrap();

    assert!(hfrs_out.is_array());
    assert!(hf_out.is_array());
}

#[test]
fn spaces_list_matches_hf() {
    require_token();
    let hfrs = CliRunner::hfrs();
    let hf = CliRunner::new("hf");
    require_cli(&hf);

    let hfrs_out = hfrs
        .run_json(&["spaces", "list", "--limit", "3"])
        .unwrap();
    let hf_out = hf
        .run_json(&["spaces", "list", "--limit", "3"])
        .unwrap();

    assert!(hfrs_out.is_array());
    assert!(hf_out.is_array());
}

#[test]
fn models_info_matches_hfjs() {
    require_token();
    let hfrs = CliRunner::hfrs();
    let hfjs = CliRunner::new("hfjs");
    require_cli(&hfjs);

    let hfrs_out = hfrs
        .run_json(&["models", "info", "gpt2"])
        .unwrap();
    let hfjs_out = hfjs
        .run_json(&["models", "info", "gpt2"])
        .unwrap();

    assert_json_equivalent(&hfrs_out, &hfjs_out, VOLATILE_FIELDS);
}

#[test]
fn version_runs() {
    let hfrs = CliRunner::hfrs();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_hfrs"))
        .arg("version")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.starts_with("hfrs "));
}

#[test]
fn env_runs() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_hfrs"))
        .arg("env")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("hfrs version:"));
}
```

- [ ] **Step 3: Verify tests compile**

Run: `cargo test -p hfrs --test cli_comparison --no-run`
Expected: Compiles

- [ ] **Step 4: Run basic tests**

Run: `HF_TOKEN=$HF_TOKEN cargo test -p hfrs --test cli_comparison -- version_runs env_runs`
Expected: Both pass

- [ ] **Step 5: Format and lint**

Run: `cargo +nightly fmt && cargo clippy -p hfrs -- -D warnings`

- [ ] **Step 6: Commit**

```bash
git add hfrs/tests/
git commit -m "feat(hfrs): add integration test harness with CLI comparison tests"
```

---

### Task 20: README

**Files:**
- Create: `hfrs/README.md`

- [ ] **Step 1: Write README**

Include:
- Brief description
- Installation: `cargo install --path hfrs`
- Usage examples (download, upload, models list)
- Full command coverage matrix (implemented commands table)
- Skipped commands table with reasons
- Token configuration (env var, `hfrs auth login`, file)
- Running tests: `HF_TOKEN=... cargo test -p hfrs --test cli_comparison`
- Link to the design spec

- [ ] **Step 2: Commit**

```bash
git add hfrs/README.md
git commit -m "docs(hfrs): add README with coverage matrix and usage guide"
```

---

### Task 21: Final Verification

- [ ] **Step 1: Full build**

Run: `cargo build -p hfrs`
Expected: Compiles with no errors

- [ ] **Step 2: Full lint**

Run: `cargo +nightly fmt --check && cargo clippy -p hfrs -- -D warnings`
Expected: No issues

- [ ] **Step 3: Run all tests**

Run: `cargo test -p huggingface-hub` (existing tests still pass)
Run: `HF_TOKEN=$HF_TOKEN cargo test -p hfrs --test cli_comparison`
Expected: All pass

- [ ] **Step 4: Smoke test key commands**

Run: `cargo run -p hfrs -- models list --limit 2`
Run: `cargo run -p hfrs -- models info gpt2 --format json`
Run: `cargo run -p hfrs -- datasets list --limit 2`
Run: `cargo run -p hfrs -- version`
Run: `cargo run -p hfrs -- env`
Expected: All produce sensible output

- [ ] **Step 5: Commit any final fixes**

```bash
git add -A
git commit -m "chore(hfrs): final cleanup and verification"
```
