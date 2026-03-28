# hfrs CLI Design Spec

A Rust CLI tool that mirrors the `hf` CLI provided by the Python `huggingface_hub` library. Built on top of the `huggingface-hub` Rust crate using `clap` with derive macros.

## Goals

1. Provide a Rust-native CLI (`hfrs`) with the same command surface as `hf` for all commands backed by the `huggingface-hub` crate
2. Use clap derive to parse arguments into structs and enums
3. Support `--format json` and `--format table` output where applicable
4. Integration tests that run `hf`, `hfjs`, and `hfrs` side by side and compare JSON output
5. Document which commands are implemented and which are skipped

## Non-Goals

- Matching the exact table formatting of the Python CLI (we use our own table layout)
- Implementing commands that have no backing in the `huggingface-hub` crate (buckets, sync, etc.)
- Plugin/extension system

## Crate Structure

New crate at `hfrs/` in the workspace root, sibling to `huggingface_hub/`.

```
hfrs/
├── Cargo.toml
├── README.md                     # Coverage matrix, usage, skipped commands
├── src/
│   ├── main.rs                   # Entry point: parse args, build HfApi, dispatch
│   ├── cli.rs                    # Top-level Cli struct, Command enum, shared enums
│   ├── output.rs                 # CommandResult, OutputFormat, table/json rendering
│   ├── commands/
│   │   ├── mod.rs                # Re-exports all command modules
│   │   ├── auth/
│   │   │   ├── mod.rs            # AuthCommand enum
│   │   │   ├── login.rs
│   │   │   ├── logout.rs
│   │   │   ├── switch.rs
│   │   │   ├── list.rs
│   │   │   └── whoami.rs
│   │   ├── models/
│   │   │   ├── mod.rs
│   │   │   ├── info.rs
│   │   │   └── list.rs
│   │   ├── datasets/
│   │   │   ├── mod.rs
│   │   │   ├── info.rs
│   │   │   └── list.rs
│   │   ├── spaces/
│   │   │   ├── mod.rs
│   │   │   ├── info.rs
│   │   │   └── list.rs
│   │   ├── repos/
│   │   │   ├── mod.rs
│   │   │   ├── create.rs
│   │   │   ├── delete.rs
│   │   │   ├── move_repo.rs
│   │   │   ├── settings.rs
│   │   │   ├── delete_files.rs
│   │   │   ├── branch.rs         # branch create + delete
│   │   │   └── tag.rs            # tag create + delete + list
│   │   ├── download.rs           # Top-level download command
│   │   ├── upload.rs             # Top-level upload command
│   │   ├── discussions/
│   │   │   ├── mod.rs
│   │   │   ├── list.rs
│   │   │   ├── info.rs
│   │   │   ├── create.rs
│   │   │   ├── comment.rs
│   │   │   ├── merge.rs
│   │   │   ├── close.rs
│   │   │   ├── reopen.rs
│   │   │   ├── rename.rs
│   │   │   └── diff.rs
│   │   ├── collections/
│   │   │   ├── mod.rs
│   │   │   ├── info.rs
│   │   │   ├── list.rs
│   │   │   ├── create.rs
│   │   │   ├── delete.rs
│   │   │   ├── update.rs
│   │   │   ├── add_item.rs
│   │   │   ├── update_item.rs
│   │   │   └── delete_item.rs
│   │   ├── webhooks/
│   │   │   ├── mod.rs
│   │   │   ├── list.rs
│   │   │   ├── info.rs
│   │   │   ├── create.rs
│   │   │   ├── update.rs
│   │   │   ├── delete.rs
│   │   │   ├── enable.rs
│   │   │   └── disable.rs
│   │   ├── endpoints/
│   │   │   ├── mod.rs
│   │   │   ├── list.rs
│   │   │   ├── describe.rs
│   │   │   ├── deploy.rs
│   │   │   ├── delete.rs
│   │   │   ├── pause.rs
│   │   │   ├── resume.rs
│   │   │   ├── scale_to_zero.rs
│   │   │   └── update.rs
│   │   ├── jobs/
│   │   │   ├── mod.rs
│   │   │   ├── run.rs
│   │   │   ├── ps.rs
│   │   │   ├── inspect.rs
│   │   │   ├── cancel.rs
│   │   │   ├── logs.rs
│   │   │   ├── hardware.rs
│   │   │   ├── stats.rs
│   │   │   └── scheduled/
│   │   │       ├── mod.rs
│   │   │       ├── run.rs
│   │   │       ├── ps.rs
│   │   │       ├── inspect.rs
│   │   │       ├── delete.rs
│   │   │       ├── suspend.rs
│   │   │       └── resume.rs
│   │   ├── papers/
│   │   │   ├── mod.rs
│   │   │   ├── info.rs
│   │   │   ├── list.rs
│   │   │   └── search.rs
│   │   ├── likes/
│   │   │   ├── mod.rs
│   │   │   ├── like.rs
│   │   │   ├── unlike.rs
│   │   │   └── list.rs
│   │   ├── access_requests/
│   │   │   ├── mod.rs
│   │   │   ├── list.rs
│   │   │   ├── accept.rs
│   │   │   ├── reject.rs
│   │   │   ├── cancel.rs
│   │   │   └── grant.rs
│   │   ├── cache/
│   │   │   ├── mod.rs
│   │   │   ├── list.rs
│   │   │   └── rm.rs
│   │   ├── env.rs
│   │   └── version.rs
│   └── util/
│       ├── mod.rs
│       └── token.rs              # Token file read/write/switch/list
└── tests/
    ├── cli_comparison.rs         # Integration tests: hf vs hfjs vs hfrs
    └── helpers/
        └── mod.rs                # CliRunner, assert_json_equivalent, VOLATILE_FIELDS
```

## CLI Parsing

### Top-Level Struct

```rust
#[derive(Parser)]
#[command(name = "hfrs", about = "Hugging Face Hub CLI (Rust)")]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Authentication token
    #[arg(long, global = true, env = "HF_TOKEN")]
    token: Option<String>,

    /// API endpoint override
    #[arg(long, global = true, env = "HF_ENDPOINT")]
    endpoint: Option<String>,
}
```

### Command Enum

```rust
#[derive(Subcommand)]
enum Command {
    Auth(auth::AuthCommand),
    Cache(cache::CacheCommand),
    Collections(collections::CollectionsCommand),
    Datasets(datasets::DatasetsCommand),
    Discussions(discussions::DiscussionsCommand),
    Download(download::DownloadArgs),
    Endpoints(endpoints::EndpointsCommand),
    Jobs(jobs::JobsCommand),
    Likes(likes::LikesCommand),
    Models(models::ModelsCommand),
    Papers(papers::PapersCommand),
    #[command(alias = "repo")]
    Repos(repos::ReposCommand),
    Spaces(spaces::SpacesCommand),
    Upload(upload::UploadArgs),
    Webhooks(webhooks::WebhooksCommand),
    AccessRequests(access_requests::AccessRequestsCommand),
    Env(env::EnvArgs),
    Version(version::VersionArgs),
}
```

### Subcommand Pattern

Each command group follows this pattern:

```rust
// commands/repos/mod.rs
#[derive(Args)]
pub struct ReposCommand {
    #[command(subcommand)]
    pub command: ReposSubcommand,
}

#[derive(Subcommand)]
pub enum ReposSubcommand {
    Create(create::CreateArgs),
    Delete(delete::DeleteArgs),
    Move(move_repo::MoveArgs),
    Settings(settings::SettingsArgs),
    DeleteFiles(delete_files::DeleteFilesArgs),
    Branch(branch::BranchCommand),
    Tag(tag::TagCommand),
}
```

### Leaf Command Pattern

Each leaf command is a struct with clap derive fields, plus an `execute` method:

```rust
// commands/models/list.rs
#[derive(Args)]
pub struct ListArgs {
    #[arg(long)]
    search: Option<String>,
    #[arg(long)]
    author: Option<String>,
    #[arg(long)]
    filter: Vec<String>,
    #[arg(long, value_enum, default_value_t = SortOrder::TrendingScore)]
    sort: SortOrder,
    #[arg(long, default_value = "10")]
    limit: usize,
    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    format: OutputFormat,
    #[arg(short, long)]
    quiet: bool,
}

impl ListArgs {
    pub async fn execute(&self, api: &HfApi) -> anyhow::Result<CommandResult> {
        let params = ListModelsParams::builder()
            .search(self.search.as_deref())
            // ... map fields to params
            .build();
        let models: Vec<ModelInfo> = api.list_models(&params).collect().await?;
        // Build CommandResult from models
    }
}
```

### Shared Enums

Common enums used across multiple commands live in `cli.rs`:

```rust
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
    fn from(val: RepoTypeArg) -> Self { ... }
}
```

## Output System

### CommandResult

```rust
pub enum CommandResult {
    /// Structured data with rendering preferences
    Formatted {
        output: CommandOutput,
        format: OutputFormat,
        quiet: bool,
    },
    /// Raw text output (diffs, env info, etc.)
    Raw(String),
    /// No output (delete, like, etc.)
    Silent,
}

pub struct CommandOutput {
    /// Column headers for table mode
    pub headers: Vec<String>,
    /// Rows of values (each row maps to headers by index)
    pub rows: Vec<Vec<String>>,
    /// Raw serializable data for JSON mode
    pub json_value: serde_json::Value,
    /// Values for quiet mode (typically IDs), one per line
    pub quiet_values: Vec<String>,
}
```

Each leaf command's `execute` method builds the `CommandResult` including the `format` and `quiet` values from its own parsed args. This keeps the rendering decision self-contained — `main.rs` just calls `render(result)` without needing to extract format info from the command.

### Rendering

In `main.rs`, after command execution:

- `CommandResult::Silent` — print nothing
- `CommandResult::Raw(s)` — print `s` to stdout
- `CommandResult::Formatted { quiet: true, .. }` — print `quiet_values` one per line
- `CommandResult::Formatted { format: Json, .. }` — print `json_value` via `serde_json::to_string_pretty`
- `CommandResult::Formatted { format: Table, .. }` — render `headers` + `rows` using `comfy-table`

For single-item responses (e.g., `models info`), the table renders as key-value pairs (two columns: field name, value).

### Table Rendering

Using `comfy-table` for table output. Our table format does not need to match the Python CLI's format — it is independent. Tables should be readable and well-aligned.

## Auth & Token Management

### Token Storage

Compatible with the Python `hf` CLI token file format:
- Active token: `~/.cache/huggingface/token` (or `$HF_HOME/token`)
- Multi-token store: `~/.cache/huggingface/stored_tokens` (JSON: maps token names to values with one marked active)

The exact format of `stored_tokens` will be verified against the Python CLI during implementation.

### Token Resolution Order

When building `HfApi`:
1. `--token` CLI flag (highest priority)
2. `HF_TOKEN` environment variable
3. Active token from `stored_tokens` / `token` file

### Auth Commands

| Command | Implementation |
|---|---|
| `auth login` | Write token to `stored_tokens`, optionally set as active |
| `auth logout` | Remove token from `stored_tokens` by name |
| `auth switch` | Set a different stored token as active |
| `auth list` | List stored token names with masked values |
| `auth whoami` | Call `api.whoami()`, display user info |

All except `whoami` are local file operations in `util/token.rs`.

## Command Coverage

### Implemented Commands

| Command Group | Subcommands | Backing API |
|---|---|---|
| `auth` | `login`, `logout`, `switch`, `list`, `whoami` | Local file I/O + `api.whoami()` |
| `models` | `info`, `list` | `model_info`, `list_models` |
| `datasets` | `info`, `list` | `dataset_info`, `list_datasets` |
| `spaces` | `info`, `list` | `space_info`, `list_spaces` |
| `repos` | `create`, `delete`, `move`, `settings`, `delete-files` | `create_repo`, `delete_repo`, `move_repo`, `update_repo_settings`, `create_commit` |
| `repos branch` | `create`, `delete` | `create_branch`, `delete_branch` |
| `repos tag` | `create`, `delete`, `list` | `create_tag`, `delete_tag`, `list_repo_refs` |
| `download` | *(top-level)* | `download_file`, `snapshot_download` |
| `upload` | *(top-level)* | `upload_file`, `upload_folder` |
| `discussions` | `list`, `info`, `create`, `comment`, `merge`, `close`, `reopen`, `rename`, `diff` | Full discussions API |
| `collections` | `info`, `list`, `create`, `delete`, `update`, `add-item`, `update-item`, `delete-item` | Full collections API |
| `webhooks` | `list`, `info`, `create`, `update`, `delete`, `enable`, `disable` | Full webhooks API |
| `endpoints` | `list`, `describe`, `deploy`, `delete`, `pause`, `resume`, `scale-to-zero`, `update` | Full inference endpoints API |
| `jobs` | `run`, `ps`, `inspect`, `cancel`, `logs`, `hardware`, `stats` | Full jobs API |
| `jobs scheduled` | `run`, `ps`, `inspect`, `delete`, `suspend`, `resume` | Full scheduled jobs API |
| `papers` | `info`, `list`, `search` | `paper_info`, `list_daily_papers`, `list_papers` |
| `likes` | `like`, `unlike`, `list` | `like`, `unlike`, `list_liked_repos` |
| `access-requests` | `list`, `accept`, `reject`, `cancel`, `grant` | Full access requests API |
| `cache` | `list`, `rm` | `scan_cache`, `delete_cache_revisions` |
| `env` | *(top-level)* | Print runtime environment info |
| `version` | *(top-level)* | Print `hfrs` version |

### Skipped Commands

These commands are not implemented because they have no backing support in the `huggingface-hub` Rust crate. They are documented in the README.

| Command | Reason |
|---|---|
| `buckets` (all subcommands) | No Rust crate support for bucket API |
| `sync` | No Rust crate support for bucket sync |
| `upload-large-folder` | No resumable chunked upload support in crate |
| `datasets sql` | Requires DuckDB integration, not in crate |
| `datasets parquet` | Requires dataset viewer API, not in crate |
| `spaces dev-mode` | Not in crate |
| `spaces hot-reload` | Not in crate |
| `jobs uv` | UV script runner, not in crate |
| `jobs scheduled uv` | UV script runner, not in crate |
| `cache prune` | Not in crate |
| `cache verify` | Not in crate |
| `extensions` / `ext` | Plugin system, out of scope |
| `skills` | Plugin system, out of scope |
| `endpoints catalog` | Not in crate |
| `repos duplicate` | Not in crate |

## Entry Point Flow

```rust
// main.rs (pseudocode)

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Build HfApi with token resolution
    let mut builder = HfApi::builder();
    if let Some(token) = &cli.token {
        builder = builder.token(token);
    } else if let Some(token) = util::token::read_active_token() {
        builder = builder.token(token);
    }
    if let Some(endpoint) = &cli.endpoint {
        builder = builder.endpoint(endpoint);
    }
    let api = builder.build()?;

    // Dispatch to command
    let result = match cli.command {
        Command::Models(cmd) => cmd.execute(&api).await?,
        Command::Datasets(cmd) => cmd.execute(&api).await?,
        // ... etc for all commands
        Command::Auth(cmd) => cmd.execute(&api).await?,
        Command::Env(args) => args.execute().await?,
        Command::Version(args) => args.execute()?,
    };

    // Render output (format/quiet are embedded in CommandResult)
    render(result)?;

    Ok(())
}
```

## Integration Test Harness

### Architecture

Tests live at `hfrs/tests/cli_comparison.rs` with shared utilities in `hfrs/tests/helpers/mod.rs`.

### CliRunner

```rust
pub struct CliRunner {
    bin: String,           // "hf", "hfrs", or "hfjs"
    token: Option<String>,
}

impl CliRunner {
    pub fn new(bin: &str) -> Self { ... }

    /// Run a command with --format json, return parsed JSON
    pub fn run_json(&self, args: &[&str]) -> anyhow::Result<serde_json::Value> {
        // Build Command, add --token if set, add --format json
        // Execute, capture stdout, parse as JSON
    }

    /// Check if the binary is available on PATH
    pub fn is_available(&self) -> bool { ... }
}
```

### JSON Comparison

```rust
/// Fields that change between calls and should be ignored
pub const VOLATILE_FIELDS: &[&str] = &[
    "downloads",
    "downloadsAllTime",
    "trendingScore",
    "lastModified",
    "likes",
    "sha",
];

/// Assert two JSON values are structurally equivalent,
/// ignoring specified fields at any depth
pub fn assert_json_equivalent(
    actual: &serde_json::Value,
    expected: &serde_json::Value,
    ignore_fields: &[&str],
) {
    // Recursively compare, skipping keys in ignore_fields
    // On mismatch, produce a clear diff showing the path and values
}
```

### Test Requirements

Tests **hard-fail** (panic with a clear message) if:
- `hf` is not on `$PATH`
- `hfjs` is not on `$PATH`
- `HF_TOKEN` environment variable is not set

Write-operation tests additionally require `HF_TEST_WRITE=1`.

### Test Pattern

```rust
#[test]
fn models_info_matches_hf() {
    let hfrs = CliRunner::hfrs();  // uses CARGO_BIN_EXE_hfrs
    let hf = CliRunner::new("hf");
    require_cli(&hf);              // panics if not available
    require_token();               // panics if HF_TOKEN not set

    let hfrs_out = hfrs.run_json(&["models", "info", "gpt2"]).unwrap();
    let hf_out = hf.run_json(&["models", "info", "gpt2"]).unwrap();

    assert_json_equivalent(&hfrs_out, &hf_out, VOLATILE_FIELDS);
}
```

Each implemented command gets at least one comparison test against `hf` and one against `hfjs` (where `hfjs` supports the command).

## Dependencies

### Runtime

| Crate | Purpose |
|---|---|
| `huggingface-hub` (path, all features) | Core API client |
| `clap` (derive feature) | CLI argument parsing |
| `tokio` (full) | Async runtime |
| `serde` (derive) | Serialization |
| `serde_json` | JSON output |
| `comfy-table` | Table rendering |
| `dirs` | Home directory for token files |
| `anyhow` | Error handling in the binary |

### Dev Dependencies

| Crate | Purpose |
|---|---|
| `assert_cmd` | Running the built binary in integration tests |
| `serde_json` | JSON comparison in tests |

## Error Handling

The binary uses `anyhow::Result` throughout. The underlying `HfError` from `huggingface-hub` renders through anyhow's display chain, providing context like "model 'nonexistent' not found" rather than raw HTTP status codes.

Commands should provide actionable error messages:
- Missing token: "No authentication token found. Run `hfrs auth login` or set HF_TOKEN."
- Not found: "Model 'foo/bar' not found."
- Permission denied: "Permission denied. Check your token has the required scopes."

## README

The `hfrs/README.md` includes:
- Installation instructions
- Usage examples matching the `hf` CLI examples
- Full command coverage matrix (implemented vs skipped, with reasons for skipped)
- Token configuration guide
- How to run tests (including integration test requirements)
