# huggingface-hub

Async Rust client for the [Hugging Face Hub API](https://huggingface.co/docs/hub/api).

`huggingface-hub` provides a typed, ergonomic interface for interacting with the Hugging Face Hub from Rust. It is the Rust equivalent of the Python [`huggingface_hub`](https://github.com/huggingface/huggingface_hub) library.

## Features

- **Repository operations** — query model, dataset, and space metadata; create, delete, update, and move repositories
- **File operations** — upload files and folders, download files, list repository trees, check file existence
- **Commit operations** — create commits with multiple file operations, list commit history, view diffs between revisions
- **Branch and tag management** — create and delete branches and tags, list refs
- **User and organization info** — whoami, user profiles, organization details, followers
- **Streaming pagination** — list endpoints return `impl Stream<Item = Result<T>>` for lazy, memory-efficient iteration
- **Xet high-performance transfers** — optional support for Hugging Face's Xet storage backend (behind the `xet` feature flag)

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
huggingface-hub = { git = "https://github.com/huggingface-internal/huggingface-hub-rs.git" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

To enable Xet high-performance transfers:

```toml
[dependencies]
huggingface-hub = { git = "https://github.com/huggingface-internal/huggingface-hub-rs.git", features = ["xet"] }
```

## Quick Start

```rust,no_run
use huggingface_hub::{HFClient, ModelInfoParams};

#[tokio::main]
async fn main() -> huggingface_hub::Result<()> {
    let api = HFClient::new()?;

    // Get model info
    let info = api.model_info(
        &ModelInfoParams::builder().repo_id("gpt2").build()
    ).await?;
    println!("Model: {} (downloads: {:?})", info.id, info.downloads);

    Ok(())
}
```

## Usage Examples

### List models by author

```rust,no_run
use futures::StreamExt;
use huggingface_hub::{HFClient, ListModelsParams};

#[tokio::main]
async fn main() -> huggingface_hub::Result<()> {
    let api = HFClient::new()?;

    let params = ListModelsParams::builder()
        .author("meta-llama")
        .limit(5_usize)
        .build();

    let stream = api.list_models(&params);
    futures::pin_mut!(stream);

    while let Some(model) = stream.next().await {
        let model = model?;
        println!("{}", model.id);
    }

    Ok(())
}
```

### Check if a file exists

```rust,no_run
use huggingface_hub::{FileExistsParams, HFClient};

#[tokio::main]
async fn main() -> huggingface_hub::Result<()> {
    let api = HFClient::new()?;

    let exists = api.file_exists(
        &FileExistsParams::builder()
            .repo_id("gpt2")
            .filename("config.json")
            .build()
    ).await?;

    println!("config.json exists: {exists}");
    Ok(())
}
```

### Download a file

```rust,no_run
use std::path::PathBuf;
use huggingface_hub::{DownloadFileParams, HFClient};

#[tokio::main]
async fn main() -> huggingface_hub::Result<()> {
    let api = HFClient::new()?;

    let path = api.download_file(
        &DownloadFileParams::builder()
            .repo_id("gpt2")
            .filename("config.json")
            .local_dir(PathBuf::from("/tmp/hf-downloads"))
            .build()
    ).await?;

    println!("Downloaded to: {}", path.display());
    Ok(())
}
```

### Work with a repository handle

```rust,no_run
use huggingface_hub::{HFClient, RepoFileExistsParams, RepoInfo, RepoInfoParams};

#[tokio::main]
async fn main() -> huggingface_hub::Result<()> {
    let client = HFClient::new()?;
    let repo = client.model("openai-community", "gpt2");

    match repo.info(&RepoInfoParams::default()).await? {
        RepoInfo::Model(info) => println!("Model: {}", info.id),
        _ => unreachable!(),
    }

    let exists = repo
        .file_exists(
            &RepoFileExistsParams::builder()
                .filename("config.json")
                .build(),
        )
        .await?;

    println!("config.json exists: {exists}");
    Ok(())
}
```

### Upload a file

```rust,no_run
use huggingface_hub::{AddSource, HFClient, UploadFileParams};

#[tokio::main]
async fn main() -> huggingface_hub::Result<()> {
    let api = HFClient::new()?;

    let commit = api.upload_file(
        &UploadFileParams::builder()
            .repo_id("your-username/your-repo")
            .source(AddSource::Bytes(b"Hello, world!".to_vec()))
            .path_in_repo("greeting.txt")
            .commit_message("Add greeting file")
            .build()
    ).await?;

    println!("Committed: {:?}", commit.oid);
    Ok(())
}
```

### Create a repository

```rust,no_run
use huggingface_hub::{CreateRepoParams, HFClient};

#[tokio::main]
async fn main() -> huggingface_hub::Result<()> {
    let api = HFClient::new()?;

    let url = api.create_repo(
        &CreateRepoParams::builder()
            .repo_id("your-username/new-model")
            .private(true)
            .exist_ok(true)
            .build()
    ).await?;

    println!("Repository URL: {}", url.url);
    Ok(())
}
```

## Authentication

The client resolves authentication tokens in this order:

1. Explicit token via `HFClientBuilder::token()`
2. `HF_TOKEN` environment variable
3. Token file at path specified by `HF_TOKEN_PATH`
4. Default token file at `~/.cache/huggingface/token`

Set `HF_HUB_DISABLE_IMPLICIT_TOKEN` to any non-empty value to disable automatic token resolution.

## Configuration

| Environment Variable | Description |
|---|---|
| `HF_ENDPOINT` | Hub API endpoint (default: `https://huggingface.co`) |
| `HF_TOKEN` | Authentication token |
| `HF_TOKEN_PATH` | Path to token file |
| `HF_HOME` | Cache directory root (default: `~/.cache/huggingface`) |
| `HF_HUB_DISABLE_IMPLICIT_TOKEN` | Disable automatic token loading |
| `HF_HUB_USER_AGENT_ORIGIN` | Custom User-Agent origin string |

## Error Handling

All fallible operations return `Result<T, HfError>`. The `HfError` enum provides structured variants for common failure modes:

- `HfError::AuthRequired` — 401 response, token is missing or invalid
- `HfError::RepoNotFound` — repository does not exist or is inaccessible
- `HfError::EntryNotFound` — file or path does not exist in the repository
- `HfError::RevisionNotFound` — branch, tag, or commit does not exist
- `HfError::XetNotEnabled` — xet transfer required but `xet` feature is not enabled
- `HfError::Http` — other HTTP errors with status code, URL, and response body

## License

Apache-2.0
