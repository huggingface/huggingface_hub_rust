# HFRepository / HFRepo Design

## Summary

This document proposes a repository-scoped handle for the Rust client:

- Rename the top-level client to `HFClient`
- Introduce `HFRepository` with `HFRepo` as a type alias
- Move repository interactions onto `HFRepository` instances
- Keep `HFClient` responsible for global APIs and for creating repo handles

The recommended shape is for `HFRepository` to own a cheap clone of `HFClient`, not a borrowed reference. `HFClient` already wraps shared state in an `Arc`, so cloning it into repo handles is cheap and avoids lifetime-heavy APIs.

## Goals

- Make repository workflows read naturally: "get a repo, then act on it"
- Eliminate repeated repository identity and `repo_type` fields from repo-scoped calls
- Keep async ergonomics simple by avoiding borrowed-handle lifetimes
- Allow a staged migration without breaking the full surface area at once

## Non-Goals

- Moving global listing/search endpoints like `list_models` off the client
- Collapsing every feature into `HFRepository` in the first migration
- Removing `HfApi` compatibility aliases in the same change

## Recommendation

Use both patterns, but make one clearly primary:

- `HFClient` creates repo handles through constructors like `repo`, `model`, `dataset`, and `space`
- `HFRepository` stores an owned `HFClient` clone plus repository identity

That gives us the ergonomics of client-created handles and the implementation simplicity of owned state.

## Why `HFRepository` Should Own `HFClient`

### Recommended

```rust
#[derive(Clone)]
pub struct HFRepository {
    client: HFClient,
    owner: String,
    name: String,
    repo_type: RepoType,
    default_revision: Option<String>,
}
```

Benefits:

- No lifetimes on `HFRepository`
- Easy to return from `HFClient` methods
- Easy to clone, store, and pass into async tasks
- Matches the current `Arc`-backed client design

### Not Recommended

```rust
pub struct HFRepository<'a> {
    client: &'a HFClient,
    owner: String,
    name: String,
    repo_type: RepoType,
}
```

Costs:

- Lifetimes leak into the whole public API
- Harder to store inside structs
- More friction with spawned async work
- No meaningful memory win because `HFClient` is already cheap to clone

## Proposed API

### Client Constructors

```rust
impl HFClient {
    pub fn repo(
        &self,
        repo_type: RepoType,
        owner: impl Into<String>,
        name: impl Into<String>,
    ) -> HFRepository;
    pub fn model(&self, owner: impl Into<String>, name: impl Into<String>) -> HFRepository;
    pub fn dataset(&self, owner: impl Into<String>, name: impl Into<String>) -> HFRepository;
    pub fn space(&self, owner: impl Into<String>, name: impl Into<String>) -> HFRepository;
}
```

The typed constructors should be the common path. `repo(...)` remains available for dynamic call sites.

### Repository Handle

```rust
pub struct HFRepository {
    client: HFClient,
    owner: String,
    name: String,
    repo_type: RepoType,
    default_revision: Option<String>,
}

pub type HFRepo = HFRepository;
```

Basic identity and handle helpers:

```rust
impl HFRepository {
    pub fn new(
        client: HFClient,
        repo_type: RepoType,
        owner: impl Into<String>,
        name: impl Into<String>,
    ) -> Self;
    pub fn owner(&self) -> &str;
    pub fn name(&self) -> &str;
    pub fn repo_path(&self) -> String;
    pub fn repo_type(&self) -> RepoType;
    pub fn with_revision(&self, revision: impl Into<String>) -> Self;
    pub fn without_revision(&self) -> Self;
}
```

`with_revision` should return a cloned handle with a different default revision. That keeps the handle immutable and cheap to reuse.
`repo_path()` returns the Hub path form, `"owner/name"`, for internal request building and compatibility layers.

## Method Placement

### Stay On `HFClient`

- `repo(...)` with required `RepoType`
- `create_repo`
- `delete_repo`
- `move_repo`
- `list_models`
- `list_datasets`
- `list_spaces`
- `whoami`
- user and organization queries
- collection endpoints not tied to one repository
- other cross-repo or account-level APIs

### Move To `HFRepository`

- repo metadata and existence checks
- file existence, listing, path info
- download and upload operations
- snapshot download
- create commit
- commit history and diffs
- branch and tag operations
- repo-local settings updates
- repo-scoped feature APIs like discussions, likes, access requests, and space runtime operations

The rule is:

- repository handles own operations on an already-selected repository
- client methods own lifecycle operations that create, delete, or rename repositories

## Parameter Strategy

Today many params include `repo_id` and `repo_type`. With `HFRepository`, both the owner/name pair and `repo_type` become redundant on repo-scoped calls.

Recommended migration:

1. Keep existing client-level params and methods working.
2. Add repo-scoped param structs that remove repository identity and `repo_type`.
3. Implement repo methods first in terms of internal helpers, then let existing client methods delegate into them.

Example:

```rust
pub struct RepoInfoParams {
    pub revision: Option<String>,
}

pub struct DownloadParams {
    pub filename: String,
    pub local_dir: Option<PathBuf>,
    pub revision: Option<String>,
    pub force_download: Option<bool>,
    pub local_files_only: Option<bool>,
}
```

Revision resolution rule:

- Explicit revision in method params wins
- Otherwise use `HFRepository.default_revision`
- Otherwise fall back to `main`

## Example Usage

```rust
let client = HFClient::new()?;
let repo = client.model("openai-community", "gpt2");

let info = repo.info(&RepoInfoParams::default()).await?;
let config = repo
    .download_file(
        &DownloadParams::builder()
            .filename("config.json")
            .build(),
    )
    .await?;
```

Revision-pinned flow:

```rust
let repo = client.dataset("rajpurkar", "squad").with_revision("main");
let files = repo.list_files(&ListFilesParams::default()).await?;
```

## Migration Plan

### Phase 1

- Rename `HfApi` to `HFClient`
- Keep `HfApi` and `HfApiBuilder` as compatibility aliases

### Phase 2

- Introduce `HFRepository` and `HFRepo`
- Add `HFClient::{model,dataset,space,repo}`
- Add repo-scoped helpers for identity and revision defaults

### Phase 3

- Move core repo/file/commit methods onto `HFRepository`
- Add repo-scoped param types without repository identity and `repo_type`
- Have client methods delegate internally for compatibility

### Phase 4

- Expand repo handles to repo-specific features like discussions, likes, and space runtime operations
- Evaluate deprecating the old client-level repo methods

## Resolved Decisions

- `HFClient::repo(...)` requires an explicit `RepoType`; `model`, `dataset`, and `space` remain the ergonomic typed shortcuts
- `HFRepository::info()` should return a tagged enum representing model, dataset, or space info
- repository lifecycle operations such as create, delete, and move remain on `HFClient`
- repo-local mutation like `update_settings` belongs on `HFRepository`
- parsing helpers can exist at the compatibility layer, but the primary `HFRepository` API should expose `owner` and `name` as separate fields
