# hfrs

A Rust-native CLI for the [Hugging Face Hub](https://huggingface.co), mirroring the command surface of the Python `hf` CLI. Built on top of the [`huggingface-hub`](../huggingface_hub/) Rust crate with Xet high-performance transfer support.

## Installation

```sh
cargo install --path hfrs
```

## Usage

```sh
# Download a model
hfrs download meta-llama/Llama-3.2-1B-Instruct

# Download a specific file
hfrs download gpt2 config.json

# Upload a file (creates the repo if it doesn't exist)
hfrs upload my-org/my-model ./weights.bin weights.bin

# Upload a folder to a private repo
hfrs upload my-org/my-model ./model-dir --private

# List trending models
hfrs models list --limit 10

# List models as JSON
hfrs models list --limit 5 --format json

# Get info about a model
hfrs models info gpt2

# Search for datasets
hfrs datasets list --search squad --limit 5

# Authenticate
hfrs auth login --token-value hf_xxx
hfrs auth whoami

# Manage repos
hfrs repos create my-org/new-model --private
hfrs repos tag create my-org/my-model v1.0
hfrs repos delete my-org/old-model

# Manage discussions
hfrs discussions list my-org/my-model
hfrs discussions create my-org/my-model --title "Bug report"

# Print version and environment
hfrs version
hfrs env
```

## Global Options

Available on every command:

| Flag | Env Var | Description |
|---|---|---|
| `--token <TOKEN>` | `HF_TOKEN` | Authentication token |
| `--endpoint <URL>` | `HF_ENDPOINT` | API endpoint override |
| `--no-color` | `NO_COLOR=1` | Disable colored output |

Output options (`--format`, `--quiet`) are available on list and info subcommands.

## Implemented Commands

| Command Group | Subcommands |
|---|---|
| `auth` | `login`, `logout`, `switch`, `list`, `whoami` |
| `models` | `info`, `list` |
| `datasets` | `info`, `list` |
| `spaces` | `info`, `list` |
| `repos` | `create`, `delete`, `move`, `settings`, `delete-files` |
| `repos branch` | `create`, `delete` |
| `repos tag` | `create`, `delete`, `list` |
| `download` | Single file or snapshot download with `--cache-dir` support |
| `upload` | File or folder upload, auto-creates repo with `--private` |
| `discussions` | `list`, `info`, `create`, `comment`, `merge`, `close`, `reopen`, `rename`, `diff` |
| `collections` | `info`, `list`, `create`, `delete`, `update`, `add-item`, `update-item`, `delete-item` |
| `webhooks` | `list`, `info`, `create`, `update`, `delete`, `enable`, `disable` |
| `endpoints` | `list`, `describe`, `deploy`, `delete`, `pause`, `resume`, `scale-to-zero`, `update` |
| `jobs` | `run`, `ps`, `inspect`, `cancel`, `logs`, `hardware`, `stats` |
| `jobs scheduled` | `run`, `ps`, `inspect`, `delete`, `suspend`, `resume` |
| `papers` | `info`, `list`, `search` |
| `likes` | `like`, `unlike`, `list` |
| `access-requests` | `list`, `accept`, `reject`, `cancel`, `grant` |
| `cache` | `list`, `rm` |
| `env` | Print runtime environment and configuration |
| `version` | Print the hfrs version |

## Skipped Commands

Not implemented because they have no backing support in the `huggingface-hub` Rust crate:

| Command | Reason |
|---|---|
| `buckets` (all) | No bucket API in crate |
| `sync` | No bucket sync in crate |
| `upload-large-folder` | No resumable chunked upload in crate |
| `datasets sql` | Requires DuckDB, not in crate |
| `datasets parquet` | Requires dataset viewer API, not in crate |
| `spaces dev-mode` | Not in crate |
| `spaces hot-reload` | Not in crate |
| `jobs uv` / `jobs scheduled uv` | UV script runner, not in crate |
| `cache prune` / `cache verify` | Not in crate |
| `extensions` / `skills` | Plugin system, out of scope |
| `endpoints catalog` | Not in crate |
| `repos duplicate` | Not in crate |

## Token Configuration

Tokens are resolved in this order:

1. `--token <TOKEN>` CLI flag (highest priority)
2. `HF_TOKEN` environment variable
3. Active token from `~/.cache/huggingface/stored_tokens`
4. Token from `~/.cache/huggingface/token`

Token file paths respect `$HF_HOME` and `$HF_TOKEN_PATH` if set.

```sh
# Save a token
hfrs auth login --token-value hf_xxx --token-name default

# List saved tokens
hfrs auth list

# Switch between tokens
hfrs auth switch work-token

# Check who you're logged in as
hfrs auth whoami
```

## Error Handling

Errors match the style of the Python `hf` CLI:

```
Error: Repository 'nonexistent-model' not found. If the repo is private, make sure you are authenticated.
Set HF_DEBUG=1 for the full error trace.
```

Set `HF_DEBUG=1` to see the full error chain for debugging.

## Logging

| Env Var | Effect |
|---|---|
| `HF_LOG_LEVEL=<level>` | Set log level: `error`, `warn`, `info`, `debug`, `trace` |
| `HF_DEBUG=1` | Shorthand for `debug` level + full error traces |

Default log level is `warn`. `HF_LOG_LEVEL` takes precedence over `HF_DEBUG`. Logs go to stderr.

Per-module filters are supported: `HF_LOG_LEVEL=hfrs=debug,hyper=warn`.

## Color

Colored output is enabled by default when running in a terminal.

| Method | Effect |
|---|---|
| `--no-color` | Disable color |
| `NO_COLOR=1` | Disable color ([no-color.org](https://no-color.org)) |
| `CLICOLOR_FORCE=1` | Force color even when piped |

Color applies to `--help` output, error messages, and tracing logs.

## Running Tests

### Read-Only Tests

```sh
cargo test -p hfrs --test cli_comparison -- --skip write_
```

Requires `HF_TOKEN` and `hf` (Python CLI) on `$PATH`. Tests hard-fail if either is missing.

### Write Tests

```sh
HF_TEST_WRITE=1 cargo test -p hfrs --test cli_comparison -- write_
```

Creates and deletes real repos, branches, tags, and discussions on the Hub. Requires `HF_TOKEN` with write access.

### All Tests

```sh
HF_TEST_WRITE=1 cargo test -p hfrs --test cli_comparison
```

43 tests total: 7 offline smoke tests, 26 read-only API tests, 5 cross-CLI comparison tests, 5 write tests.
