# hfrs

A Rust-native CLI for the [Hugging Face Hub](https://huggingface.co), mirroring the command surface of the Python `hf` CLI. Built on top of the [`huggingface-hub`](../huggingface_hub/) Rust crate.

## Installation

```sh
cargo install --path hfrs
```

## Usage

```sh
# Download a file from a model repository
hfrs download gpt2 config.json

# Download an entire repository snapshot
hfrs download gpt2

# Upload a file to a repository
hfrs upload my-org/my-model ./weights.bin weights.bin

# Upload a folder to a repository
hfrs upload my-org/my-model ./model-dir

# List trending models
hfrs models list --limit 10

# Get info about a model
hfrs models info gpt2

# Authenticate with a token
hfrs auth login

# Show current user
hfrs auth whoami

# Print version
hfrs version

# Print runtime environment
hfrs env
```

Global flags available on every command:

```sh
--token <TOKEN>      # or set HF_TOKEN env var
--endpoint <URL>     # or set HF_ENDPOINT env var
--format table|json  # default: table
```

## Implemented Commands

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

## Skipped Commands

These commands are not implemented because they have no backing support in the `huggingface-hub` Rust crate.

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

## Token Configuration

Tokens are resolved in this order:

1. `--token <TOKEN>` CLI flag (highest priority)
2. `HF_TOKEN` environment variable
3. Active token from `~/.cache/huggingface/stored_tokens`
4. Token from `~/.cache/huggingface/token`

The token file paths respect `$HF_HOME` if set (defaults to `~/.cache/huggingface`).

To save a token for repeated use:

```sh
hfrs auth login
# Enter your token at the prompt. It is saved to stored_tokens.

hfrs auth whoami
# Verify the token works.
```

To switch between multiple saved tokens:

```sh
hfrs auth list           # list saved token names
hfrs auth switch <name>  # set a different token as active
```

## Running Tests

### Unit Tests

```sh
cargo test -p hfrs
```

### Integration Tests

Integration tests run `hfrs` against the live Hub API and compare output with the Python `hf` CLI.

Requirements:

- `HF_TOKEN` must be set to a valid Hugging Face token
- `hf` (Python CLI) must be on `$PATH` for cross-CLI comparison tests

Run read-only integration tests:

```sh
HF_TOKEN=hf_xxx cargo test -p hfrs --test cli_comparison
```

Run tests that include write operations (create/delete repos, upload files):

```sh
HF_TOKEN=hf_xxx HF_TEST_WRITE=1 cargo test -p hfrs --test cli_comparison
```

Tests hard-fail with a clear message if a required CLI is missing or `HF_TOKEN` is not set.
