# hf-hub: Rust Client for Hugging Face Hub API

## Overview

A Rust library providing an async client (`HfApi`) for the Hugging Face Hub API. It covers repository information, file operations, commits/diffs, user info, downloads, and uploads (including xet-based high-performance transfers behind a feature flag).

The library mirrors the core surface area of the Python `huggingface_hub` library's `HfApi` class, targeting the most commonly used repository and user operations.

## Project Structure

Cargo workspace with a single crate for now. Additional test/companion crates can be added later.

```
huggingface_hub_rust/
├── Cargo.toml              # Workspace root
├── hf_hub/
│   ├── Cargo.toml          # Package: hf-hub
│   └── src/
│       ├── lib.rs          # Public re-exports
│       ├── client.rs       # HfApi, HfApiBuilder, HfApiInner
│       ├── error.rs        # HfError
│       ├── constants.rs    # Env var names, default URLs
│       ├── types/
│       │   ├── mod.rs
│       │   ├── repo.rs     # RepoType, ModelInfo, DatasetInfo, SpaceInfo, RepoTreeEntry, etc.
│       │   ├── user.rs     # User, Organization
│       │   └── commit.rs   # CommitInfo, GitCommitInfo, GitRefInfo, GitRefs, DiffEntry
│       ├── api/
│       │   ├── mod.rs
│       │   ├── repo.rs     # Repo info, listing, create/delete/update
│       │   ├── files.rs    # Upload, download, list tree, file existence
│       │   ├── commits.rs  # List commits, commit detail, diffs, branches, tags
│       │   └── users.rs    # whoami, user info, user repos
│       └── xet.rs          # Xet upload/download (behind "xet" feature)
```

## Core Client

### HfApi

```rust
pub struct HfApi {
    inner: Arc<HfApiInner>,
}

struct HfApiInner {
    client: reqwest::Client,
    endpoint: String,          // Default: https://huggingface.co
    token: Option<String>,
    user_agent: String,
    headers: HeaderMap,
}
```

`HfApi` is cheaply cloneable via `Arc`. All API methods take `&self`.

### HfApiBuilder

```rust
pub struct HfApiBuilder {
    endpoint: Option<String>,
    token: Option<String>,
    user_agent: Option<String>,
    headers: Option<HeaderMap>,
    client: Option<reqwest::Client>,
}
```

Configuration resolution order:
1. Explicit builder values
2. Environment variables (`HF_ENDPOINT`, `HF_TOKEN`, `HF_TOKEN_PATH`)
3. Token file at `~/.cache/huggingface/token` (respects `HF_HOME`, `HF_HUB_CACHE`)
4. Defaults (`endpoint = "https://huggingface.co"`)

Relevant environment variables:
- `HF_ENDPOINT` — Hub API endpoint
- `HF_TOKEN` — Authentication token
- `HF_TOKEN_PATH` — Path to stored token
- `HF_HOME` — Cache directory root (default: `~/.cache/huggingface`)
- `HF_HUB_CACHE` — Hub cache location
- `HF_HUB_DISABLE_IMPLICIT_TOKEN` — Don't auto-load token
- `HF_HUB_USER_AGENT_ORIGIN` — Custom user agent origin

## Dependencies

```toml
[dependencies]
reqwest = { version = "0.12", features = ["json", "stream"] }
tokio = { version = "1", features = ["fs"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
url = "2"
futures = "0.3"
typed-builder = "0.20"

[features]
default = []
xet = ["dep:hf-xet"]

[dependencies.hf-xet]
git = "https://github.com/huggingface/xet-core"
optional = true

[dev-dependencies]
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

The exact crate name and path within the xet-core workspace will be determined during implementation.

## Error Handling

Single unified error enum using `thiserror`:

```rust
#[derive(Error, Debug)]
pub enum HfError {
    #[error("HTTP error: {status} {url}")]
    Http { status: reqwest::StatusCode, url: String, body: String },

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

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, HfError>;
```

HTTP response mapping: 401 → `AuthRequired`, 404 on repo endpoints → `RepoNotFound`, 404 on file/path endpoints → `EntryNotFound`, other non-2xx → `Http`.

**Retry/rate-limit strategy:** All HTTP requests are retried with exponential backoff via `reqwest-middleware` and `reqwest-retry`. Server errors (5xx) and rate-limit responses (429) are retried up to 3 times with exponential backoff delays.

## Types

### Design Principles

- All types derive `Debug`, `Clone`, `Deserialize`. Types used in requests also derive `Serialize`.
- Required fields from the API are non-optional. Everything else is `Option<T>`.
- Known fixed-value fields use proper enums with `FromStr`, `Display`, and serde support.
- Polymorphic responses use `#[serde(tag = "type")]` tagged unions.

### Enums for Known Values

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RepoType {
    Model,
    Dataset,
    Space,
}
// Implements FromStr, Display
```

Other enums to create as encountered: gated status, Space SDK type, Space hardware, Space stage, etc.

### Tagged Unions

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum RepoTreeEntry {
    File {
        oid: String,
        size: u64,
        path: String,
        lfs: Option<BlobLfsInfo>,
        last_commit: Option<LastCommitInfo>,
    },
    Directory {
        oid: String,
        path: String,
    },
}
```

### Commit Operations

Used by `create_commit` to describe file mutations in a single commit:

```rust
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

pub enum AddSource {
    File(PathBuf),
    Bytes(Vec<u8>),
}
```

`CommitOperationCopy` (LFS-only in Python) is excluded since LFS upload is out of scope.

### Key Struct Definitions

**CommitInfo** — returned by upload/commit operations:

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct CommitInfo {
    pub commit_url: String,
    pub commit_message: String,
    pub commit_description: Option<String>,
    pub oid: String,
    pub pr_url: Option<String>,
    pub pr_num: Option<u64>,
}
```

**GitRefs** — returned by `list_repo_refs`:

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct GitRefs {
    pub branches: Vec<GitRefInfo>,
    pub tags: Vec<GitRefInfo>,
    pub converts: Vec<GitRefInfo>,
}
```

**GitCommitInfo** — individual commit metadata:

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct GitCommitInfo {
    pub commit_id: String,
    pub authors: Vec<CommitAuthor>,
    pub created_at: String,
    pub title: String,
    pub message: String,
    pub parents: Vec<String>,
}
```

**Repository types:** `ModelInfo`, `DatasetInfo`, `SpaceInfo` follow the same pattern — `id: String` required, everything else `Option<T>`. `RepoSibling`, `BlobLfsInfo`, `LastCommitInfo`, `RepoUrl` are supporting structs.

> **Note:** `SpaceInfo` and `space_info()` / `list_spaces()` are in scope as read-only operations. Space *management* (runtime, secrets, variables, pause/restart, etc.) is out of scope.

**User types:** `User`, `Organization` — `username`/`name` required, everything else `Option<T>`.

**Xet types:** `XetConnectionInfo` (internal, not public).

### Key Params Struct Definitions

**CreateCommitParams:**

```rust
#[derive(TypedBuilder)]
pub struct CreateCommitParams {
    pub repo_id: String,
    pub operations: Vec<CommitOperation>,
    pub commit_message: String,
    #[builder(default, setter(into, strip_option))]
    pub commit_description: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub create_pr: Option<bool>,
    #[builder(default, setter(into, strip_option))]
    pub parent_commit: Option<String>,
}
```

**UploadFileParams:**

```rust
#[derive(TypedBuilder)]
pub struct UploadFileParams {
    pub repo_id: String,
    pub source: AddSource,
    pub path_in_repo: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub commit_message: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub commit_description: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub create_pr: Option<bool>,
    #[builder(default, setter(into, strip_option))]
    pub parent_commit: Option<String>,
}
```

**UploadFolderParams:**

```rust
#[derive(TypedBuilder)]
pub struct UploadFolderParams {
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
    #[builder(default, setter(into, strip_option))]
    pub create_pr: Option<bool>,
    #[builder(default, setter(into, strip_option))]
    pub allow_patterns: Option<Vec<String>>,
    #[builder(default, setter(into, strip_option))]
    pub ignore_patterns: Option<Vec<String>>,
    #[builder(default, setter(into, strip_option))]
    pub delete_patterns: Option<Vec<String>>,
}
```

**DownloadFileParams:**

```rust
#[derive(TypedBuilder)]
pub struct DownloadFileParams {
    pub repo_id: String,
    pub filename: String,
    pub local_dir: PathBuf,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
}
```

Downloads go to a caller-specified `local_dir`. No caching in v1 — see Out of Scope.

**CreateRepoParams:**

```rust
#[derive(TypedBuilder)]
pub struct CreateRepoParams {
    pub repo_id: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
    #[builder(default, setter(into, strip_option))]
    pub private: Option<bool>,
    #[builder(default)]
    pub exist_ok: bool,
    #[builder(default, setter(into, strip_option))]
    pub space_sdk: Option<String>,
}
```

Other params structs (`DeleteRepoParams`, `ModelInfoParams`, `ListRepoFilesParams`, etc.) follow the same pattern: required fields non-optional, everything else `Option<T>` with `#[builder(default)]`.

## API Methods

### Parameter Design

- **1-2 non-optional parameters:** Direct function arguments
- **3+ parameters (including optionals):** Params struct with `#[derive(TypedBuilder)]`

Param structs use `typed-builder` for ergonomic construction:

```rust
#[derive(TypedBuilder)]
pub struct ListModelsParams {
    #[builder(default, setter(into, strip_option))]
    pub search: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub author: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub limit: Option<usize>,
    // ...
}

// Usage:
let params = ListModelsParams::builder()
    .author("meta-llama")
    .limit(10)
    .build();
```

### Method Inventory

#### Direct Signatures (~7 methods)

```rust
impl HfApi {
    pub async fn whoami(&self) -> Result<User>;
    pub async fn auth_check(&self) -> Result<()>;
    pub async fn get_user_overview(&self, username: &str) -> Result<User>;
    pub async fn get_organization_overview(&self, organization: &str) -> Result<Organization>;
    pub fn list_user_followers(&self, username: &str) -> impl Stream<Item = Result<User>>;
    pub fn list_user_following(&self, username: &str) -> impl Stream<Item = Result<User>>;
    pub fn list_organization_members(&self, organization: &str) -> impl Stream<Item = Result<User>>;
}
```

#### Params-Based Methods (~29 methods)

**Repo info & listing:**
```rust
pub async fn model_info(&self, params: &ModelInfoParams) -> Result<ModelInfo>;
pub async fn dataset_info(&self, params: &DatasetInfoParams) -> Result<DatasetInfo>;
pub async fn space_info(&self, params: &SpaceInfoParams) -> Result<SpaceInfo>;
pub async fn repo_exists(&self, params: &RepoExistsParams) -> Result<bool>;
pub async fn revision_exists(&self, params: &RevisionExistsParams) -> Result<bool>;
pub async fn file_exists(&self, params: &FileExistsParams) -> Result<bool>;
pub fn list_models(&self, params: &ListModelsParams) -> impl Stream<Item = Result<ModelInfo>>;
pub fn list_datasets(&self, params: &ListDatasetsParams) -> impl Stream<Item = Result<DatasetInfo>>;
pub fn list_spaces(&self, params: &ListSpacesParams) -> impl Stream<Item = Result<SpaceInfo>>;
```

**Repo management:**
```rust
pub async fn create_repo(&self, params: &CreateRepoParams) -> Result<RepoUrl>;
pub async fn delete_repo(&self, params: &DeleteRepoParams) -> Result<()>;
pub async fn update_repo_settings(&self, params: &UpdateRepoParams) -> Result<()>;
pub async fn move_repo(&self, params: &MoveRepoParams) -> Result<RepoUrl>;
```

**File operations:**
```rust
pub async fn list_repo_files(&self, params: &ListRepoFilesParams) -> Result<Vec<String>>;
pub fn list_repo_tree(&self, params: &ListRepoTreeParams) -> impl Stream<Item = Result<RepoTreeEntry>>;
pub async fn get_paths_info(&self, params: &GetPathsInfoParams) -> Result<Vec<RepoTreeEntry>>;
pub async fn download_file(&self, params: &DownloadFileParams) -> Result<PathBuf>;
pub async fn upload_file(&self, params: &UploadFileParams) -> Result<CommitInfo>;
pub async fn upload_folder(&self, params: &UploadFolderParams) -> Result<CommitInfo>;
pub async fn delete_file(&self, params: &DeleteFileParams) -> Result<CommitInfo>;
pub async fn delete_folder(&self, params: &DeleteFolderParams) -> Result<CommitInfo>;
pub async fn create_commit(&self, params: &CreateCommitParams) -> Result<CommitInfo>;
```

**Commits & diffs:**
```rust
pub fn list_repo_commits(&self, params: &ListRepoCommitsParams) -> impl Stream<Item = Result<GitCommitInfo>>;
pub async fn list_repo_refs(&self, params: &ListRepoRefsParams) -> Result<GitRefs>;
pub async fn get_commit_diff(&self, params: &GetCommitDiffParams) -> Result<Vec<DiffEntry>>;
pub async fn get_raw_diff(&self, params: &GetRawDiffParams) -> Result<String>;
pub async fn create_branch(&self, params: &CreateBranchParams) -> Result<()>;
pub async fn delete_branch(&self, params: &DeleteBranchParams) -> Result<()>;
pub async fn create_tag(&self, params: &CreateTagParams) -> Result<()>;
pub async fn delete_tag(&self, params: &DeleteTagParams) -> Result<()>;
```

## Pagination

Paginated endpoints return `impl Stream<Item = Result<T>>` using `futures::stream::try_unfold`. No additional crates needed.

```rust
struct PaginationState {
    buffer: VecDeque<serde_json::Value>,
    next_url: Option<Url>,
    is_first_page: bool,
    done: bool,
}

fn paginate<T: DeserializeOwned>(
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
            if let Some(raw) = state.buffer.pop_front() {
                let item: T = serde_json::from_value(raw)?;
                return Ok(Some((item, state)));
            }
            if state.done { return Ok(None); }

            let url = match state.next_url.take() {
                Some(u) => u,
                None => { state.done = true; return Ok(None); }
            };

            // Only send query params on the first page. Subsequent pages
            // use the full URL from the Link header which already includes params.
            let mut request = self.inner.client.get(url)
                .headers(self.auth_headers());
            if state.is_first_page {
                request = request.query(&params);
                state.is_first_page = false;
            }
            let response = request.send().await?;

            state.next_url = parse_link_header_next(&response);
            if state.next_url.is_none() { state.done = true; }

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
```

Consumer usage:
```rust
let stream = api.list_models(&ListModelsParams::builder().author("meta-llama").build());
pin_mut!(stream);
while let Some(model) = stream.next().await {
    println!("{}", model?.id);
}
```

## Xet Integration

### Feature Flag

The `xet` feature enables the `hf-xet` dependency (git dependency from `huggingface/xet-core`). All xet code is behind `#[cfg(feature = "xet")]`.

### Download Flow

1. `download_file()` sends a HEAD request to resolve file metadata
2. Checks response headers for `X-Xet-Hash`, `X-Xet-Refresh-Route`
3. If xet headers present:
   - With `xet` feature: call xet download with CAS endpoint + token
   - Without `xet` feature: return `HfError::XetNotEnabled`
4. If no xet headers: standard HTTP GET, stream response body to file

### Upload Flow

1. `upload_file()` / `upload_folder()` / `create_commit()` call Hub API to negotiate transfer protocol
2. If xet required:
   - With `xet` feature: fetch xet write token, call xet upload
   - Without `xet` feature: return `HfError::XetNotEnabled`
3. If regular: multipart upload via Hub API

No LFS upload path — the server only responds with xet or regular transfer.

### Internal Types

```rust
pub(crate) struct XetConnectionInfo {
    pub endpoint: String,
    pub access_token: String,
    pub expiration_unix_epoch: u64,
}
```

Token refresh is handled by re-fetching connection info from the Hub API when tokens expire.

## Out of Scope (v1)

The following features are deferred to future versions. Each can be added as additional methods on `HfApi` or new feature flags without breaking the v1 API.

### Feature Categories

1. **Spaces management** — get/set runtime, secrets, variables, pause, restart, dev mode, hardware requests, duplicate space, persistent storage
2. **Inference Endpoints** — create, list, get, update, delete, pause, resume, scale-to-zero, catalog
3. **Collections** — create, list, get, update, delete, add/remove/update items
4. **Discussions & Pull Requests** — create discussion, create PR, comment, edit comment, hide comment, merge PR, rename, change status
5. **Webhooks** — create, list, get, update, delete, enable, disable
6. **Jobs & Scheduled Jobs** — run job, run uv job, list, inspect, cancel, fetch logs, fetch metrics, list hardware, create/list/inspect/delete/suspend/resume scheduled jobs
7. **Access Requests** (gated repos) — list pending/accepted/rejected requests, accept, reject, cancel, grant access
8. **Buckets** (enterprise storage) — create, info, list, delete, move, list tree, get paths info, batch files, download, sync, get file metadata
9. **Likes & Interactions** — like, unlike, list liked repos, list repo likers
10. **LFS-specific operations** — preupload LFS files, permanently delete LFS files, list LFS files
11. **Safetensors metadata** — get/parse safetensors metadata from remote and local files
12. **Papers** — list papers, list daily papers, paper info
13. **Download cache management** — full cache layout with symlinks, blob storage, refs, etag-based deduplication (matching Python library's `~/.cache/huggingface/hub` structure)
14. **Sync (blocking) interface** — `sync_` prefixed methods using `reqwest::blocking::Client`, behind a `sync` feature flag
15. **Organization followers** — list organization followers
