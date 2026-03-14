# hf-hub Rust Library Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an async Rust client library for the Hugging Face Hub API covering repo info, file operations, commits/diffs, user operations, downloads, and uploads (with xet support behind a feature flag).

**Architecture:** Single crate (`hf-hub`) in a cargo workspace. `HfApi` wraps `Arc<HfApiInner>` holding a `reqwest::Client`. All methods are async. Paginated endpoints return `impl Stream`. Param structs use `typed-builder`. A unified `HfError` enum handles all errors.

**Tech Stack:** Rust, reqwest, tokio, serde, thiserror, typed-builder, futures, url, globset

**Spec:** `docs/superpowers/specs/2026-03-13-hf-hub-rust-design.md`

---

## File Map

| File | Responsibility |
|------|---------------|
| `Cargo.toml` | Workspace root |
| `hf_hub/Cargo.toml` | Package manifest with deps and features |
| `hf_hub/src/lib.rs` | Public re-exports |
| `hf_hub/src/constants.rs` | Env var names, default URLs, repo type URL prefixes |
| `hf_hub/src/error.rs` | `HfError` enum, `Result<T>` alias, response-to-error mapping helper |
| `hf_hub/src/client.rs` | `HfApi`, `HfApiBuilder`, `HfApiInner`, auth header helpers, response checking |
| `hf_hub/src/pagination.rs` | Generic `paginate<T>()` using `futures::stream::try_unfold`, Link header parsing |
| `hf_hub/src/types/mod.rs` | Module declarations, common re-exports |
| `hf_hub/src/types/repo.rs` | `RepoType`, `ModelInfo`, `DatasetInfo`, `SpaceInfo`, `RepoTreeEntry`, `RepoSibling`, `BlobLfsInfo`, `LastCommitInfo`, `RepoUrl` |
| `hf_hub/src/types/user.rs` | `User`, `Organization` |
| `hf_hub/src/types/commit.rs` | `CommitInfo`, `GitCommitInfo`, `GitRefInfo`, `GitRefs`, `CommitAuthor`, `DiffEntry`, `CommitOperation`, `AddSource` |
| `hf_hub/src/types/params.rs` | All `*Params` structs with `TypedBuilder` |
| `hf_hub/src/api/mod.rs` | Module declarations |
| `hf_hub/src/api/repo.rs` | `model_info`, `dataset_info`, `space_info`, `repo_exists`, `revision_exists`, `file_exists`, `list_models`, `list_datasets`, `list_spaces`, `create_repo`, `delete_repo`, `update_repo_settings`, `move_repo` |
| `hf_hub/src/api/files.rs` | `list_repo_files`, `list_repo_tree`, `get_paths_info`, `download_file`, `upload_file`, `upload_folder`, `delete_file`, `delete_folder`, `create_commit` |
| `hf_hub/src/api/commits.rs` | `list_repo_commits`, `list_repo_refs`, `get_commit_diff`, `get_raw_diff`, `create_branch`, `delete_branch`, `create_tag`, `delete_tag` |
| `hf_hub/src/api/users.rs` | `whoami`, `auth_check`, `get_user_overview`, `get_organization_overview`, `list_user_followers`, `list_user_following`, `list_organization_members` |
| `hf_hub/src/xet.rs` | `XetConnectionInfo`, xet download/upload behind `#[cfg(feature = "xet")]` |

---

## Chunk 1: Project Scaffolding, Error Types, Constants, Core Types

### Task 1: Workspace and Crate Scaffolding

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `hf_hub/Cargo.toml` (package manifest)
- Create: `hf_hub/src/lib.rs` (crate root)

- [ ] **Step 1: Create workspace root Cargo.toml**

```toml
[workspace]
members = ["hf_hub"]
resolver = "2"
```

- [ ] **Step 2: Create hf_hub/Cargo.toml**

```toml
[package]
name = "hf-hub"
version = "0.1.0"
edition = "2021"
description = "Rust client for the Hugging Face Hub API"
license = "Apache-2.0"

[dependencies]
reqwest = { version = "0.12", features = ["json", "stream", "multipart"] }
tokio = { version = "1", features = ["fs", "io-util"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
url = "2"
futures = "0.3"
typed-builder = "0.20"
globset = "0.4"

[dev-dependencies]
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
tempfile = "3"
```

Note: the `xet` feature and its dependency will be added in Task 13 (Chunk 7) once we implement xet integration. For now we omit it to avoid build issues with a git dependency we haven't validated yet.

- [ ] **Step 3: Create hf_hub/src/lib.rs with module declarations**

```rust
pub mod constants;
pub mod error;
pub mod types;
pub mod client;
pub mod pagination;
pub mod api;

pub use client::{HfApi, HfApiBuilder};
pub use error::{HfError, Result};
pub use types::*;
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check` from workspace root
Expected: Compilation errors (modules don't exist yet) — that's fine, we'll fix them in the next steps.

### Task 2: Constants

**Files:**
- Create: `hf_hub/src/constants.rs`

- [ ] **Step 1: Write constants**

```rust
/// Default Hugging Face Hub endpoint
pub const DEFAULT_HF_ENDPOINT: &str = "https://huggingface.co";

/// Default revision (branch)
pub const DEFAULT_REVISION: &str = "main";

// Environment variable names
pub const HF_ENDPOINT: &str = "HF_ENDPOINT";
pub const HF_TOKEN: &str = "HF_TOKEN";
pub const HF_TOKEN_PATH: &str = "HF_TOKEN_PATH";
pub const HF_HOME: &str = "HF_HOME";
pub const HF_HUB_CACHE: &str = "HF_HUB_CACHE";
pub const HF_HUB_DISABLE_IMPLICIT_TOKEN: &str = "HF_HUB_DISABLE_IMPLICIT_TOKEN";
pub const HF_HUB_USER_AGENT_ORIGIN: &str = "HF_HUB_USER_AGENT_ORIGIN";

/// Default HF home directory
pub const DEFAULT_HF_HOME: &str = "~/.cache/huggingface";

/// Token filename within HF_HOME
pub const TOKEN_FILENAME: &str = "token";

/// URL prefixes for different repo types
/// Models have no prefix, datasets use "datasets/", spaces use "spaces/"
pub fn repo_type_url_prefix(repo_type: Option<crate::types::repo::RepoType>) -> &'static str {
    match repo_type {
        None | Some(crate::types::repo::RepoType::Model) => "",
        Some(crate::types::repo::RepoType::Dataset) => "datasets/",
        Some(crate::types::repo::RepoType::Space) => "spaces/",
    }
}

/// API path segment for repo types: "models", "datasets", "spaces"
pub fn repo_type_api_segment(repo_type: Option<crate::types::repo::RepoType>) -> &'static str {
    match repo_type {
        None | Some(crate::types::repo::RepoType::Model) => "models",
        Some(crate::types::repo::RepoType::Dataset) => "datasets",
        Some(crate::types::repo::RepoType::Space) => "spaces",
    }
}
```

### Task 3: Error Types

**Files:**
- Create: `hf_hub/src/error.rs`

- [ ] **Step 1: Write error types**

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HfError {
    #[error("HTTP error: {status} {url}")]
    Http {
        status: reqwest::StatusCode,
        url: String,
        body: String,
    },

    #[error("Authentication required")]
    AuthRequired,

    #[error("Repository not found: {repo_id}")]
    RepoNotFound { repo_id: String },

    #[error("Revision not found: {revision} in {repo_id}")]
    RevisionNotFound { repo_id: String, revision: String },

    #[error("Entry not found: {path} in {repo_id}")]
    EntryNotFound { path: String, repo_id: String },

    #[error("Xet feature required but not enabled")]
    XetNotEnabled,

    #[error(transparent)]
    Request(#[from] reqwest::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Url(#[from] url::ParseError),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, HfError>;

/// Context for mapping HTTP 404 errors to specific HfError variants.
pub(crate) enum NotFoundContext {
    /// 404 means the repository does not exist
    Repo,
    /// 404 means a file/path does not exist within the repo
    Entry { path: String },
    /// 404 means the revision does not exist
    Revision { revision: String },
    /// No special mapping — use generic Http error
    Generic,
}
```

### Task 4: Core Types — Repo

**Files:**
- Create: `hf_hub/src/types/mod.rs`
- Create: `hf_hub/src/types/repo.rs`

- [ ] **Step 1: Create types/mod.rs**

```rust
pub mod repo;
pub mod user;
pub mod commit;
pub mod params;

pub use repo::*;
pub use user::*;
pub use commit::*;
pub use params::*;
```

- [ ] **Step 2: Write repo types**

```rust
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RepoType {
    Model,
    Dataset,
    Space,
}

impl fmt::Display for RepoType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RepoType::Model => write!(f, "model"),
            RepoType::Dataset => write!(f, "dataset"),
            RepoType::Space => write!(f, "space"),
        }
    }
}

impl FromStr for RepoType {
    type Err = crate::error::HfError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "model" => Ok(RepoType::Model),
            "dataset" => Ok(RepoType::Dataset),
            "space" => Ok(RepoType::Space),
            _ => Err(crate::error::HfError::Other(format!("Unknown repo type: {s}"))),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct BlobLfsInfo {
    pub size: Option<u64>,
    pub sha256: Option<String>,
    pub pointer_size: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LastCommitInfo {
    pub id: Option<String>,
    pub title: Option<String>,
    pub date: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RepoSibling {
    pub rfilename: String,
    pub size: Option<u64>,
    pub lfs: Option<BlobLfsInfo>,
}

/// Tagged union for tree entries returned by list_repo_tree
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum RepoTreeEntry {
    File {
        oid: String,
        size: u64,
        path: String,
        lfs: Option<BlobLfsInfo>,
        #[serde(default, rename = "lastCommit")]
        last_commit: Option<LastCommitInfo>,
    },
    Directory {
        oid: String,
        path: String,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    #[serde(rename = "_id")]
    pub mongo_id: Option<String>,
    pub model_id: Option<String>,
    pub author: Option<String>,
    pub sha: Option<String>,
    pub private: Option<bool>,
    pub gated: Option<serde_json::Value>,
    pub disabled: Option<bool>,
    pub downloads: Option<u64>,
    pub downloads_all_time: Option<u64>,
    pub likes: Option<u64>,
    pub tags: Option<Vec<String>>,
    pub pipeline_tag: Option<String>,
    pub library_name: Option<String>,
    pub created_at: Option<String>,
    pub last_modified: Option<String>,
    pub siblings: Option<Vec<RepoSibling>>,
    pub card_data: Option<serde_json::Value>,
    pub config: Option<serde_json::Value>,
    pub trending_score: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetInfo {
    pub id: String,
    #[serde(rename = "_id")]
    pub mongo_id: Option<String>,
    pub author: Option<String>,
    pub sha: Option<String>,
    pub private: Option<bool>,
    pub gated: Option<serde_json::Value>,
    pub disabled: Option<bool>,
    pub downloads: Option<u64>,
    pub downloads_all_time: Option<u64>,
    pub likes: Option<u64>,
    pub tags: Option<Vec<String>>,
    pub created_at: Option<String>,
    pub last_modified: Option<String>,
    pub siblings: Option<Vec<RepoSibling>>,
    pub card_data: Option<serde_json::Value>,
    pub trending_score: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpaceInfo {
    pub id: String,
    #[serde(rename = "_id")]
    pub mongo_id: Option<String>,
    pub author: Option<String>,
    pub sha: Option<String>,
    pub private: Option<bool>,
    pub gated: Option<serde_json::Value>,
    pub disabled: Option<bool>,
    pub likes: Option<u64>,
    pub tags: Option<Vec<String>>,
    pub created_at: Option<String>,
    pub last_modified: Option<String>,
    pub siblings: Option<Vec<RepoSibling>>,
    pub card_data: Option<serde_json::Value>,
    pub sdk: Option<String>,
    pub trending_score: Option<f64>,
}

/// URL returned by create_repo/move_repo
#[derive(Debug, Clone, Deserialize)]
pub struct RepoUrl {
    pub url: String,
}
```

### Task 5: Core Types — User & Commit

**Files:**
- Create: `hf_hub/src/types/user.rs`
- Create: `hf_hub/src/types/commit.rs`

- [ ] **Step 1: Write user types**

```rust
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    #[serde(alias = "login", alias = "user")]
    pub username: String,
    #[serde(alias = "name")]
    pub fullname: Option<String>,
    pub avatar_url: Option<String>,
    #[serde(rename = "type")]
    pub user_type: Option<String>,
    pub is_pro: Option<bool>,
    pub email: Option<String>,
    pub email_verified: Option<bool>,
    pub plan: Option<String>,
    pub can_pay: Option<bool>,
    pub orgs: Option<Vec<OrgMembership>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrgMembership {
    pub name: Option<String>,
    pub fullname: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Organization {
    pub name: String,
    pub fullname: Option<String>,
    pub avatar_url: Option<String>,
    #[serde(rename = "type")]
    pub org_type: Option<String>,
}
```

- [ ] **Step 2: Write commit types**

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct CommitAuthor {
    pub user: Option<String>,
    pub name: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GitCommitInfo {
    pub id: String,
    pub authors: Vec<CommitAuthor>,
    pub date: Option<String>,
    pub title: String,
    pub message: String,
    #[serde(default)]
    pub parents: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitRefInfo {
    pub name: String,
    #[serde(rename = "ref")]
    pub git_ref: String,
    pub target_commit: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitRefs {
    pub branches: Vec<GitRefInfo>,
    pub tags: Vec<GitRefInfo>,
    #[serde(default)]
    pub converts: Vec<GitRefInfo>,
    #[serde(default, rename = "pullRequests")]
    pub pull_requests: Vec<GitRefInfo>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitInfo {
    pub commit_url: Option<String>,
    pub commit_message: Option<String>,
    pub commit_description: Option<String>,
    pub oid: Option<String>,
    pub pr_url: Option<String>,
    pub pr_num: Option<u64>,
}

/// A single entry in a commit diff
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffEntry {
    pub path: Option<String>,
    pub old_path: Option<String>,
    pub status: Option<String>,
}

/// Describes a file mutation in a commit
#[derive(Debug, Clone)]
pub enum CommitOperation {
    /// Upload a file (from path or bytes)
    Add {
        path_in_repo: String,
        source: AddSource,
    },
    /// Delete a file or folder
    Delete {
        path_in_repo: String,
    },
}

/// Source of content for an add operation
#[derive(Debug, Clone)]
pub enum AddSource {
    File(PathBuf),
    Bytes(Vec<u8>),
}
```

### Task 6: Params Structs

**Files:**
- Create: `hf_hub/src/types/params.rs`

- [ ] **Step 1: Write all params structs**

This file contains all `*Params` structs. Each uses `#[derive(TypedBuilder)]` with `#[builder(default, setter(into, strip_option))]` on optional fields.

```rust
use std::path::PathBuf;
use typed_builder::TypedBuilder;
use super::commit::{AddSource, CommitOperation};
use super::repo::RepoType;

// --- Repo Info ---

#[derive(TypedBuilder)]
pub struct ModelInfoParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
}

#[derive(TypedBuilder)]
pub struct DatasetInfoParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
}

#[derive(TypedBuilder)]
pub struct SpaceInfoParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
}

// --- Existence Checks ---

#[derive(TypedBuilder)]
pub struct RepoExistsParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[derive(TypedBuilder)]
pub struct RevisionExistsParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub revision: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[derive(TypedBuilder)]
pub struct FileExistsParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub filename: String,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

// --- Repo Listing ---

#[derive(TypedBuilder)]
pub struct ListModelsParams {
    #[builder(default, setter(into, strip_option))]
    pub search: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub author: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub filter: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub sort: Option<String>,
    #[builder(default, setter(strip_option))]
    pub limit: Option<usize>,
    #[builder(default, setter(into, strip_option))]
    pub pipeline_tag: Option<String>,
    #[builder(default, setter(strip_option))]
    pub full: Option<bool>,
    #[builder(default, setter(strip_option))]
    pub card_data: Option<bool>,
    #[builder(default, setter(strip_option))]
    pub fetch_config: Option<bool>,
}

#[derive(TypedBuilder)]
pub struct ListDatasetsParams {
    #[builder(default, setter(into, strip_option))]
    pub search: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub author: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub filter: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub sort: Option<String>,
    #[builder(default, setter(strip_option))]
    pub limit: Option<usize>,
    #[builder(default, setter(strip_option))]
    pub full: Option<bool>,
}

#[derive(TypedBuilder)]
pub struct ListSpacesParams {
    #[builder(default, setter(into, strip_option))]
    pub search: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub author: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub filter: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub sort: Option<String>,
    #[builder(default, setter(strip_option))]
    pub limit: Option<usize>,
    #[builder(default, setter(strip_option))]
    pub full: Option<bool>,
}

// --- Repo Management ---

#[derive(TypedBuilder)]
pub struct CreateRepoParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
    #[builder(default, setter(strip_option))]
    pub private: Option<bool>,
    #[builder(default)]
    pub exist_ok: bool,
    #[builder(default, setter(into, strip_option))]
    pub space_sdk: Option<String>,
}

#[derive(TypedBuilder)]
pub struct DeleteRepoParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
    #[builder(default)]
    pub missing_ok: bool,
}

#[derive(TypedBuilder)]
pub struct UpdateRepoParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
    #[builder(default, setter(strip_option))]
    pub private: Option<bool>,
    #[builder(default, setter(into, strip_option))]
    pub gated: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub description: Option<String>,
}

#[derive(TypedBuilder)]
pub struct MoveRepoParams {
    #[builder(setter(into))]
    pub from_id: String,
    #[builder(setter(into))]
    pub to_id: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

// --- File Operations ---

#[derive(TypedBuilder)]
pub struct ListRepoFilesParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[derive(TypedBuilder)]
pub struct ListRepoTreeParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
    #[builder(default)]
    pub recursive: bool,
    #[builder(default)]
    pub expand: bool,
}

#[derive(TypedBuilder)]
pub struct GetPathsInfoParams {
    #[builder(setter(into))]
    pub repo_id: String,
    pub paths: Vec<String>,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[derive(TypedBuilder)]
pub struct DownloadFileParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub filename: String,
    pub local_dir: PathBuf,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
}

#[derive(TypedBuilder)]
pub struct UploadFileParams {
    #[builder(setter(into))]
    pub repo_id: String,
    pub source: AddSource,
    #[builder(setter(into))]
    pub path_in_repo: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub commit_message: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub commit_description: Option<String>,
    #[builder(default, setter(strip_option))]
    pub create_pr: Option<bool>,
    #[builder(default, setter(into, strip_option))]
    pub parent_commit: Option<String>,
}

#[derive(TypedBuilder)]
pub struct UploadFolderParams {
    #[builder(setter(into))]
    pub repo_id: String,
    pub folder_path: PathBuf,
    #[builder(default, setter(into, strip_option))]
    pub path_in_repo: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub commit_message: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub commit_description: Option<String>,
    #[builder(default, setter(strip_option))]
    pub create_pr: Option<bool>,
    #[builder(default, setter(into, strip_option))]
    pub allow_patterns: Option<Vec<String>>,
    #[builder(default, setter(into, strip_option))]
    pub ignore_patterns: Option<Vec<String>>,
    #[builder(default, setter(into, strip_option))]
    pub delete_patterns: Option<Vec<String>>,
}

#[derive(TypedBuilder)]
pub struct DeleteFileParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub path_in_repo: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub commit_message: Option<String>,
    #[builder(default, setter(strip_option))]
    pub create_pr: Option<bool>,
}

#[derive(TypedBuilder)]
pub struct DeleteFolderParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub path_in_repo: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub commit_message: Option<String>,
    #[builder(default, setter(strip_option))]
    pub create_pr: Option<bool>,
}

#[derive(TypedBuilder)]
pub struct CreateCommitParams {
    #[builder(setter(into))]
    pub repo_id: String,
    pub operations: Vec<CommitOperation>,
    #[builder(setter(into))]
    pub commit_message: String,
    #[builder(default, setter(into, strip_option))]
    pub commit_description: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(strip_option))]
    pub create_pr: Option<bool>,
    #[builder(default, setter(into, strip_option))]
    pub parent_commit: Option<String>,
}

// --- Commits & Diffs ---

#[derive(TypedBuilder)]
pub struct ListRepoCommitsParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[derive(TypedBuilder)]
pub struct ListRepoRefsParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
    #[builder(default)]
    pub include_pull_requests: bool,
}

#[derive(TypedBuilder)]
pub struct GetCommitDiffParams {
    #[builder(setter(into))]
    pub repo_id: String,
    /// Revision range in the format "revA...revB"
    #[builder(setter(into))]
    pub compare: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[derive(TypedBuilder)]
pub struct GetRawDiffParams {
    #[builder(setter(into))]
    pub repo_id: String,
    /// Revision range in the format "revA...revB"
    #[builder(setter(into))]
    pub compare: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

// --- Branch/Tag Operations ---

#[derive(TypedBuilder)]
pub struct CreateBranchParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub branch: String,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[derive(TypedBuilder)]
pub struct DeleteBranchParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub branch: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[derive(TypedBuilder)]
pub struct CreateTagParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub tag: String,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub message: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[derive(TypedBuilder)]
pub struct DeleteTagParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub tag: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}
```

### Task 7: Verify Chunk 1 Compiles

- [ ] **Step 1: Create stub files for remaining modules**

Create `hf_hub/src/client.rs`:
```rust
// Stub — implemented in Chunk 2
```

Create `hf_hub/src/pagination.rs`:
```rust
// Stub — implemented in Chunk 2
```

Create `hf_hub/src/api/mod.rs`:
```rust
pub mod repo;
pub mod files;
pub mod commits;
pub mod users;
```

Create `hf_hub/src/api/repo.rs`, `hf_hub/src/api/files.rs`, `hf_hub/src/api/commits.rs`, `hf_hub/src/api/users.rs` as empty files.

Create `hf_hub/src/xet.rs` as empty file.

- [ ] **Step 2: Run cargo check**

Run: `cargo check`
Expected: PASS (compiles with no errors)

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "feat: scaffold workspace, error types, constants, and core types"
```

---

## Chunk 2: Client, Pagination, Response Helpers

### Task 8: HfApiBuilder and HfApi

**Files:**
- Create: `hf_hub/src/client.rs`

- [ ] **Step 1: Implement client**

```rust
use std::sync::Arc;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};
use crate::constants;
use crate::error::{HfError, Result};

pub struct HfApi {
    pub(crate) inner: Arc<HfApiInner>,
}

impl Clone for HfApi {
    fn clone(&self) -> Self {
        HfApi {
            inner: Arc::clone(&self.inner),
        }
    }
}

pub(crate) struct HfApiInner {
    pub(crate) client: reqwest::Client,
    pub(crate) endpoint: String,
    pub(crate) token: Option<String>,
}

pub struct HfApiBuilder {
    endpoint: Option<String>,
    token: Option<String>,
    user_agent: Option<String>,
    headers: Option<HeaderMap>,
    client: Option<reqwest::Client>,
}

impl HfApiBuilder {
    pub fn new() -> Self {
        Self {
            endpoint: None,
            token: None,
            user_agent: None,
            headers: None,
            client: None,
        }
    }

    pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    pub fn headers(mut self, headers: HeaderMap) -> Self {
        self.headers = Some(headers);
        self
    }

    /// Provide a pre-configured reqwest::Client. Note: caller is responsible
    /// for setting User-Agent and other default headers on this client.
    pub fn client(mut self, client: reqwest::Client) -> Self {
        self.client = Some(client);
        self
    }

    pub fn build(self) -> Result<HfApi> {
        let endpoint = self.endpoint
            .or_else(|| std::env::var(constants::HF_ENDPOINT).ok())
            .unwrap_or_else(|| constants::DEFAULT_HF_ENDPOINT.to_string());

        // Validate endpoint URL early to avoid panics later
        let _ = url::Url::parse(&endpoint)?;

        let token = self.token.or_else(|| resolve_token());

        let mut default_headers = self.headers.unwrap_or_default();

        // Set User-Agent
        let user_agent = self.user_agent.unwrap_or_else(|| {
            let ua_origin = std::env::var(constants::HF_HUB_USER_AGENT_ORIGIN).ok();
            match ua_origin {
                Some(origin) => format!("hf-hub-rust/0.1.0; {origin}"),
                None => "hf-hub-rust/0.1.0".to_string(),
            }
        });
        default_headers.insert(
            USER_AGENT,
            HeaderValue::from_str(&user_agent)
                .map_err(|e| HfError::Other(format!("Invalid user agent: {e}")))?,
        );

        let client = match self.client {
            Some(c) => c,
            None => reqwest::Client::builder()
                .default_headers(default_headers)
                .build()?,
        };

        Ok(HfApi {
            inner: Arc::new(HfApiInner {
                client,
                endpoint: endpoint.trim_end_matches('/').to_string(),
                token,
            }),
        })
    }
}

impl Default for HfApiBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl HfApi {
    pub fn new() -> Result<Self> {
        HfApiBuilder::new().build()
    }

    pub fn builder() -> HfApiBuilder {
        HfApiBuilder::new()
    }

    /// Build authorization headers for requests
    pub(crate) fn auth_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Some(ref token) = self.inner.token {
            if let Ok(val) = HeaderValue::from_str(&format!("Bearer {token}")) {
                headers.insert(AUTHORIZATION, val);
            }
        }
        headers
    }

    /// Build a URL for the API: {endpoint}/api/{segment}/{repo_id}
    pub(crate) fn api_url(
        &self,
        repo_type: Option<crate::types::RepoType>,
        repo_id: &str,
    ) -> String {
        let segment = constants::repo_type_api_segment(repo_type);
        format!("{}/api/{}/{}", self.inner.endpoint, segment, repo_id)
    }

    /// Build a download URL: {endpoint}/{prefix}{repo_id}/resolve/{revision}/{filename}
    pub(crate) fn download_url(
        &self,
        repo_type: Option<crate::types::RepoType>,
        repo_id: &str,
        revision: &str,
        filename: &str,
    ) -> String {
        let prefix = constants::repo_type_url_prefix(repo_type);
        format!(
            "{}/{}{}/resolve/{}/{}",
            self.inner.endpoint, prefix, repo_id, revision, filename
        )
    }

    /// Check an HTTP response and map error status codes to HfError variants.
    /// Returns the response on success (2xx).
    ///
    /// `repo_id` and `not_found_ctx` control how 404s are mapped:
    /// - `NotFoundContext::Repo` → `HfError::RepoNotFound`
    /// - `NotFoundContext::Entry { path }` → `HfError::EntryNotFound`
    /// - `NotFoundContext::Revision { revision }` → `HfError::RevisionNotFound`
    /// - `NotFoundContext::Generic` → `HfError::Http`
    pub(crate) async fn check_response(
        &self,
        response: reqwest::Response,
        repo_id: Option<&str>,
        not_found_ctx: crate::error::NotFoundContext,
    ) -> Result<reqwest::Response> {
        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }

        let url = response.url().to_string();
        let body = response.text().await.unwrap_or_default();
        let repo_id_str = repo_id.unwrap_or("").to_string();

        match status.as_u16() {
            401 => Err(HfError::AuthRequired),
            404 => match not_found_ctx {
                crate::error::NotFoundContext::Repo => {
                    Err(HfError::RepoNotFound { repo_id: repo_id_str })
                }
                crate::error::NotFoundContext::Entry { path } => {
                    Err(HfError::EntryNotFound { path, repo_id: repo_id_str })
                }
                crate::error::NotFoundContext::Revision { revision } => {
                    Err(HfError::RevisionNotFound { revision, repo_id: repo_id_str })
                }
                crate::error::NotFoundContext::Generic => {
                    Err(HfError::Http { status, url, body })
                }
            },
            _ => Err(HfError::Http { status, url, body }),
        }
    }
}

/// Resolve token from environment or token file
fn resolve_token() -> Option<String> {
    // Check if implicit token is disabled (any non-empty value disables it)
    if let Ok(val) = std::env::var(constants::HF_HUB_DISABLE_IMPLICIT_TOKEN) {
        if !val.is_empty() {
            return None;
        }
    }

    // 1. HF_TOKEN env var
    if let Ok(token) = std::env::var(constants::HF_TOKEN) {
        if !token.is_empty() {
            return Some(token);
        }
    }

    // 2. HF_TOKEN_PATH env var
    if let Ok(path) = std::env::var(constants::HF_TOKEN_PATH) {
        if let Ok(token) = std::fs::read_to_string(&path) {
            let token = token.trim().to_string();
            if !token.is_empty() {
                return Some(token);
            }
        }
    }

    // 3. Default token file: $HF_HOME/token
    let hf_home = std::env::var(constants::HF_HOME)
        .unwrap_or_else(|_| {
            let home = dirs_or_home();
            format!("{home}/.cache/huggingface")
        });
    let token_path = format!("{hf_home}/{}", constants::TOKEN_FILENAME);
    if let Ok(token) = std::fs::read_to_string(&token_path) {
        let token = token.trim().to_string();
        if !token.is_empty() {
            return Some(token);
        }
    }

    None
}

fn dirs_or_home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string())
}
```

### Task 9: Pagination Helper

**Files:**
- Create: `hf_hub/src/pagination.rs`

- [ ] **Step 1: Implement pagination with Link header parsing**

```rust
use std::collections::VecDeque;
use futures::stream::{self, Stream};
use reqwest::header::HeaderMap;
use serde::de::DeserializeOwned;
use url::Url;
use crate::client::HfApi;
use crate::error::{HfError, Result};

struct PaginationState {
    buffer: VecDeque<serde_json::Value>,
    next_url: Option<Url>,
    is_first_page: bool,
    done: bool,
}

impl HfApi {
    /// Create a paginated stream from an initial URL and query params.
    /// Query params are only sent on the first request; subsequent pages
    /// use the full URL from the Link header.
    pub(crate) fn paginate<T: DeserializeOwned + 'static>(
        &self,
        initial_url: Url,
        params: Vec<(String, String)>,
    ) -> impl Stream<Item = Result<T>> + '_ {
        let state = PaginationState {
            buffer: VecDeque::new(),
            next_url: Some(initial_url),
            is_first_page: true,
            done: false,
        };

        stream::try_unfold(state, move |mut state| {
            let params = params.clone();
            async move {
                // Drain buffer first
                if let Some(raw) = state.buffer.pop_front() {
                    let item: T = serde_json::from_value(raw)?;
                    return Ok(Some((item, state)));
                }

                if state.done {
                    return Ok(None);
                }

                let url = match state.next_url.take() {
                    Some(u) => u,
                    None => {
                        state.done = true;
                        return Ok(None);
                    }
                };

                let mut request = self.inner.client.get(url.clone())
                    .headers(self.auth_headers());
                if state.is_first_page {
                    request = request.query(&params);
                    state.is_first_page = false;
                }

                let response = request.send().await?;

                if !response.status().is_success() {
                    let status = response.status();
                    let resp_url = response.url().to_string();
                    let body = response.text().await.unwrap_or_default();
                    return Err(HfError::Http {
                        status,
                        url: resp_url,
                        body,
                    });
                }

                state.next_url = parse_link_header_next(response.headers());
                if state.next_url.is_none() {
                    state.done = true;
                }

                let items: Vec<serde_json::Value> = response.json().await?;
                state.buffer = VecDeque::from(items);

                match state.buffer.pop_front() {
                    Some(raw) => {
                        let item: T = serde_json::from_value(raw)?;
                        Ok(Some((item, state)))
                    }
                    None => Ok(None),
                }
            }
        })
    }
}

/// Parse the `Link` header for a `rel="next"` URL.
/// Format: `<https://huggingface.co/api/models?p=1>; rel="next"`
fn parse_link_header_next(headers: &HeaderMap) -> Option<Url> {
    let link_header = headers.get("link")?.to_str().ok()?;

    for part in link_header.split(',') {
        let part = part.trim();
        // Check if this segment has rel="next"
        if !part.contains("rel=\"next\"") {
            continue;
        }
        // Extract URL between < and >
        let start = part.find('<')? + 1;
        let end = part.find('>')?;
        let url_str = &part[start..end];
        return Url::parse(url_str).ok();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{HeaderMap, HeaderValue};

    #[test]
    fn test_parse_link_header_next() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "link",
            HeaderValue::from_static(
                r#"<https://huggingface.co/api/models?p=1>; rel="next""#,
            ),
        );
        let url = parse_link_header_next(&headers).unwrap();
        assert_eq!(url.as_str(), "https://huggingface.co/api/models?p=1");
    }

    #[test]
    fn test_parse_link_header_no_next() {
        let headers = HeaderMap::new();
        assert!(parse_link_header_next(&headers).is_none());
    }

    #[test]
    fn test_parse_link_header_multiple_rels() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "link",
            HeaderValue::from_static(
                r#"<https://example.com/prev>; rel="prev", <https://example.com/next>; rel="next""#,
            ),
        );
        let url = parse_link_header_next(&headers).unwrap();
        assert_eq!(url.as_str(), "https://example.com/next");
    }
}
```

### Task 10: Verify Chunk 2 Compiles + Run Tests

- [ ] **Step 1: Run cargo check**

Run: `cargo check`
Expected: PASS

- [ ] **Step 2: Run pagination unit tests**

Run: `cargo test -p hf-hub pagination`
Expected: 3 tests pass

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "feat: implement HfApi client, builder, and pagination"
```

---

## Chunk 3: Repo Info, Listing, and Existence Checks

### Task 11: Repo Info Methods

**Files:**
- Modify: `hf_hub/src/api/repo.rs`

- [ ] **Step 1: Implement repo info and existence check methods**

```rust
use futures::Stream;
use url::Url;
use crate::client::HfApi;
use crate::constants;
use crate::error::{HfError, Result};
use crate::types::*;

impl HfApi {
    /// Get info about a model repository.
    /// Endpoint: GET /api/models/{repo_id} or /api/models/{repo_id}/revision/{revision}
    pub async fn model_info(&self, params: &ModelInfoParams) -> Result<ModelInfo> {
        let mut url = self.api_url(Some(RepoType::Model), &params.repo_id);
        if let Some(ref revision) = params.revision {
            url = format!("{url}/revision/{revision}");
        }
        let response = self.inner.client.get(&url)
            .headers(self.auth_headers())
            .send()
            .await?;
        let response = self.check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Repo).await?;
        Ok(response.json().await?)
    }

    /// Get info about a dataset repository.
    /// Endpoint: GET /api/datasets/{repo_id} or /api/datasets/{repo_id}/revision/{revision}
    pub async fn dataset_info(&self, params: &DatasetInfoParams) -> Result<DatasetInfo> {
        let mut url = self.api_url(Some(RepoType::Dataset), &params.repo_id);
        if let Some(ref revision) = params.revision {
            url = format!("{url}/revision/{revision}");
        }
        let response = self.inner.client.get(&url)
            .headers(self.auth_headers())
            .send()
            .await?;
        let response = self.check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Repo).await?;
        Ok(response.json().await?)
    }

    /// Get info about a space.
    /// Endpoint: GET /api/spaces/{repo_id} or /api/spaces/{repo_id}/revision/{revision}
    pub async fn space_info(&self, params: &SpaceInfoParams) -> Result<SpaceInfo> {
        let mut url = self.api_url(Some(RepoType::Space), &params.repo_id);
        if let Some(ref revision) = params.revision {
            url = format!("{url}/revision/{revision}");
        }
        let response = self.inner.client.get(&url)
            .headers(self.auth_headers())
            .send()
            .await?;
        let response = self.check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Repo).await?;
        Ok(response.json().await?)
    }

    /// Check if a repository exists.
    pub async fn repo_exists(&self, params: &RepoExistsParams) -> Result<bool> {
        let url = self.api_url(params.repo_type, &params.repo_id);
        let response = self.inner.client.get(&url)
            .headers(self.auth_headers())
            .send()
            .await?;
        match response.status().as_u16() {
            200..=299 => Ok(true),
            404 => Ok(false),
            401 => Err(HfError::AuthRequired),
            status => {
                let url = response.url().to_string();
                let body = response.text().await.unwrap_or_default();
                Err(HfError::Http {
                    status: reqwest::StatusCode::from_u16(status).unwrap(),
                    url,
                    body,
                })
            }
        }
    }

    /// Check if a specific revision exists in a repository.
    pub async fn revision_exists(&self, params: &RevisionExistsParams) -> Result<bool> {
        let url = format!(
            "{}/revision/{}",
            self.api_url(params.repo_type, &params.repo_id),
            params.revision
        );
        let response = self.inner.client.get(&url)
            .headers(self.auth_headers())
            .send()
            .await?;
        match response.status().as_u16() {
            200..=299 => Ok(true),
            404 => Ok(false),
            401 => Err(HfError::AuthRequired),
            status => {
                let url_str = response.url().to_string();
                let body = response.text().await.unwrap_or_default();
                Err(HfError::Http {
                    status: reqwest::StatusCode::from_u16(status).unwrap(),
                    url: url_str,
                    body,
                })
            }
        }
    }

    /// Check if a file exists in a repository by sending a HEAD request
    /// to the download URL.
    pub async fn file_exists(&self, params: &FileExistsParams) -> Result<bool> {
        let revision = params.revision.as_deref()
            .unwrap_or(constants::DEFAULT_REVISION);
        let url = self.download_url(params.repo_type, &params.repo_id, revision, &params.filename);
        let response = self.inner.client.head(&url)
            .headers(self.auth_headers())
            .send()
            .await?;
        match response.status().as_u16() {
            200..=299 => Ok(true),
            404 => Ok(false),
            401 => Err(HfError::AuthRequired),
            status => {
                let url_str = response.url().to_string();
                let body = response.text().await.unwrap_or_default();
                Err(HfError::Http {
                    status: reqwest::StatusCode::from_u16(status).unwrap(),
                    url: url_str,
                    body,
                })
            }
        }
    }
}
```

### Task 12: Repo Listing Methods

**Files:**
- Modify: `hf_hub/src/api/repo.rs` (append to existing)

- [ ] **Step 1: Implement list methods and repo management**

Append to `hf_hub/src/api/repo.rs`:

```rust
// --- Listing methods (paginated streams) ---

impl HfApi {
    /// List models on the Hub.
    /// Endpoint: GET /api/models
    pub fn list_models(&self, params: &ListModelsParams) -> impl Stream<Item = Result<ModelInfo>> + '_ {
        let url = Url::parse(&format!("{}/api/models", self.inner.endpoint)).unwrap();
        let mut query: Vec<(String, String)> = Vec::new();
        if let Some(ref search) = params.search {
            query.push(("search".into(), search.clone()));
        }
        if let Some(ref author) = params.author {
            query.push(("author".into(), author.clone()));
        }
        if let Some(ref filter) = params.filter {
            query.push(("filter".into(), filter.clone()));
        }
        if let Some(ref sort) = params.sort {
            query.push(("sort".into(), sort.clone()));
        }
        if let Some(limit) = params.limit {
            query.push(("limit".into(), limit.to_string()));
        }
        if let Some(ref pipeline_tag) = params.pipeline_tag {
            query.push(("pipeline_tag".into(), pipeline_tag.clone()));
        }
        if params.full == Some(true) {
            query.push(("full".into(), "true".into()));
        }
        if params.card_data == Some(true) {
            query.push(("cardData".into(), "true".into()));
        }
        if params.fetch_config == Some(true) {
            query.push(("config".into(), "true".into()));
        }
        self.paginate(url, query)
    }

    /// List datasets on the Hub.
    /// Endpoint: GET /api/datasets
    pub fn list_datasets(&self, params: &ListDatasetsParams) -> impl Stream<Item = Result<DatasetInfo>> + '_ {
        let url = Url::parse(&format!("{}/api/datasets", self.inner.endpoint)).unwrap();
        let mut query: Vec<(String, String)> = Vec::new();
        if let Some(ref search) = params.search {
            query.push(("search".into(), search.clone()));
        }
        if let Some(ref author) = params.author {
            query.push(("author".into(), author.clone()));
        }
        if let Some(ref filter) = params.filter {
            query.push(("filter".into(), filter.clone()));
        }
        if let Some(ref sort) = params.sort {
            query.push(("sort".into(), sort.clone()));
        }
        if let Some(limit) = params.limit {
            query.push(("limit".into(), limit.to_string()));
        }
        if params.full == Some(true) {
            query.push(("full".into(), "true".into()));
        }
        self.paginate(url, query)
    }

    /// List spaces on the Hub.
    /// Endpoint: GET /api/spaces
    pub fn list_spaces(&self, params: &ListSpacesParams) -> impl Stream<Item = Result<SpaceInfo>> + '_ {
        let url = Url::parse(&format!("{}/api/spaces", self.inner.endpoint)).unwrap();
        let mut query: Vec<(String, String)> = Vec::new();
        if let Some(ref search) = params.search {
            query.push(("search".into(), search.clone()));
        }
        if let Some(ref author) = params.author {
            query.push(("author".into(), author.clone()));
        }
        if let Some(ref filter) = params.filter {
            query.push(("filter".into(), filter.clone()));
        }
        if let Some(ref sort) = params.sort {
            query.push(("sort".into(), sort.clone()));
        }
        if let Some(limit) = params.limit {
            query.push(("limit".into(), limit.to_string()));
        }
        if params.full == Some(true) {
            query.push(("full".into(), "true".into()));
        }
        self.paginate(url, query)
    }

    // --- Repo Management ---

    /// Create a new repository.
    /// Endpoint: POST /api/repos/create
    pub async fn create_repo(&self, params: &CreateRepoParams) -> Result<RepoUrl> {
        let url = format!("{}/api/repos/create", self.inner.endpoint);

        // Split repo_id into namespace and name
        let (namespace, name) = split_repo_id(&params.repo_id);

        let mut body = serde_json::json!({
            "name": name,
            "private": params.private.unwrap_or(false),
        });

        if let Some(ns) = namespace {
            body["organization"] = serde_json::Value::String(ns.to_string());
        }
        if let Some(ref repo_type) = params.repo_type {
            body["type"] = serde_json::Value::String(repo_type.to_string());
        }
        if let Some(ref sdk) = params.space_sdk {
            body["sdk"] = serde_json::Value::String(sdk.clone());
        }

        let response = self.inner.client.post(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;

        if response.status().as_u16() == 409 && params.exist_ok {
            // Repo already exists, construct RepoUrl manually
            let prefix = constants::repo_type_url_prefix(params.repo_type);
            return Ok(RepoUrl {
                url: format!("{}/{}{}", self.inner.endpoint, prefix, params.repo_id),
            });
        }

        let response = self.check_response(response, None, crate::error::NotFoundContext::Generic).await?;
        Ok(response.json().await?)
    }

    /// Delete a repository.
    /// Endpoint: DELETE /api/repos/delete
    pub async fn delete_repo(&self, params: &DeleteRepoParams) -> Result<()> {
        let url = format!("{}/api/repos/delete", self.inner.endpoint);

        let (namespace, name) = split_repo_id(&params.repo_id);

        let mut body = serde_json::json!({ "name": name });
        if let Some(ns) = namespace {
            body["organization"] = serde_json::Value::String(ns.to_string());
        }
        if let Some(ref repo_type) = params.repo_type {
            body["type"] = serde_json::Value::String(repo_type.to_string());
        }

        let response = self.inner.client.delete(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;

        if response.status().as_u16() == 404 && params.missing_ok {
            return Ok(());
        }

        self.check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Repo).await?;
        Ok(())
    }

    /// Update repository settings.
    /// Endpoint: PUT /api/{repo_type}s/{repo_id}/settings
    pub async fn update_repo_settings(&self, params: &UpdateRepoParams) -> Result<()> {
        let url = format!("{}/settings", self.api_url(params.repo_type, &params.repo_id));
        let mut body = serde_json::Map::new();

        if let Some(private) = params.private {
            body.insert("private".into(), serde_json::Value::Bool(private));
        }
        if let Some(ref gated) = params.gated {
            body.insert("gated".into(), serde_json::Value::String(gated.clone()));
        }
        if let Some(ref description) = params.description {
            body.insert("description".into(), serde_json::Value::String(description.clone()));
        }

        let response = self.inner.client.put(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;

        self.check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Repo).await?;
        Ok(())
    }

    /// Move (rename) a repository.
    /// Endpoint: POST /api/repos/move
    pub async fn move_repo(&self, params: &MoveRepoParams) -> Result<RepoUrl> {
        let url = format!("{}/api/repos/move", self.inner.endpoint);
        let mut body = serde_json::json!({
            "fromRepo": params.from_id,
            "toRepo": params.to_id,
        });
        if let Some(ref repo_type) = params.repo_type {
            body["type"] = serde_json::Value::String(repo_type.to_string());
        }

        let response = self.inner.client.post(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;

        self.check_response(response, None, crate::error::NotFoundContext::Generic).await?;
        let prefix = constants::repo_type_url_prefix(params.repo_type);
        Ok(RepoUrl {
            url: format!("{}/{}{}", self.inner.endpoint, prefix, params.to_id),
        })
    }
}

/// Split "namespace/name" into (Some("namespace"), "name") or (None, "name")
fn split_repo_id(repo_id: &str) -> (Option<&str>, &str) {
    match repo_id.split_once('/') {
        Some((ns, name)) => (Some(ns), name),
        None => (None, repo_id),
    }
}
```

### Task 13: Verify Chunk 3 Compiles

- [ ] **Step 1: Run cargo check**

Run: `cargo check`
Expected: PASS

- [ ] **Step 2: Commit**

```bash
git add -A && git commit -m "feat: implement repo info, listing, existence checks, and repo management"
```

---

## Chunk 4: User Operations

### Task 14: User API Methods

**Files:**
- Modify: `hf_hub/src/api/users.rs`

- [ ] **Step 1: Implement user methods**

```rust
use futures::Stream;
use url::Url;
use crate::client::HfApi;
use crate::error::{HfError, Result};
use crate::types::*;

impl HfApi {
    /// Get authenticated user info.
    /// Endpoint: GET /api/whoami-v2
    pub async fn whoami(&self) -> Result<User> {
        let url = format!("{}/api/whoami-v2", self.inner.endpoint);
        let response = self.inner.client.get(&url)
            .headers(self.auth_headers())
            .send()
            .await?;
        let response = self.check_response(response, None, crate::error::NotFoundContext::Generic).await?;
        Ok(response.json().await?)
    }

    /// Check if the current token is valid.
    /// Endpoint: GET /api/whoami-v2
    /// Returns Ok(()) on success, Err(AuthRequired) if invalid.
    pub async fn auth_check(&self) -> Result<()> {
        self.whoami().await?;
        Ok(())
    }

    /// Get overview of a user.
    /// Endpoint: GET /api/users/{username}/overview
    pub async fn get_user_overview(&self, username: &str) -> Result<User> {
        let url = format!("{}/api/users/{}/overview", self.inner.endpoint, username);
        let response = self.inner.client.get(&url)
            .headers(self.auth_headers())
            .send()
            .await?;
        let response = self.check_response(response, None, crate::error::NotFoundContext::Generic).await?;
        Ok(response.json().await?)
    }

    /// Get overview of an organization.
    /// Endpoint: GET /api/organizations/{organization}/overview
    pub async fn get_organization_overview(&self, organization: &str) -> Result<Organization> {
        let url = format!(
            "{}/api/organizations/{}/overview",
            self.inner.endpoint, organization
        );
        let response = self.inner.client.get(&url)
            .headers(self.auth_headers())
            .send()
            .await?;
        let response = self.check_response(response, None, crate::error::NotFoundContext::Generic).await?;
        Ok(response.json().await?)
    }

    /// List followers of a user.
    /// Endpoint: GET /api/users/{username}/followers
    pub fn list_user_followers(&self, username: &str) -> impl Stream<Item = Result<User>> + '_ {
        let url = Url::parse(&format!(
            "{}/api/users/{}/followers",
            self.inner.endpoint, username
        ))
        .unwrap();
        self.paginate(url, vec![])
    }

    /// List users that a user is following.
    /// Endpoint: GET /api/users/{username}/following
    pub fn list_user_following(&self, username: &str) -> impl Stream<Item = Result<User>> + '_ {
        let url = Url::parse(&format!(
            "{}/api/users/{}/following",
            self.inner.endpoint, username
        ))
        .unwrap();
        self.paginate(url, vec![])
    }

    /// List members of an organization.
    /// Endpoint: GET /api/organizations/{organization}/members
    pub fn list_organization_members(
        &self,
        organization: &str,
    ) -> impl Stream<Item = Result<User>> + '_ {
        let url = Url::parse(&format!(
            "{}/api/organizations/{}/members",
            self.inner.endpoint, organization
        ))
        .unwrap();
        self.paginate(url, vec![])
    }
}
```

### Task 15: Verify + Commit

- [ ] **Step 1: Run cargo check**

Run: `cargo check`
Expected: PASS

- [ ] **Step 2: Commit**

```bash
git add -A && git commit -m "feat: implement user and organization API methods"
```

---

## Chunk 5: File Listing, Commits, Diffs, Branches, Tags

### Task 16: File Listing Methods

**Files:**
- Modify: `hf_hub/src/api/files.rs`

- [ ] **Step 1: Implement file listing methods**

```rust
use std::path::PathBuf;
use futures::stream::{self, Stream, StreamExt, TryStreamExt};
use url::Url;
use crate::client::HfApi;
use crate::constants;
use crate::error::{HfError, Result};
use crate::types::*;

impl HfApi {
    /// List file paths in a repository (convenience wrapper over list_repo_tree).
    /// Returns all file paths recursively.
    pub async fn list_repo_files(&self, params: &ListRepoFilesParams) -> Result<Vec<String>> {
        let tree_params = ListRepoTreeParams::builder()
            .repo_id(&params.repo_id)
            .recursive(true)
            .build();
        // Copy over optional fields
        let tree_params = ListRepoTreeParams {
            revision: params.revision.clone(),
            repo_type: params.repo_type,
            ..tree_params
        };

        let stream = self.list_repo_tree(&tree_params);
        futures::pin_mut!(stream);

        let mut files = Vec::new();
        while let Some(entry) = stream.next().await {
            let entry = entry?;
            if let RepoTreeEntry::File { path, .. } = entry {
                files.push(path);
            }
        }
        Ok(files)
    }

    /// List files and directories in a repository tree.
    /// Endpoint: GET /api/{repo_type}s/{repo_id}/tree/{revision}
    pub fn list_repo_tree(
        &self,
        params: &ListRepoTreeParams,
    ) -> impl Stream<Item = Result<RepoTreeEntry>> + '_ {
        let revision = params.revision.as_deref()
            .unwrap_or(constants::DEFAULT_REVISION);
        let url_str = format!(
            "{}/tree/{}",
            self.api_url(params.repo_type, &params.repo_id),
            revision
        );
        let url = Url::parse(&url_str).unwrap();

        let mut query: Vec<(String, String)> = Vec::new();
        if params.recursive {
            query.push(("recursive".into(), "true".into()));
        }
        if params.expand {
            query.push(("expand".into(), "true".into()));
        }

        self.paginate(url, query)
    }

    /// Get info about specific paths in a repository.
    /// Endpoint: POST /api/{repo_type}s/{repo_id}/paths-info/{revision}
    pub async fn get_paths_info(&self, params: &GetPathsInfoParams) -> Result<Vec<RepoTreeEntry>> {
        let revision = params.revision.as_deref()
            .unwrap_or(constants::DEFAULT_REVISION);
        let url = format!(
            "{}/paths-info/{}",
            self.api_url(params.repo_type, &params.repo_id),
            revision
        );

        let body = serde_json::json!({
            "paths": params.paths,
        });

        let response = self.inner.client.post(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;

        let response = self.check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Entry { path: params.paths.join(", ") }).await?;
        Ok(response.json().await?)
    }
}
```

### Task 17: Commit and Diff Methods

**Files:**
- Modify: `hf_hub/src/api/commits.rs`

- [ ] **Step 1: Implement commit, diff, branch, and tag methods**

```rust
use futures::Stream;
use url::Url;
use crate::client::HfApi;
use crate::constants;
use crate::error::{HfError, Result};
use crate::types::*;

impl HfApi {
    /// List commits in a repository.
    /// Endpoint: GET /api/{repo_type}s/{repo_id}/commits/{revision}
    pub fn list_repo_commits(
        &self,
        params: &ListRepoCommitsParams,
    ) -> impl Stream<Item = Result<GitCommitInfo>> + '_ {
        let revision = params.revision.as_deref()
            .unwrap_or(constants::DEFAULT_REVISION);
        let url_str = format!(
            "{}/commits/{}",
            self.api_url(params.repo_type, &params.repo_id),
            revision
        );
        let url = Url::parse(&url_str).unwrap();
        self.paginate(url, vec![])
    }

    /// List branches, tags, and (optionally) pull requests.
    /// Endpoint: GET /api/{repo_type}s/{repo_id}/refs
    pub async fn list_repo_refs(&self, params: &ListRepoRefsParams) -> Result<GitRefs> {
        let url = format!("{}/refs", self.api_url(params.repo_type, &params.repo_id));
        let mut query: Vec<(&str, String)> = Vec::new();
        if params.include_pull_requests {
            query.push(("include_prs", "1".into()));
        }

        let response = self.inner.client.get(&url)
            .headers(self.auth_headers())
            .query(&query)
            .send()
            .await?;

        let response = self.check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Repo).await?;
        Ok(response.json().await?)
    }

    /// Get the structured diff between two revisions.
    /// Endpoint: GET /api/{repo_type}s/{repo_id}/compare/{compare}
    /// `compare` is in the format "revA...revB"
    pub async fn get_commit_diff(&self, params: &GetCommitDiffParams) -> Result<Vec<DiffEntry>> {
        let url = format!(
            "{}/compare/{}",
            self.api_url(params.repo_type, &params.repo_id),
            params.compare
        );

        let response = self.inner.client.get(&url)
            .headers(self.auth_headers())
            .send()
            .await?;

        let response = self.check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Repo).await?;
        Ok(response.json().await?)
    }

    /// Get the raw diff between two revisions.
    /// Endpoint: GET /api/{repo_type}s/{repo_id}/compare/{compare}?raw=true
    pub async fn get_raw_diff(&self, params: &GetRawDiffParams) -> Result<String> {
        let url = format!(
            "{}/compare/{}",
            self.api_url(params.repo_type, &params.repo_id),
            params.compare
        );

        let response = self.inner.client.get(&url)
            .headers(self.auth_headers())
            .query(&[("raw", "true")])
            .send()
            .await?;

        let response = self.check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Repo).await?;
        Ok(response.text().await?)
    }

    /// Create a new branch.
    /// Endpoint: POST /api/{repo_type}s/{repo_id}/branch/{branch}
    pub async fn create_branch(&self, params: &CreateBranchParams) -> Result<()> {
        let url = format!(
            "{}/branch/{}",
            self.api_url(params.repo_type, &params.repo_id),
            params.branch
        );

        let mut body = serde_json::Map::new();
        if let Some(ref revision) = params.revision {
            body.insert(
                "startingPoint".into(),
                serde_json::Value::String(revision.clone()),
            );
        }

        let response = self.inner.client.post(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;

        self.check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Repo).await?;
        Ok(())
    }

    /// Delete a branch.
    /// Endpoint: DELETE /api/{repo_type}s/{repo_id}/branch/{branch}
    pub async fn delete_branch(&self, params: &DeleteBranchParams) -> Result<()> {
        let url = format!(
            "{}/branch/{}",
            self.api_url(params.repo_type, &params.repo_id),
            params.branch
        );

        let response = self.inner.client.delete(&url)
            .headers(self.auth_headers())
            .send()
            .await?;

        self.check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Repo).await?;
        Ok(())
    }

    /// Create a new tag.
    /// Endpoint: POST /api/{repo_type}s/{repo_id}/tag/{revision}
    pub async fn create_tag(&self, params: &CreateTagParams) -> Result<()> {
        let revision = params.revision.as_deref()
            .unwrap_or(constants::DEFAULT_REVISION);
        let url = format!(
            "{}/tag/{}",
            self.api_url(params.repo_type, &params.repo_id),
            revision
        );

        let mut body = serde_json::json!({ "tag": params.tag });
        if let Some(ref message) = params.message {
            body["message"] = serde_json::Value::String(message.clone());
        }

        let response = self.inner.client.post(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;

        self.check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Repo).await?;
        Ok(())
    }

    /// Delete a tag.
    /// Endpoint: DELETE /api/{repo_type}s/{repo_id}/tag/{tag}
    pub async fn delete_tag(&self, params: &DeleteTagParams) -> Result<()> {
        let url = format!(
            "{}/tag/{}",
            self.api_url(params.repo_type, &params.repo_id),
            params.tag
        );

        let response = self.inner.client.delete(&url)
            .headers(self.auth_headers())
            .send()
            .await?;

        self.check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Repo).await?;
        Ok(())
    }
}
```

### Task 18: Verify + Commit

- [ ] **Step 1: Run cargo check**

Run: `cargo check`
Expected: PASS

- [ ] **Step 2: Commit**

```bash
git add -A && git commit -m "feat: implement file listing, commits, diffs, branches, and tags"
```

---

## Chunk 6: File Download

### Task 19: Download File (Regular HTTP)

**Files:**
- Modify: `hf_hub/src/api/files.rs` (append)

- [ ] **Step 1: Implement download_file**

Append to `hf_hub/src/api/files.rs`:

```rust
impl HfApi {
    /// Download a single file from a repository to a local directory.
    ///
    /// Sends a HEAD request first to check for xet headers.
    /// If xet headers are present and the "xet" feature is not enabled,
    /// returns HfError::XetNotEnabled.
    /// Otherwise, streams the file content to `local_dir/filename`.
    ///
    /// Endpoint: GET {endpoint}/{prefix}{repo_id}/resolve/{revision}/{filename}
    pub async fn download_file(&self, params: &DownloadFileParams) -> Result<PathBuf> {
        let revision = params.revision.as_deref()
            .unwrap_or(constants::DEFAULT_REVISION);
        let url = self.download_url(
            params.repo_type,
            &params.repo_id,
            revision,
            &params.filename,
        );

        // HEAD request to check metadata (xet headers, redirects)
        let head_response = self.inner.client.head(&url)
            .headers(self.auth_headers())
            .send()
            .await?;

        let head_response = self.check_response(head_response, Some(&params.repo_id), crate::error::NotFoundContext::Entry { path: params.filename.clone() }).await?;

        // Check for xet headers
        if head_response.headers().get("x-xet-hash").is_some() {
            #[cfg(feature = "xet")]
            {
                return crate::xet::xet_download(self, params, &head_response).await;
            }
            #[cfg(not(feature = "xet"))]
            {
                return Err(HfError::XetNotEnabled);
            }
        }

        // Standard HTTP download
        let response = self.inner.client.get(&url)
            .headers(self.auth_headers())
            .send()
            .await?;
        let response = self.check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Entry { path: params.filename.clone() }).await?;

        // Ensure local_dir exists
        tokio::fs::create_dir_all(&params.local_dir).await?;

        // Determine output path — preserve subdirectory structure from filename
        let dest_path = params.local_dir.join(&params.filename);
        if let Some(parent) = dest_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Stream response to file
        let mut file = tokio::fs::File::create(&dest_path).await?;
        let mut stream = response.bytes_stream();
        use tokio::io::AsyncWriteExt;
        use futures::StreamExt;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
        }
        file.flush().await?;

        Ok(dest_path)
    }
}
```

### Task 20: Xet Stubs

**Files:**
- Modify: `hf_hub/src/xet.rs`

- [ ] **Step 1: Create xet module with placeholder for future xet feature**

```rust
//! Xet high-performance transfer support.
//!
//! This module is only active when the "xet" feature is enabled.
//! When xet headers are detected during download/upload but the feature
//! is not enabled, HfError::XetNotEnabled is returned.

#[cfg(feature = "xet")]
pub(crate) struct XetConnectionInfo {
    pub endpoint: String,
    pub access_token: String,
    pub expiration_unix_epoch: u64,
}

#[cfg(feature = "xet")]
pub(crate) async fn xet_download(
    _api: &crate::client::HfApi,
    _params: &crate::types::DownloadFileParams,
    _head_response: &reqwest::Response,
) -> crate::error::Result<std::path::PathBuf> {
    // TODO: Implement xet download using hf-xet crate
    Err(crate::error::HfError::Other(
        "Xet download not yet implemented".to_string(),
    ))
}

#[cfg(feature = "xet")]
pub(crate) async fn xet_upload(
    _api: &crate::client::HfApi,
    _files: &[(String, crate::types::AddSource)],
    _repo_id: &str,
    _repo_type: Option<crate::types::RepoType>,
    _revision: &str,
) -> crate::error::Result<()> {
    // TODO: Implement xet upload using hf-xet crate
    Err(crate::error::HfError::Other(
        "Xet upload not yet implemented".to_string(),
    ))
}
```

### Task 21: Verify + Commit

- [ ] **Step 1: Run cargo check**

Run: `cargo check`
Expected: PASS

- [ ] **Step 2: Commit**

```bash
git add -A && git commit -m "feat: implement file download with xet detection and stubs"
```

---

## Chunk 7: File Upload and create_commit

### Task 22: Create Commit (Multipart Upload)

**Files:**
- Modify: `hf_hub/src/api/files.rs` (append)

- [ ] **Step 1: Implement create_commit, upload_file, upload_folder, delete_file, delete_folder**

Append to `hf_hub/src/api/files.rs`:

```rust
use reqwest::multipart;

impl HfApi {
    /// Create a commit with multiple operations.
    ///
    /// For add operations, this uploads files via multipart form to
    /// POST /api/{repo_type}s/{repo_id}/commit/{revision}
    ///
    /// **IMPLEMENTATION NOTE:** The multipart protocol below is based on the
    /// Python huggingface_hub library's implementation. The exact format
    /// (header JSON structure, part naming) MUST be validated against the
    /// live Hub API during integration testing. The Python library's
    /// `_commit_api.py` is the reference implementation. If the format
    /// doesn't match, refer to the Python source for the correct protocol.
    ///
    /// For xet-enabled repos, if the server negotiates xet transfer,
    /// the xet feature must be enabled or HfError::XetNotEnabled is returned.
    pub async fn create_commit(&self, params: &CreateCommitParams) -> Result<CommitInfo> {
        let revision = params.revision.as_deref()
            .unwrap_or(constants::DEFAULT_REVISION);
        let url = format!(
            "{}/commit/{}",
            self.api_url(params.repo_type, &params.repo_id),
            revision
        );

        // Build the multipart form
        let mut form = multipart::Form::new();

        // Add commit metadata as JSON header part
        let mut header = serde_json::json!({
            "summary": params.commit_message,
        });
        if let Some(ref desc) = params.commit_description {
            header["description"] = serde_json::Value::String(desc.clone());
        }
        if let Some(ref parent) = params.parent_commit {
            header["parentCommit"] = serde_json::Value::String(parent.clone());
        }

        // Process operations
        let mut operations_json = Vec::new();

        for op in &params.operations {
            match op {
                CommitOperation::Add { path_in_repo, source } => {
                    let content = match source {
                        AddSource::File(path) => tokio::fs::read(path).await?,
                        AddSource::Bytes(bytes) => bytes.clone(),
                    };

                    operations_json.push(serde_json::json!({
                        "key": "file",
                        "path": path_in_repo,
                    }));

                    let part = multipart::Part::bytes(content)
                        .file_name(path_in_repo.clone());
                    form = form.part(format!("file:{}", path_in_repo), part);
                }
                CommitOperation::Delete { path_in_repo } => {
                    operations_json.push(serde_json::json!({
                        "key": "deletedFile",
                        "path": path_in_repo,
                    }));
                }
            }
        }

        header["lfsFiles"] = serde_json::json!([]);
        header["files"] = serde_json::json!(operations_json);

        // The header JSON goes as the first part
        let header_part = multipart::Part::text(serde_json::to_string(&header)?)
            .mime_str("application/json")?;
        form = form.part("header", header_part);

        let mut request = self.inner.client.post(&url)
            .headers(self.auth_headers())
            .multipart(form);

        if let Some(create_pr) = params.create_pr {
            if create_pr {
                request = request.query(&[("create_pr", "1")]);
            }
        }

        let response = request.send().await?;
        let response = self.check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Repo).await?;
        Ok(response.json().await?)
    }

    /// Upload a single file to a repository. Convenience wrapper around create_commit.
    pub async fn upload_file(&self, params: &UploadFileParams) -> Result<CommitInfo> {
        let commit_message = params.commit_message.clone()
            .unwrap_or_else(|| format!("Upload {}", params.path_in_repo));

        let commit_params = CreateCommitParams::builder()
            .repo_id(&params.repo_id)
            .operations(vec![CommitOperation::Add {
                path_in_repo: params.path_in_repo.clone(),
                source: params.source.clone(),
            }])
            .commit_message(commit_message)
            .build();

        let commit_params = CreateCommitParams {
            commit_description: params.commit_description.clone(),
            repo_type: params.repo_type,
            revision: params.revision.clone(),
            create_pr: params.create_pr,
            parent_commit: params.parent_commit.clone(),
            ..commit_params
        };

        self.create_commit(&commit_params).await
    }

    /// Upload a folder to a repository. Walks the directory and creates add operations.
    pub async fn upload_folder(&self, params: &UploadFolderParams) -> Result<CommitInfo> {
        let mut operations = Vec::new();

        let folder = &params.folder_path;
        let base_repo_path = params.path_in_repo.as_deref().unwrap_or("");

        collect_files_recursive(
            folder,
            folder,
            base_repo_path,
            &params.allow_patterns,
            &params.ignore_patterns,
            &mut operations,
        )
        .await?;

        // If delete_patterns is set, list existing remote files and add delete
        // operations for any that match the patterns.
        if let Some(ref delete_patterns) = params.delete_patterns {
            let revision = params.revision.as_deref()
                .unwrap_or(constants::DEFAULT_REVISION);
            let tree_params = ListRepoTreeParams::builder()
                .repo_id(&params.repo_id)
                .recursive(true)
                .build();
            let tree_params = ListRepoTreeParams {
                revision: Some(revision.to_string()),
                repo_type: params.repo_type,
                ..tree_params
            };
            let stream = self.list_repo_tree(&tree_params);
            futures::pin_mut!(stream);
            while let Some(entry) = stream.next().await {
                let entry = entry?;
                if let RepoTreeEntry::File { path, .. } = entry {
                    if matches_any_glob(delete_patterns, &path) {
                        operations.push(CommitOperation::Delete { path_in_repo: path });
                    }
                }
            }
        }

        let commit_message = params.commit_message.clone()
            .unwrap_or_else(|| "Upload folder".to_string());

        let commit_params = CreateCommitParams::builder()
            .repo_id(&params.repo_id)
            .operations(operations)
            .commit_message(commit_message)
            .build();

        let commit_params = CreateCommitParams {
            commit_description: params.commit_description.clone(),
            repo_type: params.repo_type,
            revision: params.revision.clone(),
            create_pr: params.create_pr,
            ..commit_params
        };

        self.create_commit(&commit_params).await
    }

    /// Delete a file from a repository. Convenience wrapper around create_commit.
    pub async fn delete_file(&self, params: &DeleteFileParams) -> Result<CommitInfo> {
        let commit_message = params.commit_message.clone()
            .unwrap_or_else(|| format!("Delete {}", params.path_in_repo));

        let commit_params = CreateCommitParams::builder()
            .repo_id(&params.repo_id)
            .operations(vec![CommitOperation::Delete {
                path_in_repo: params.path_in_repo.clone(),
            }])
            .commit_message(commit_message)
            .build();

        let commit_params = CreateCommitParams {
            repo_type: params.repo_type,
            revision: params.revision.clone(),
            create_pr: params.create_pr,
            ..commit_params
        };

        self.create_commit(&commit_params).await
    }

    /// Delete a folder from a repository. Lists files under the path and deletes them.
    pub async fn delete_folder(&self, params: &DeleteFolderParams) -> Result<CommitInfo> {
        let revision = params.revision.as_deref()
            .unwrap_or(constants::DEFAULT_REVISION);

        // List all files under the folder path
        let tree_params = ListRepoTreeParams::builder()
            .repo_id(&params.repo_id)
            .recursive(true)
            .build();
        let tree_params = ListRepoTreeParams {
            revision: Some(revision.to_string()),
            repo_type: params.repo_type,
            ..tree_params
        };

        let stream = self.list_repo_tree(&tree_params);
        futures::pin_mut!(stream);

        let mut operations = Vec::new();
        let prefix = if params.path_in_repo.ends_with('/') {
            params.path_in_repo.clone()
        } else {
            format!("{}/", params.path_in_repo)
        };

        while let Some(entry) = stream.next().await {
            let entry = entry?;
            if let RepoTreeEntry::File { path, .. } = entry {
                if path.starts_with(&prefix) || path == params.path_in_repo {
                    operations.push(CommitOperation::Delete { path_in_repo: path });
                }
            }
        }

        let commit_message = params.commit_message.clone()
            .unwrap_or_else(|| format!("Delete {}", params.path_in_repo));

        let commit_params = CreateCommitParams::builder()
            .repo_id(&params.repo_id)
            .operations(operations)
            .commit_message(commit_message)
            .build();

        let commit_params = CreateCommitParams {
            repo_type: params.repo_type,
            revision: Some(revision.to_string()),
            create_pr: params.create_pr,
            ..commit_params
        };

        self.create_commit(&commit_params).await
    }
}

/// Recursively collect files from a directory into CommitOperation::Add entries.
/// Respects allow_patterns and ignore_patterns (glob-style).
async fn collect_files_recursive(
    root: &std::path::Path,
    current: &std::path::Path,
    base_repo_path: &str,
    allow_patterns: &Option<Vec<String>>,
    ignore_patterns: &Option<Vec<String>>,
    operations: &mut Vec<CommitOperation>,
) -> Result<()> {
    let mut entries = tokio::fs::read_dir(current).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let metadata = entry.metadata().await?;

        if metadata.is_dir() {
            Box::pin(collect_files_recursive(
                root,
                &path,
                base_repo_path,
                allow_patterns,
                ignore_patterns,
                operations,
            ))
            .await?;
        } else if metadata.is_file() {
            let relative = path.strip_prefix(root)
                .map_err(|e| HfError::Other(e.to_string()))?;
            let relative_str = relative.to_string_lossy();

            // Apply pattern filtering using globset
            if let Some(ref allow) = allow_patterns {
                if !matches_any_glob(allow, &relative_str) {
                    continue;
                }
            }
            if let Some(ref ignore) = ignore_patterns {
                if matches_any_glob(ignore, &relative_str) {
                    continue;
                }
            }

            let repo_path = if base_repo_path.is_empty() {
                relative_str.to_string()
            } else {
                format!("{}/{}", base_repo_path.trim_end_matches('/'), relative_str)
            };

            operations.push(CommitOperation::Add {
                path_in_repo: repo_path,
                source: AddSource::File(path),
            });
        }
    }

    Ok(())
}

/// Check if a path matches any of the given glob patterns using the `globset` crate.
fn matches_any_glob(patterns: &[String], path: &str) -> bool {
    use globset::Glob;
    patterns.iter().any(|p| {
        Glob::new(p)
            .ok()
            .and_then(|g| Some(g.compile_matcher().is_match(path)))
            .unwrap_or(false)
    })
}
```

### Task 23: Update lib.rs exports

**Files:**
- Modify: `hf_hub/src/lib.rs`

- [ ] **Step 1: Add xet module declaration**

Update `hf_hub/src/lib.rs` to include the xet module:

```rust
pub mod constants;
pub mod error;
pub mod types;
pub mod client;
pub mod pagination;
pub mod api;
pub(crate) mod xet;

pub use client::{HfApi, HfApiBuilder};
pub use error::{HfError, Result};
pub use types::*;
```

### Task 24: Verify + Commit

- [ ] **Step 1: Run cargo check**

Run: `cargo check`
Expected: PASS

- [ ] **Step 2: Run all existing tests**

Run: `cargo test -p hf-hub`
Expected: All pagination tests pass

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "feat: implement file upload, download, create_commit, and folder operations"
```

---

## Chunk 8: Integration Tests

### Task 25: Integration Tests Against Live Hub API

**Files:**
- Create: `hf_hub/tests/integration_test.rs`

These tests hit the real Hugging Face Hub API. They require a valid `HF_TOKEN` environment variable. Read-only tests skip gracefully if no token is set. Write tests require `HF_TEST_WRITE=1`.

- [ ] **Step 1: Write integration tests**

```rust
//! Integration tests against the live Hugging Face Hub API.
//!
//! Read-only tests: require HF_TOKEN, skip if not set.
//! Write tests: require HF_TOKEN + HF_TEST_WRITE=1, skip otherwise.
//!
//! Run read-only: HF_TOKEN=hf_xxx cargo test -p hf-hub --test integration_test
//! Run all: HF_TOKEN=hf_xxx HF_TEST_WRITE=1 cargo test -p hf-hub --test integration_test

use futures::StreamExt;
use hf_hub::{HfApi, HfApiBuilder};
use hf_hub::types::*;

fn api() -> Option<HfApi> {
    if std::env::var("HF_TOKEN").is_err() {
        return None;
    }
    Some(HfApiBuilder::new().build().expect("Failed to create HfApi"))
}

fn write_enabled() -> bool {
    std::env::var("HF_TEST_WRITE").ok().map_or(false, |v| v == "1")
}

macro_rules! skip_if_no_token {
    () => {
        let Some(api) = api() else {
            eprintln!("Skipping test: HF_TOKEN not set");
            return;
        };
    };
}

#[tokio::test]
async fn test_model_info() {
    let Some(api) = api() else { return };
    let params = ModelInfoParams::builder()
        .repo_id("gpt2")
        .build();
    let info = api.model_info(&params).await.unwrap();
    assert_eq!(info.id, "openai-community/gpt2");
}

#[tokio::test]
async fn test_dataset_info() {
    let Some(api) = api() else { return };
    let params = DatasetInfoParams::builder()
        .repo_id("rajpurkar/squad")
        .build();
    let info = api.dataset_info(&params).await.unwrap();
    assert!(info.id.contains("squad"));
}

#[tokio::test]
async fn test_repo_exists() {
    let Some(api) = api() else { return };
    let params = RepoExistsParams::builder()
        .repo_id("gpt2")
        .build();
    assert!(api.repo_exists(&params).await.unwrap());

    let params = RepoExistsParams::builder()
        .repo_id("this-repo-definitely-does-not-exist-12345")
        .build();
    assert!(!api.repo_exists(&params).await.unwrap());
}

#[tokio::test]
async fn test_file_exists() {
    let Some(api) = api() else { return };
    let params = FileExistsParams::builder()
        .repo_id("gpt2")
        .filename("config.json")
        .build();
    assert!(api.file_exists(&params).await.unwrap());

    let params = FileExistsParams::builder()
        .repo_id("gpt2")
        .filename("nonexistent_file.xyz")
        .build();
    assert!(!api.file_exists(&params).await.unwrap());
}

#[tokio::test]
async fn test_list_models() {
    let Some(api) = api() else { return };
    let params = ListModelsParams::builder()
        .author("openai-community")
        .limit(3_usize)
        .build();
    let stream = api.list_models(&params);
    futures::pin_mut!(stream);

    let mut count = 0;
    while let Some(model) = stream.next().await {
        let model = model.unwrap();
        assert!(model.id.starts_with("openai-community/"));
        count += 1;
        if count >= 3 {
            break;
        }
    }
    assert!(count > 0);
}

#[tokio::test]
async fn test_list_repo_files() {
    let Some(api) = api() else { return };
    let params = ListRepoFilesParams::builder()
        .repo_id("gpt2")
        .build();
    let files = api.list_repo_files(&params).await.unwrap();
    assert!(files.contains(&"config.json".to_string()));
    assert!(files.contains(&"README.md".to_string()));
}

#[tokio::test]
async fn test_list_repo_tree() {
    let Some(api) = api() else { return };
    let params = ListRepoTreeParams::builder()
        .repo_id("gpt2")
        .build();
    let stream = api.list_repo_tree(&params);
    futures::pin_mut!(stream);

    let mut found_config = false;
    while let Some(entry) = stream.next().await {
        let entry = entry.unwrap();
        if let RepoTreeEntry::File { path, .. } = &entry {
            if path == "config.json" {
                found_config = true;
                break;
            }
        }
    }
    assert!(found_config);
}

#[tokio::test]
async fn test_list_repo_commits() {
    let Some(api) = api() else { return };
    let params = ListRepoCommitsParams::builder()
        .repo_id("gpt2")
        .build();
    let stream = api.list_repo_commits(&params);
    futures::pin_mut!(stream);

    let first = stream.next().await.unwrap().unwrap();
    assert!(!first.id.is_empty());
    assert!(!first.title.is_empty());
}

#[tokio::test]
async fn test_list_repo_refs() {
    let Some(api) = api() else { return };
    let params = ListRepoRefsParams::builder()
        .repo_id("gpt2")
        .build();
    let refs = api.list_repo_refs(&params).await.unwrap();
    assert!(!refs.branches.is_empty());
    // "main" branch should exist
    assert!(refs.branches.iter().any(|b| b.name == "main"));
}

#[tokio::test]
async fn test_revision_exists() {
    let Some(api) = api() else { return };
    let params = RevisionExistsParams::builder()
        .repo_id("gpt2")
        .revision("main")
        .build();
    assert!(api.revision_exists(&params).await.unwrap());

    let params = RevisionExistsParams::builder()
        .repo_id("gpt2")
        .revision("nonexistent-branch-xyz")
        .build();
    assert!(!api.revision_exists(&params).await.unwrap());
}

#[tokio::test]
async fn test_download_file() {
    let Some(api) = api() else { return };
    let dir = tempfile::tempdir().unwrap();
    let params = DownloadFileParams::builder()
        .repo_id("gpt2")
        .filename("config.json")
        .local_dir(dir.path().to_path_buf())
        .build();
    let path = api.download_file(&params).await.unwrap();
    assert!(path.exists());
    let content = std::fs::read_to_string(&path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(json.get("model_type").is_some());
}

// --- Write operation tests (require HF_TEST_WRITE=1) ---

#[tokio::test]
async fn test_create_and_delete_repo() {
    let Some(api) = api() else { return };
    if !write_enabled() { return; }

    let repo_id = format!("{}/hf-hub-rust-test-{}", "assafvayner", uuid_v4_short());

    // Create
    let params = CreateRepoParams::builder()
        .repo_id(&repo_id)
        .private(true)
        .exist_ok(true)
        .build();
    let url = api.create_repo(&params).await.unwrap();
    assert!(url.url.contains(&repo_id));

    // Upload a file
    let params = UploadFileParams::builder()
        .repo_id(&repo_id)
        .source(AddSource::Bytes(b"hello world".to_vec()))
        .path_in_repo("test.txt")
        .commit_message("test upload")
        .build();
    let commit = api.upload_file(&params).await.unwrap();
    assert!(commit.oid.is_some());

    // Verify file exists
    let params = FileExistsParams::builder()
        .repo_id(&repo_id)
        .filename("test.txt")
        .build();
    assert!(api.file_exists(&params).await.unwrap());

    // Delete repo
    let params = DeleteRepoParams::builder()
        .repo_id(&repo_id)
        .build();
    api.delete_repo(&params).await.unwrap();
}

fn uuid_v4_short() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    format!("{:x}{:x}", t.as_secs(), t.subsec_nanos())
}
```

### Task 25b: Unit Tests for Helper Functions

**Files:**
- Create: `hf_hub/src/tests.rs` (or add to relevant modules)

- [ ] **Step 1: Add unit tests for constants and helpers**

Add to `hf_hub/src/constants.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::repo::RepoType;

    #[test]
    fn test_repo_type_url_prefix() {
        assert_eq!(repo_type_url_prefix(None), "");
        assert_eq!(repo_type_url_prefix(Some(RepoType::Model)), "");
        assert_eq!(repo_type_url_prefix(Some(RepoType::Dataset)), "datasets/");
        assert_eq!(repo_type_url_prefix(Some(RepoType::Space)), "spaces/");
    }

    #[test]
    fn test_repo_type_api_segment() {
        assert_eq!(repo_type_api_segment(None), "models");
        assert_eq!(repo_type_api_segment(Some(RepoType::Model)), "models");
        assert_eq!(repo_type_api_segment(Some(RepoType::Dataset)), "datasets");
        assert_eq!(repo_type_api_segment(Some(RepoType::Space)), "spaces");
    }
}
```

Add to `hf_hub/src/types/repo.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repo_type_from_str() {
        assert_eq!("model".parse::<RepoType>().unwrap(), RepoType::Model);
        assert_eq!("dataset".parse::<RepoType>().unwrap(), RepoType::Dataset);
        assert_eq!("space".parse::<RepoType>().unwrap(), RepoType::Space);
        assert_eq!("MODEL".parse::<RepoType>().unwrap(), RepoType::Model);
        assert!("invalid".parse::<RepoType>().is_err());
    }

    #[test]
    fn test_repo_type_display() {
        assert_eq!(RepoType::Model.to_string(), "model");
        assert_eq!(RepoType::Dataset.to_string(), "dataset");
        assert_eq!(RepoType::Space.to_string(), "space");
    }

    #[test]
    fn test_repo_tree_entry_deserialize_file() {
        let json = r#"{"type":"file","oid":"abc123","size":100,"path":"test.txt"}"#;
        let entry: RepoTreeEntry = serde_json::from_str(json).unwrap();
        match entry {
            RepoTreeEntry::File { path, size, .. } => {
                assert_eq!(path, "test.txt");
                assert_eq!(size, 100);
            }
            _ => panic!("Expected File variant"),
        }
    }

    #[test]
    fn test_repo_tree_entry_deserialize_directory() {
        let json = r#"{"type":"directory","oid":"def456","path":"src"}"#;
        let entry: RepoTreeEntry = serde_json::from_str(json).unwrap();
        match entry {
            RepoTreeEntry::Directory { path, .. } => {
                assert_eq!(path, "src");
            }
            _ => panic!("Expected Directory variant"),
        }
    }
}
```

Add to `hf_hub/src/api/repo.rs` (for `split_repo_id`):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_repo_id() {
        assert_eq!(split_repo_id("user/repo"), (Some("user"), "repo"));
        assert_eq!(split_repo_id("repo"), (None, "repo"));
        assert_eq!(split_repo_id("org/sub/repo"), (Some("org"), "sub/repo"));
    }
}
```

### Task 26: Run Integration Tests

- [ ] **Step 1: Run integration tests**

Run: `cargo test -p hf-hub --test integration_test`
Expected: All tests pass (requires `HF_TOKEN` and internet)

- [ ] **Step 2: Fix any deserialization issues**

If any tests fail due to unexpected JSON fields from the API, add `#[serde(flatten)] pub extra: serde_json::Value` or add missing fields to the struct. Iterate until all tests pass.

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "test: add integration tests for read-only Hub API operations"
```

---

## Chunk 9: Final Polish and Documentation

### Task 27: Review Public API and Exports

**Files:**
- Modify: `hf_hub/src/lib.rs`

- [ ] **Step 1: Ensure all public types are re-exported**

Verify that `lib.rs` re-exports everything users need:
- `HfApi`, `HfApiBuilder`
- `HfError`, `Result`
- All types from `types/` (via `pub use types::*`)
- All param structs

- [ ] **Step 2: Add crate-level doc comment**

Add to top of `lib.rs`:

```rust
//! # hf-hub
//!
//! Async Rust client for the [Hugging Face Hub API](https://huggingface.co/docs/hub/api).
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use hf_hub::{HfApi, ModelInfoParams};
//!
//! #[tokio::main]
//! async fn main() -> hf_hub::Result<()> {
//!     let api = HfApi::new()?;
//!     let info = api.model_info(
//!         &ModelInfoParams::builder().repo_id("gpt2").build()
//!     ).await?;
//!     println!("Model: {}", info.id);
//!     Ok(())
//! }
//! ```
```

### Task 28: Final Check

- [ ] **Step 1: Run cargo check**

Run: `cargo check`
Expected: PASS

- [ ] **Step 2: Run all tests**

Run: `cargo test -p hf-hub`
Expected: All tests pass

- [ ] **Step 3: Run clippy**

Run: `cargo clippy -p hf-hub -- -W clippy::all`
Expected: No warnings (fix any that appear)

- [ ] **Step 4: Final commit**

```bash
git add -A && git commit -m "feat: polish public API, add crate docs"
```

---

## Summary

| Chunk | Tasks | What it delivers |
|-------|-------|-----------------|
| 1 | 1-7 | Scaffolding, error types, constants, all core types and params structs |
| 2 | 8-10 | HfApi client + builder, pagination with Link header |
| 3 | 11-13 | Repo info, listing, existence checks, repo management (create/delete/update/move) |
| 4 | 14-15 | User operations (whoami, user info, org info, followers) |
| 5 | 16-18 | File listing, commits, diffs, branches, tags |
| 6 | 19-21 | File download with xet detection + xet stubs |
| 7 | 22-24 | File upload, create_commit, upload_folder, delete operations |
| 8 | 25-26 | Integration tests against live Hub API |
| 9 | 27-28 | Public API polish, docs, clippy |
