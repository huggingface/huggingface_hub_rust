# Feature Expansion Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add 9 feature-gated API modules to the huggingface-hub Rust client: Spaces management, Inference Endpoints, Collections, Discussions & PRs, Webhooks, Jobs, Access Requests, Likes, and Papers.

**Architecture:** Each feature is an independent module behind a cargo feature flag. All features follow the existing pattern: types in `types/{feature}.rs`, params in `types/params.rs`, API methods in `api/{feature}.rs`, feature-gated in `lib.rs`. No features require external crate dependencies — the feature flags purely gate compilation of the module code. All features reuse the existing `HfApi` client, auth headers, pagination, and error handling infrastructure.

**Tech Stack:** Rust, reqwest (via reqwest-middleware), serde, typed-builder, tokio, futures

---

## Common Pattern Reference

Every feature follows this identical structure. Read this first.

### File Layout Per Feature

```
huggingface_hub/src/types/{feature}.rs   — Response/data structs (Deserialize)
huggingface_hub/src/types/params.rs      — Params structs (TypedBuilder) — append to existing file
huggingface_hub/src/types/mod.rs         — Add `pub mod {feature}; pub use {feature}::*;` behind cfg
huggingface_hub/src/api/{feature}.rs     — `impl HfApi { ... }` methods
huggingface_hub/src/api/mod.rs           — Add `pub mod {feature};` behind cfg
huggingface_hub/Cargo.toml               — Add feature flag: `{feature} = []`
huggingface_hub/tests/integration_test.rs — Integration tests (append)
```

### Conventions

- All response types: `#[derive(Debug, Clone, Deserialize)]` with `#[serde(rename_all = "camelCase")]`
- All params: `#[derive(TypedBuilder)]` with required fields using `#[builder(setter(into))]` and optional fields using `#[builder(default, setter(into, strip_option))]`
- Feature gating: `#[cfg(feature = "xyz")]` on module declarations in `lib.rs`, `types/mod.rs`, `api/mod.rs`, and on params/integration tests
- API methods: `&self` on `HfApi`, async, return `Result<T>`
- Paginated endpoints: return `impl Stream<Item = Result<T>> + '_` using `self.paginate(url, query)`
- Error mapping: use `self.check_response(response, repo_id, NotFoundContext)` for all HTTP calls
- Integration tests: skip when `HF_TOKEN` is not set; write tests skip when `HF_TEST_WRITE` != "1"
- Unit tests: serde deserialization tests for response types, placed in `#[cfg(test)] mod tests` in the types file
- Format and lint after every feature: `cargo +nightly fmt && cargo clippy -p huggingface-hub --features {feature} -- -D warnings`

### Adding a Feature Flag (Cargo.toml)

Feature flags are empty (`[]`) since no external dependencies are needed. Add to the `[features]` section:

```toml
{feature} = []
```

### Module Gating Pattern

In `lib.rs`, `types/mod.rs`, `api/mod.rs`:

```rust
#[cfg(feature = "{feature}")]
pub mod {feature};
#[cfg(feature = "{feature}")]
pub use {feature}::*;  // only in types/mod.rs
```

In `types/params.rs`, wrap params structs:

```rust
#[cfg(feature = "{feature}")]
// ... struct definition
```

---

## Chunk 1: Spaces Management (Feature 1)

### Feature: `spaces`

Manage Space runtime, secrets, variables, hardware, pause/restart, sleep time, and duplication.

### API Endpoints

| Method | Python Equivalent | HTTP | Endpoint |
|--------|-------------------|------|----------|
| `get_space_runtime` | `get_space_runtime` | GET | `/api/spaces/{repo_id}/runtime` |
| `request_space_hardware` | `request_space_hardware` | POST | `/api/spaces/{repo_id}/hardware` |
| `set_space_sleep_time` | `set_space_sleep_time` | POST | `/api/spaces/{repo_id}/sleeptime` |
| `pause_space` | `pause_space` | POST | `/api/spaces/{repo_id}/pause` |
| `restart_space` | `restart_space` | POST | `/api/spaces/{repo_id}/restart` |
| `add_space_secret` | `add_space_secret` | POST | `/api/spaces/{repo_id}/secrets` |
| `delete_space_secret` | `delete_space_secret` | DELETE | `/api/spaces/{repo_id}/secrets` |
| `add_space_variable` | `add_space_variable` | POST | `/api/spaces/{repo_id}/variables` |
| `delete_space_variable` | `delete_space_variable` | DELETE | `/api/spaces/{repo_id}/variables` |
| `duplicate_space` | `duplicate_space` | POST | `/api/spaces/{repo_id}/duplicate` |

### Task 1.1: Types — `types/spaces.rs`

**Files:**
- Create: `huggingface_hub/src/types/spaces.rs`

- [ ] **Step 1: Create the types file with response structs and unit tests**

```rust
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpaceRuntime {
    pub stage: Option<String>,
    pub hardware: Option<String>,
    pub requested_hardware: Option<String>,
    pub sleep_time: Option<u64>,
    #[serde(default)]
    pub raw: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpaceVariable {
    pub key: String,
    pub value: Option<String>,
    pub description: Option<String>,
    pub updated_at: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_space_runtime_deserialize() {
        let json = r#"{"stage":"RUNNING","hardware":"cpu-basic","sleepTime":7200}"#;
        let runtime: SpaceRuntime = serde_json::from_str(json).unwrap();
        assert_eq!(runtime.stage.as_deref(), Some("RUNNING"));
        assert_eq!(runtime.hardware.as_deref(), Some("cpu-basic"));
        assert_eq!(runtime.sleep_time, Some(7200));
    }

    #[test]
    fn test_space_runtime_deserialize_minimal() {
        let json = r#"{"stage":"BUILDING"}"#;
        let runtime: SpaceRuntime = serde_json::from_str(json).unwrap();
        assert_eq!(runtime.stage.as_deref(), Some("BUILDING"));
        assert!(runtime.hardware.is_none());
    }

    #[test]
    fn test_space_variable_deserialize() {
        let json = r#"{"key":"MODEL_ID","value":"gpt2","description":"The model"}"#;
        let var: SpaceVariable = serde_json::from_str(json).unwrap();
        assert_eq!(var.key, "MODEL_ID");
        assert_eq!(var.value.as_deref(), Some("gpt2"));
    }
}
```

- [ ] **Step 2: Register module in `types/mod.rs`**

Add after existing modules:

```rust
#[cfg(feature = "spaces")]
pub mod spaces;
#[cfg(feature = "spaces")]
pub use spaces::*;
```

- [ ] **Step 3: Run unit tests**

Run: `cargo test -p huggingface-hub --features spaces -- spaces`
Expected: All tests pass.

### Task 1.2: Params — append to `types/params.rs`

**Files:**
- Modify: `huggingface_hub/src/types/params.rs`

- [ ] **Step 1: Add params structs for spaces feature**

Append to `types/params.rs`:

```rust
#[cfg(feature = "spaces")]
#[derive(TypedBuilder)]
pub struct GetSpaceRuntimeParams {
    #[builder(setter(into))]
    pub repo_id: String,
}

#[cfg(feature = "spaces")]
#[derive(TypedBuilder)]
pub struct RequestSpaceHardwareParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub hardware: String,
    #[builder(default, setter(into, strip_option))]
    pub sleep_time: Option<u64>,
}

#[cfg(feature = "spaces")]
#[derive(TypedBuilder)]
pub struct SetSpaceSleepTimeParams {
    #[builder(setter(into))]
    pub repo_id: String,
    pub sleep_time: u64,
}

#[cfg(feature = "spaces")]
#[derive(TypedBuilder)]
pub struct PauseSpaceParams {
    #[builder(setter(into))]
    pub repo_id: String,
}

#[cfg(feature = "spaces")]
#[derive(TypedBuilder)]
pub struct RestartSpaceParams {
    #[builder(setter(into))]
    pub repo_id: String,
}

#[cfg(feature = "spaces")]
#[derive(TypedBuilder)]
pub struct AddSpaceSecretParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub key: String,
    #[builder(setter(into))]
    pub value: String,
    #[builder(default, setter(into, strip_option))]
    pub description: Option<String>,
}

#[cfg(feature = "spaces")]
#[derive(TypedBuilder)]
pub struct DeleteSpaceSecretParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub key: String,
}

#[cfg(feature = "spaces")]
#[derive(TypedBuilder)]
pub struct AddSpaceVariableParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub key: String,
    #[builder(setter(into))]
    pub value: String,
    #[builder(default, setter(into, strip_option))]
    pub description: Option<String>,
}

#[cfg(feature = "spaces")]
#[derive(TypedBuilder)]
pub struct DeleteSpaceVariableParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub key: String,
}

#[cfg(feature = "spaces")]
#[derive(TypedBuilder)]
pub struct DuplicateSpaceParams {
    #[builder(setter(into))]
    pub from_id: String,
    #[builder(default, setter(into, strip_option))]
    pub to_id: Option<String>,
    #[builder(default, setter(strip_option))]
    pub private: Option<bool>,
    #[builder(default, setter(into, strip_option))]
    pub hardware: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub storage: Option<String>,
    #[builder(default, setter(strip_option))]
    pub sleep_time: Option<u64>,
    #[builder(default, setter(into, strip_option))]
    pub secrets: Option<Vec<serde_json::Value>>,
    #[builder(default, setter(into, strip_option))]
    pub variables: Option<Vec<serde_json::Value>>,
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p huggingface-hub --features spaces`
Expected: Compiles cleanly.

### Task 1.3: API Module — `api/spaces.rs`

**Files:**
- Create: `huggingface_hub/src/api/spaces.rs`
- Modify: `huggingface_hub/src/api/mod.rs`

- [ ] **Step 1: Create the API implementation file**

```rust
use crate::client::HfApi;
use crate::error::Result;
use crate::types::{
    AddSpaceSecretParams, AddSpaceVariableParams, DeleteSpaceSecretParams,
    DeleteSpaceVariableParams, DuplicateSpaceParams, GetSpaceRuntimeParams,
    PauseSpaceParams, RequestSpaceHardwareParams, RestartSpaceParams,
    RepoUrl, SetSpaceSleepTimeParams, SpaceRuntime,
};

impl HfApi {
    /// Get runtime information about a Space.
    /// Endpoint: GET /api/spaces/{repo_id}/runtime
    pub async fn get_space_runtime(&self, params: &GetSpaceRuntimeParams) -> Result<SpaceRuntime> {
        let url = format!(
            "{}/api/spaces/{}/runtime",
            self.inner.endpoint, params.repo_id
        );
        let response = self
            .inner
            .client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await?;
        let response = self
            .check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(response.json().await?)
    }

    /// Request specific hardware for a Space.
    /// Endpoint: POST /api/spaces/{repo_id}/hardware
    pub async fn request_space_hardware(
        &self,
        params: &RequestSpaceHardwareParams,
    ) -> Result<SpaceRuntime> {
        let url = format!(
            "{}/api/spaces/{}/hardware",
            self.inner.endpoint, params.repo_id
        );
        let mut body = serde_json::json!({ "flavor": params.hardware });
        if let Some(sleep_time) = params.sleep_time {
            body["sleepTime"] = serde_json::json!(sleep_time);
        }
        let response = self
            .inner
            .client
            .post(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;
        let response = self
            .check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(response.json().await?)
    }

    /// Set the sleep time for a Space.
    /// Endpoint: POST /api/spaces/{repo_id}/sleeptime
    pub async fn set_space_sleep_time(
        &self,
        params: &SetSpaceSleepTimeParams,
    ) -> Result<()> {
        let url = format!(
            "{}/api/spaces/{}/sleeptime",
            self.inner.endpoint, params.repo_id
        );
        let body = serde_json::json!({ "seconds": params.sleep_time });
        let response = self
            .inner
            .client
            .post(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;
        self.check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(())
    }

    /// Pause a Space.
    /// Endpoint: POST /api/spaces/{repo_id}/pause
    pub async fn pause_space(&self, params: &PauseSpaceParams) -> Result<SpaceRuntime> {
        let url = format!(
            "{}/api/spaces/{}/pause",
            self.inner.endpoint, params.repo_id
        );
        let response = self
            .inner
            .client
            .post(&url)
            .headers(self.auth_headers())
            .send()
            .await?;
        let response = self
            .check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(response.json().await?)
    }

    /// Restart a Space.
    /// Endpoint: POST /api/spaces/{repo_id}/restart
    pub async fn restart_space(&self, params: &RestartSpaceParams) -> Result<SpaceRuntime> {
        let url = format!(
            "{}/api/spaces/{}/restart",
            self.inner.endpoint, params.repo_id
        );
        let response = self
            .inner
            .client
            .post(&url)
            .headers(self.auth_headers())
            .send()
            .await?;
        let response = self
            .check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(response.json().await?)
    }

    /// Add or update a secret in a Space.
    /// Endpoint: POST /api/spaces/{repo_id}/secrets
    pub async fn add_space_secret(&self, params: &AddSpaceSecretParams) -> Result<()> {
        let url = format!(
            "{}/api/spaces/{}/secrets",
            self.inner.endpoint, params.repo_id
        );
        let mut body = serde_json::json!({
            "key": params.key,
            "value": params.value,
        });
        if let Some(ref desc) = params.description {
            body["description"] = serde_json::json!(desc);
        }
        let response = self
            .inner
            .client
            .post(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;
        self.check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(())
    }

    /// Delete a secret from a Space.
    /// Endpoint: DELETE /api/spaces/{repo_id}/secrets
    pub async fn delete_space_secret(&self, params: &DeleteSpaceSecretParams) -> Result<()> {
        let url = format!(
            "{}/api/spaces/{}/secrets",
            self.inner.endpoint, params.repo_id
        );
        let body = serde_json::json!({ "key": params.key });
        let response = self
            .inner
            .client
            .delete(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;
        self.check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(())
    }

    /// Add or update an environment variable in a Space.
    /// Endpoint: POST /api/spaces/{repo_id}/variables
    pub async fn add_space_variable(&self, params: &AddSpaceVariableParams) -> Result<()> {
        let url = format!(
            "{}/api/spaces/{}/variables",
            self.inner.endpoint, params.repo_id
        );
        let mut body = serde_json::json!({
            "key": params.key,
            "value": params.value,
        });
        if let Some(ref desc) = params.description {
            body["description"] = serde_json::json!(desc);
        }
        let response = self
            .inner
            .client
            .post(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;
        self.check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(())
    }

    /// Delete an environment variable from a Space.
    /// Endpoint: DELETE /api/spaces/{repo_id}/variables
    pub async fn delete_space_variable(&self, params: &DeleteSpaceVariableParams) -> Result<()> {
        let url = format!(
            "{}/api/spaces/{}/variables",
            self.inner.endpoint, params.repo_id
        );
        let body = serde_json::json!({ "key": params.key });
        let response = self
            .inner
            .client
            .delete(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;
        self.check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(())
    }

    /// Duplicate a Space.
    /// Endpoint: POST /api/spaces/{from_id}/duplicate
    pub async fn duplicate_space(&self, params: &DuplicateSpaceParams) -> Result<RepoUrl> {
        let url = format!(
            "{}/api/spaces/{}/duplicate",
            self.inner.endpoint, params.from_id
        );
        let mut body = serde_json::Map::new();
        if let Some(ref to_id) = params.to_id {
            body.insert("repository".into(), serde_json::json!(to_id));
        }
        if let Some(private) = params.private {
            body.insert("private".into(), serde_json::json!(private));
        }
        if let Some(ref hw) = params.hardware {
            body.insert("hardware".into(), serde_json::json!(hw));
        }
        if let Some(ref storage) = params.storage {
            body.insert("storage".into(), serde_json::json!(storage));
        }
        if let Some(sleep_time) = params.sleep_time {
            body.insert("sleepTime".into(), serde_json::json!(sleep_time));
        }
        if let Some(ref secrets) = params.secrets {
            body.insert("secrets".into(), serde_json::json!(secrets));
        }
        if let Some(ref variables) = params.variables {
            body.insert("variables".into(), serde_json::json!(variables));
        }
        let response = self
            .inner
            .client
            .post(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;
        let response = self
            .check_response(response, Some(&params.from_id), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(response.json().await?)
    }
}
```

- [ ] **Step 2: Register module in `api/mod.rs`**

Add: `#[cfg(feature = "spaces")] pub mod spaces;`

- [ ] **Step 3: Add feature to `Cargo.toml`**

Add to `[features]`: `spaces = []`

- [ ] **Step 4: Verify compilation**

Run: `cargo check -p huggingface-hub --features spaces`

- [ ] **Step 5: Format and lint**

Run: `cargo +nightly fmt && cargo clippy -p huggingface-hub --features spaces -- -D warnings`

### Task 1.4: Integration Tests

**Files:**
- Modify: `huggingface_hub/tests/integration_test.rs`

- [ ] **Step 1: Add integration tests for spaces**

Append to `integration_test.rs`:

```rust
#[cfg(feature = "spaces")]
#[tokio::test]
async fn test_get_space_runtime() {
    let Some(api) = api() else { return };
    let params = GetSpaceRuntimeParams::builder()
        .repo_id("huggingface/transformers-benchmarks")
        .build();
    let runtime = api.get_space_runtime(&params).await.unwrap();
    assert!(runtime.stage.is_some());
}
```

- [ ] **Step 2: Run integration tests**

Run: `HF_TOKEN=hf_xxx cargo test -p huggingface-hub --features spaces --test integration_test -- test_get_space_runtime`

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat: add spaces management feature module"
```

---

## Chunk 2: Inference Endpoints (Feature 2)

### Feature: `inference_endpoints`

Manage Inference Endpoints — create, list, get, update, delete, pause, resume, scale-to-zero.

### API Endpoints

| Method | HTTP | Endpoint |
|--------|------|----------|
| `create_inference_endpoint` | POST | `/api/inference-endpoints/endpoint` |
| `get_inference_endpoint` | GET | `/api/inference-endpoints/endpoint/{namespace}/{name}` |
| `list_inference_endpoints` | GET | `/api/inference-endpoints/endpoint/{namespace}` |
| `update_inference_endpoint` | PUT | `/api/inference-endpoints/endpoint/{namespace}/{name}` |
| `delete_inference_endpoint` | DELETE | `/api/inference-endpoints/endpoint/{namespace}/{name}` |
| `pause_inference_endpoint` | POST | `/api/inference-endpoints/endpoint/{namespace}/{name}/pause` |
| `resume_inference_endpoint` | POST | `/api/inference-endpoints/endpoint/{namespace}/{name}/resume` |
| `scale_to_zero_inference_endpoint` | POST | `/api/inference-endpoints/endpoint/{namespace}/{name}/scale-to-zero` |

### Task 2.1: Types — `types/inference_endpoints.rs`

**Files:**
- Create: `huggingface_hub/src/types/inference_endpoints.rs`

- [ ] **Step 1: Create the types file**

```rust
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InferenceEndpointInfo {
    pub name: String,
    pub namespace: Option<String>,
    #[serde(default)]
    pub status: InferenceEndpointStatus,
    pub url: Option<String>,
    pub model: Option<InferenceEndpointModel>,
    pub provider: Option<InferenceEndpointProvider>,
    #[serde(rename = "type")]
    pub endpoint_type: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    #[serde(default)]
    pub raw: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InferenceEndpointModel {
    pub repository: Option<String>,
    pub framework: Option<String>,
    pub revision: Option<String>,
    pub task: Option<String>,
    pub image: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InferenceEndpointProvider {
    pub vendor: Option<String>,
    pub region: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct InferenceEndpointStatus {
    pub state: Option<String>,
    pub message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inference_endpoint_info_deserialize() {
        let json = r#"{
            "name": "my-endpoint",
            "status": {"state": "running"},
            "url": "https://my-endpoint.endpoints.huggingface.cloud",
            "type": "protected"
        }"#;
        let info: InferenceEndpointInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.name, "my-endpoint");
        assert_eq!(info.status.state.as_deref(), Some("running"));
        assert_eq!(info.endpoint_type.as_deref(), Some("protected"));
    }

    #[test]
    fn test_inference_endpoint_info_minimal() {
        let json = r#"{"name":"test","status":{}}"#;
        let info: InferenceEndpointInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.name, "test");
        assert!(info.url.is_none());
    }
}
```

- [ ] **Step 2: Register module in `types/mod.rs`**

```rust
#[cfg(feature = "inference_endpoints")]
pub mod inference_endpoints;
#[cfg(feature = "inference_endpoints")]
pub use inference_endpoints::*;
```

### Task 2.2: Params — append to `types/params.rs`

**Files:**
- Modify: `huggingface_hub/src/types/params.rs`

- [ ] **Step 1: Add params structs**

```rust
#[cfg(feature = "inference_endpoints")]
#[derive(TypedBuilder)]
pub struct CreateInferenceEndpointParams {
    #[builder(setter(into))]
    pub name: String,
    #[builder(setter(into))]
    pub repository: String,
    #[builder(setter(into))]
    pub framework: String,
    #[builder(setter(into))]
    pub task: String,
    #[builder(setter(into))]
    pub accelerator: String,
    #[builder(setter(into))]
    pub instance_size: String,
    #[builder(setter(into))]
    pub instance_type: String,
    #[builder(setter(into))]
    pub region: String,
    #[builder(setter(into))]
    pub vendor: String,
    #[builder(default, setter(into, strip_option))]
    pub namespace: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(strip_option))]
    pub min_replica: Option<u32>,
    #[builder(default, setter(strip_option))]
    pub max_replica: Option<u32>,
    #[builder(default, setter(strip_option))]
    pub scale_to_zero_timeout: Option<u32>,
    #[builder(default, setter(into, strip_option))]
    pub endpoint_type: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub custom_image: Option<serde_json::Value>,
    #[builder(default, setter(into, strip_option))]
    pub secrets: Option<std::collections::HashMap<String, String>>,
}

#[cfg(feature = "inference_endpoints")]
#[derive(TypedBuilder)]
pub struct GetInferenceEndpointParams {
    #[builder(setter(into))]
    pub name: String,
    #[builder(default, setter(into, strip_option))]
    pub namespace: Option<String>,
}

#[cfg(feature = "inference_endpoints")]
#[derive(TypedBuilder)]
pub struct ListInferenceEndpointsParams {
    #[builder(default, setter(into, strip_option))]
    pub namespace: Option<String>,
}

#[cfg(feature = "inference_endpoints")]
#[derive(TypedBuilder)]
pub struct UpdateInferenceEndpointParams {
    #[builder(setter(into))]
    pub name: String,
    #[builder(default, setter(into, strip_option))]
    pub namespace: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub accelerator: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub instance_size: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub instance_type: Option<String>,
    #[builder(default, setter(strip_option))]
    pub min_replica: Option<u32>,
    #[builder(default, setter(strip_option))]
    pub max_replica: Option<u32>,
    #[builder(default, setter(strip_option))]
    pub scale_to_zero_timeout: Option<u32>,
    #[builder(default, setter(into, strip_option))]
    pub repository: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub framework: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub revision: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub task: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub custom_image: Option<serde_json::Value>,
    #[builder(default, setter(into, strip_option))]
    pub secrets: Option<std::collections::HashMap<String, String>>,
}

#[cfg(feature = "inference_endpoints")]
#[derive(TypedBuilder)]
pub struct DeleteInferenceEndpointParams {
    #[builder(setter(into))]
    pub name: String,
    #[builder(default, setter(into, strip_option))]
    pub namespace: Option<String>,
}

#[cfg(feature = "inference_endpoints")]
#[derive(TypedBuilder)]
pub struct PauseInferenceEndpointParams {
    #[builder(setter(into))]
    pub name: String,
    #[builder(default, setter(into, strip_option))]
    pub namespace: Option<String>,
}

#[cfg(feature = "inference_endpoints")]
#[derive(TypedBuilder)]
pub struct ResumeInferenceEndpointParams {
    #[builder(setter(into))]
    pub name: String,
    #[builder(default, setter(into, strip_option))]
    pub namespace: Option<String>,
}

#[cfg(feature = "inference_endpoints")]
#[derive(TypedBuilder)]
pub struct ScaleToZeroInferenceEndpointParams {
    #[builder(setter(into))]
    pub name: String,
    #[builder(default, setter(into, strip_option))]
    pub namespace: Option<String>,
}
```

### Task 2.3: API Module — `api/inference_endpoints.rs`

**Files:**
- Create: `huggingface_hub/src/api/inference_endpoints.rs`
- Modify: `huggingface_hub/src/api/mod.rs`

- [ ] **Step 1: Create the API file**

All methods use the Inference Endpoints API base: `https://api.endpoints.huggingface.cloud/v2/endpoint`. The namespace defaults to the authenticated user's username (fetched via `whoami` if not provided). Use `self.inner.endpoint` only for the Hub API; the IE API has a fixed base URL.

```rust
use crate::client::HfApi;
use crate::error::Result;
use crate::types::{
    CreateInferenceEndpointParams, DeleteInferenceEndpointParams,
    GetInferenceEndpointParams, InferenceEndpointInfo, ListInferenceEndpointsParams,
    PauseInferenceEndpointParams, ResumeInferenceEndpointParams,
    ScaleToZeroInferenceEndpointParams, UpdateInferenceEndpointParams,
};

const IE_API_BASE: &str = "https://api.endpoints.huggingface.cloud/v2/endpoint";

impl HfApi {
    async fn resolve_ie_namespace(&self, namespace: &Option<String>) -> Result<String> {
        match namespace {
            Some(ns) => Ok(ns.clone()),
            None => {
                let user = self.whoami().await?;
                Ok(user.username)
            }
        }
    }

    /// Create a new Inference Endpoint.
    pub async fn create_inference_endpoint(
        &self,
        params: &CreateInferenceEndpointParams,
    ) -> Result<InferenceEndpointInfo> {
        let ns = self.resolve_ie_namespace(&params.namespace).await?;
        let url = format!("{IE_API_BASE}/{ns}");
        let mut body = serde_json::json!({
            "name": params.name,
            "model": {
                "repository": params.repository,
                "framework": params.framework,
                "task": params.task,
            },
            "provider": {
                "vendor": params.vendor,
                "region": params.region,
            },
            "compute": {
                "accelerator": params.accelerator,
                "instanceSize": params.instance_size,
                "instanceType": params.instance_type,
            },
        });
        if let Some(ref revision) = params.revision {
            body["model"]["revision"] = serde_json::json!(revision);
        }
        if let Some(min) = params.min_replica {
            body["compute"]["scaling"] = serde_json::json!({"minReplica": min});
        }
        if let Some(max) = params.max_replica {
            body["compute"]["scaling"]["maxReplica"] = serde_json::json!(max);
        }
        if let Some(ref endpoint_type) = params.endpoint_type {
            body["type"] = serde_json::json!(endpoint_type);
        }
        if let Some(ref custom_image) = params.custom_image {
            body["model"]["image"] = custom_image.clone();
        }
        if let Some(ref secrets) = params.secrets {
            body["model"]["secrets"] = serde_json::json!(secrets);
        }
        let response = self
            .inner
            .client
            .post(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;
        let response = self
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    /// Get info about an Inference Endpoint.
    pub async fn get_inference_endpoint(
        &self,
        params: &GetInferenceEndpointParams,
    ) -> Result<InferenceEndpointInfo> {
        let ns = self.resolve_ie_namespace(&params.namespace).await?;
        let url = format!("{IE_API_BASE}/{ns}/{}", params.name);
        let response = self
            .inner
            .client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await?;
        let response = self
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    /// List Inference Endpoints.
    pub async fn list_inference_endpoints(
        &self,
        params: &ListInferenceEndpointsParams,
    ) -> Result<Vec<InferenceEndpointInfo>> {
        let ns = self.resolve_ie_namespace(&params.namespace).await?;
        let url = format!("{IE_API_BASE}/{ns}");
        let response = self
            .inner
            .client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await?;
        let response = self
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        let wrapper: serde_json::Value = response.json().await?;
        let items = wrapper
            .get("items")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let endpoints: Vec<InferenceEndpointInfo> = serde_json::from_value(serde_json::json!(items))?;
        Ok(endpoints)
    }

    /// Update an Inference Endpoint.
    pub async fn update_inference_endpoint(
        &self,
        params: &UpdateInferenceEndpointParams,
    ) -> Result<InferenceEndpointInfo> {
        let ns = self.resolve_ie_namespace(&params.namespace).await?;
        let url = format!("{IE_API_BASE}/{ns}/{}", params.name);
        let mut body = serde_json::Map::new();
        let mut compute = serde_json::Map::new();
        let mut model = serde_json::Map::new();
        if let Some(ref acc) = params.accelerator {
            compute.insert("accelerator".into(), serde_json::json!(acc));
        }
        if let Some(ref size) = params.instance_size {
            compute.insert("instanceSize".into(), serde_json::json!(size));
        }
        if let Some(ref itype) = params.instance_type {
            compute.insert("instanceType".into(), serde_json::json!(itype));
        }
        if params.min_replica.is_some() || params.max_replica.is_some() || params.scale_to_zero_timeout.is_some() {
            let mut scaling = serde_json::Map::new();
            if let Some(min) = params.min_replica {
                scaling.insert("minReplica".into(), serde_json::json!(min));
            }
            if let Some(max) = params.max_replica {
                scaling.insert("maxReplica".into(), serde_json::json!(max));
            }
            if let Some(timeout) = params.scale_to_zero_timeout {
                scaling.insert("scaleToZeroTimeout".into(), serde_json::json!(timeout));
            }
            compute.insert("scaling".into(), serde_json::json!(scaling));
        }
        if let Some(ref repo) = params.repository {
            model.insert("repository".into(), serde_json::json!(repo));
        }
        if let Some(ref fw) = params.framework {
            model.insert("framework".into(), serde_json::json!(fw));
        }
        if let Some(ref rev) = params.revision {
            model.insert("revision".into(), serde_json::json!(rev));
        }
        if let Some(ref task) = params.task {
            model.insert("task".into(), serde_json::json!(task));
        }
        if let Some(ref img) = params.custom_image {
            model.insert("image".into(), img.clone());
        }
        if let Some(ref secrets) = params.secrets {
            model.insert("secrets".into(), serde_json::json!(secrets));
        }
        if !compute.is_empty() {
            body.insert("compute".into(), serde_json::json!(compute));
        }
        if !model.is_empty() {
            body.insert("model".into(), serde_json::json!(model));
        }
        let response = self
            .inner
            .client
            .put(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;
        let response = self
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    /// Delete an Inference Endpoint.
    pub async fn delete_inference_endpoint(
        &self,
        params: &DeleteInferenceEndpointParams,
    ) -> Result<()> {
        let ns = self.resolve_ie_namespace(&params.namespace).await?;
        let url = format!("{IE_API_BASE}/{ns}/{}", params.name);
        let response = self
            .inner
            .client
            .delete(&url)
            .headers(self.auth_headers())
            .send()
            .await?;
        self.check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(())
    }

    /// Pause an Inference Endpoint.
    pub async fn pause_inference_endpoint(
        &self,
        params: &PauseInferenceEndpointParams,
    ) -> Result<InferenceEndpointInfo> {
        let ns = self.resolve_ie_namespace(&params.namespace).await?;
        let url = format!("{IE_API_BASE}/{ns}/{}/pause", params.name);
        let response = self
            .inner
            .client
            .post(&url)
            .headers(self.auth_headers())
            .send()
            .await?;
        let response = self
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    /// Resume an Inference Endpoint.
    pub async fn resume_inference_endpoint(
        &self,
        params: &ResumeInferenceEndpointParams,
    ) -> Result<InferenceEndpointInfo> {
        let ns = self.resolve_ie_namespace(&params.namespace).await?;
        let url = format!("{IE_API_BASE}/{ns}/{}/resume", params.name);
        let response = self
            .inner
            .client
            .post(&url)
            .headers(self.auth_headers())
            .send()
            .await?;
        let response = self
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    /// Scale an Inference Endpoint to zero replicas.
    pub async fn scale_to_zero_inference_endpoint(
        &self,
        params: &ScaleToZeroInferenceEndpointParams,
    ) -> Result<InferenceEndpointInfo> {
        let ns = self.resolve_ie_namespace(&params.namespace).await?;
        let url = format!("{IE_API_BASE}/{ns}/{}/scale-to-zero", params.name);
        let response = self
            .inner
            .client
            .post(&url)
            .headers(self.auth_headers())
            .send()
            .await?;
        let response = self
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }
}
```

- [ ] **Step 2: Register in `api/mod.rs`, add feature to `Cargo.toml`**
- [ ] **Step 3: Format, lint, verify compilation**

Run: `cargo +nightly fmt && cargo clippy -p huggingface-hub --features inference_endpoints -- -D warnings`

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: add inference_endpoints feature module"
```

---

## Chunk 3: Collections (Feature 3)

### Feature: `collections`

### API Endpoints

| Method | HTTP | Endpoint |
|--------|------|----------|
| `get_collection` | GET | `/api/collections/{slug}` |
| `list_collections` | GET | `/api/collections` |
| `create_collection` | POST | `/api/collections` |
| `update_collection_metadata` | PATCH | `/api/collections/{slug}` |
| `delete_collection` | DELETE | `/api/collections/{slug}` |
| `add_collection_item` | POST | `/api/collections/{slug}/items` |
| `update_collection_item` | PATCH | `/api/collections/{slug}/items/{item_id}` |
| `delete_collection_item` | DELETE | `/api/collections/{slug}/items/{item_id}` |

### Task 3.1: Types — `types/collections.rs`

**Files:**
- Create: `huggingface_hub/src/types/collections.rs`

- [ ] **Step 1: Create types file**

```rust
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub slug: String,
    pub title: Option<String>,
    pub owner: Option<String>,
    #[serde(default)]
    pub items: Vec<CollectionItem>,
    pub last_updated: Option<String>,
    pub position: Option<i64>,
    pub private: Option<bool>,
    pub theme: Option<String>,
    pub upvotes: Option<u64>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionItem {
    #[serde(rename = "_id")]
    pub item_object_id: Option<String>,
    pub item_id: Option<String>,
    pub item_type: Option<String>,
    pub position: Option<i64>,
    pub note: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collection_deserialize() {
        let json = r#"{
            "slug": "user/my-collection-abc123",
            "title": "My Collection",
            "owner": "user",
            "items": [],
            "private": false,
            "upvotes": 5
        }"#;
        let coll: Collection = serde_json::from_str(json).unwrap();
        assert_eq!(coll.slug, "user/my-collection-abc123");
        assert_eq!(coll.title.as_deref(), Some("My Collection"));
        assert_eq!(coll.upvotes, Some(5));
    }

    #[test]
    fn test_collection_item_deserialize() {
        let json = r#"{
            "_id": "item123",
            "itemId": "gpt2",
            "itemType": "model",
            "position": 0
        }"#;
        let item: CollectionItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.item_object_id.as_deref(), Some("item123"));
        assert_eq!(item.item_type.as_deref(), Some("model"));
    }
}
```

- [ ] **Step 2: Register module, add params, create API file, add feature flag, lint, commit**

Follow the same pattern as Chunks 1-2. The params structs:

```rust
#[cfg(feature = "collections")]
#[derive(TypedBuilder)]
pub struct GetCollectionParams {
    #[builder(setter(into))]
    pub slug: String,
}

#[cfg(feature = "collections")]
#[derive(TypedBuilder)]
pub struct ListCollectionsParams {
    #[builder(default, setter(into, strip_option))]
    pub owner: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub item: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub item_type: Option<String>,
    #[builder(default, setter(strip_option))]
    pub limit: Option<usize>,
    #[builder(default, setter(strip_option))]
    pub offset: Option<usize>,
}

#[cfg(feature = "collections")]
#[derive(TypedBuilder)]
pub struct CreateCollectionParams {
    #[builder(setter(into))]
    pub title: String,
    #[builder(default, setter(into, strip_option))]
    pub description: Option<String>,
    #[builder(default, setter(strip_option))]
    pub private: Option<bool>,
    #[builder(default, setter(into, strip_option))]
    pub namespace: Option<String>,
}

#[cfg(feature = "collections")]
#[derive(TypedBuilder)]
pub struct UpdateCollectionMetadataParams {
    #[builder(setter(into))]
    pub slug: String,
    #[builder(default, setter(into, strip_option))]
    pub title: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub description: Option<String>,
    #[builder(default, setter(strip_option))]
    pub private: Option<bool>,
    #[builder(default, setter(strip_option))]
    pub position: Option<i64>,
    #[builder(default, setter(into, strip_option))]
    pub theme: Option<String>,
}

#[cfg(feature = "collections")]
#[derive(TypedBuilder)]
pub struct DeleteCollectionParams {
    #[builder(setter(into))]
    pub slug: String,
    #[builder(default)]
    pub missing_ok: bool,
}

#[cfg(feature = "collections")]
#[derive(TypedBuilder)]
pub struct AddCollectionItemParams {
    #[builder(setter(into))]
    pub slug: String,
    #[builder(setter(into))]
    pub item_id: String,
    #[builder(setter(into))]
    pub item_type: String,
    #[builder(default, setter(into, strip_option))]
    pub note: Option<String>,
    #[builder(default)]
    pub exists_ok: bool,
}

#[cfg(feature = "collections")]
#[derive(TypedBuilder)]
pub struct UpdateCollectionItemParams {
    #[builder(setter(into))]
    pub slug: String,
    #[builder(setter(into))]
    pub item_object_id: String,
    #[builder(default, setter(into, strip_option))]
    pub note: Option<String>,
    #[builder(default, setter(strip_option))]
    pub position: Option<i64>,
}

#[cfg(feature = "collections")]
#[derive(TypedBuilder)]
pub struct DeleteCollectionItemParams {
    #[builder(setter(into))]
    pub slug: String,
    #[builder(setter(into))]
    pub item_object_id: String,
}
```

The API methods follow standard patterns — GET/POST/PATCH/DELETE with JSON bodies. `list_collections` uses query params. `delete_collection` returns 404 → Ok if `missing_ok`. `add_collection_item` returns 409 → Ok if `exists_ok`.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat: add collections feature module"
```

---

## Chunk 4: Discussions & Pull Requests (Feature 4)

### Feature: `discussions`

### API Endpoints

| Method | HTTP | Endpoint |
|--------|------|----------|
| `get_repo_discussions` | GET | `/api/{repo_type}s/{repo_id}/discussions` |
| `get_discussion_details` | GET | `/api/{repo_type}s/{repo_id}/discussions/{num}` |
| `create_discussion` | POST | `/api/{repo_type}s/{repo_id}/discussions` |
| `create_pull_request` | POST | `/api/{repo_type}s/{repo_id}/discussions` |
| `comment_discussion` | POST | `/api/{repo_type}s/{repo_id}/discussions/{num}/comment` |
| `edit_discussion_comment` | POST | `/api/{repo_type}s/{repo_id}/discussions/{num}/comment/{comment_id}/edit` |
| `hide_discussion_comment` | PUT | `/api/{repo_type}s/{repo_id}/discussions/{num}/comment/{comment_id}/hide` |
| `rename_discussion` | POST | `/api/{repo_type}s/{repo_id}/discussions/{num}/title` |
| `change_discussion_status` | POST | `/api/{repo_type}s/{repo_id}/discussions/{num}/status` |
| `merge_pull_request` | POST | `/api/{repo_type}s/{repo_id}/discussions/{num}/merge` |

### Task 4.1: Types — `types/discussions.rs`

**Files:**
- Create: `huggingface_hub/src/types/discussions.rs`

- [ ] **Step 1: Create types file**

```rust
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Discussion {
    pub num: u64,
    pub author: Option<String>,
    pub title: Option<String>,
    pub status: Option<String>,
    pub is_pull_request: Option<bool>,
    pub created_at: Option<String>,
    pub repo_id: Option<String>,
    pub repo_type: Option<String>,
    pub endpoint: Option<String>,
    pub pinned: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscussionWithDetails {
    pub num: u64,
    pub author: Option<String>,
    pub title: Option<String>,
    pub status: Option<String>,
    pub is_pull_request: Option<bool>,
    pub created_at: Option<String>,
    #[serde(default)]
    pub events: Vec<DiscussionEvent>,
    #[serde(default)]
    pub conflicting_files: Vec<String>,
    pub target_branch: Option<String>,
    pub merge_commit_oid: Option<String>,
    pub diff: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscussionEvent {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub event_type: Option<String>,
    pub author: Option<String>,
    pub content: Option<String>,
    pub created_at: Option<String>,
    pub edited: Option<bool>,
    pub hidden: Option<bool>,
    #[serde(default)]
    pub raw: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscussionComment {
    pub id: Option<String>,
    pub author: Option<String>,
    pub content: Option<String>,
    pub created_at: Option<String>,
    pub edited: Option<bool>,
    pub hidden: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discussion_deserialize() {
        let json = r#"{
            "num": 5,
            "author": "user1",
            "title": "Fix typo",
            "status": "open",
            "isPullRequest": true
        }"#;
        let disc: Discussion = serde_json::from_str(json).unwrap();
        assert_eq!(disc.num, 5);
        assert_eq!(disc.is_pull_request, Some(true));
    }

    #[test]
    fn test_discussion_with_details_deserialize() {
        let json = r#"{
            "num": 3,
            "title": "Bug report",
            "status": "open",
            "isPullRequest": false,
            "events": [{"id": "abc", "type": "comment", "content": "hello"}],
            "conflictingFiles": [],
            "targetBranch": "refs/heads/main"
        }"#;
        let disc: DiscussionWithDetails = serde_json::from_str(json).unwrap();
        assert_eq!(disc.num, 3);
        assert_eq!(disc.events.len(), 1);
    }
}
```

- [ ] **Step 2: Add params, API methods, feature flag, lint, commit**

Params include:

```rust
#[cfg(feature = "discussions")]
#[derive(TypedBuilder)]
pub struct GetRepoDiscussionsParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
    #[builder(default, setter(into, strip_option))]
    pub author: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub discussion_type: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub discussion_status: Option<String>,
}

#[cfg(feature = "discussions")]
#[derive(TypedBuilder)]
pub struct GetDiscussionDetailsParams {
    #[builder(setter(into))]
    pub repo_id: String,
    pub discussion_num: u64,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[cfg(feature = "discussions")]
#[derive(TypedBuilder)]
pub struct CreateDiscussionParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub title: String,
    #[builder(default, setter(into, strip_option))]
    pub description: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[cfg(feature = "discussions")]
#[derive(TypedBuilder)]
pub struct CreatePullRequestParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub title: String,
    #[builder(default, setter(into, strip_option))]
    pub description: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[cfg(feature = "discussions")]
#[derive(TypedBuilder)]
pub struct CommentDiscussionParams {
    #[builder(setter(into))]
    pub repo_id: String,
    pub discussion_num: u64,
    #[builder(setter(into))]
    pub comment: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[cfg(feature = "discussions")]
#[derive(TypedBuilder)]
pub struct EditDiscussionCommentParams {
    #[builder(setter(into))]
    pub repo_id: String,
    pub discussion_num: u64,
    #[builder(setter(into))]
    pub comment_id: String,
    #[builder(setter(into))]
    pub new_content: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[cfg(feature = "discussions")]
#[derive(TypedBuilder)]
pub struct HideDiscussionCommentParams {
    #[builder(setter(into))]
    pub repo_id: String,
    pub discussion_num: u64,
    #[builder(setter(into))]
    pub comment_id: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[cfg(feature = "discussions")]
#[derive(TypedBuilder)]
pub struct RenameDiscussionParams {
    #[builder(setter(into))]
    pub repo_id: String,
    pub discussion_num: u64,
    #[builder(setter(into))]
    pub new_title: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[cfg(feature = "discussions")]
#[derive(TypedBuilder)]
pub struct ChangeDiscussionStatusParams {
    #[builder(setter(into))]
    pub repo_id: String,
    pub discussion_num: u64,
    #[builder(setter(into))]
    pub new_status: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[cfg(feature = "discussions")]
#[derive(TypedBuilder)]
pub struct MergePullRequestParams {
    #[builder(setter(into))]
    pub repo_id: String,
    pub discussion_num: u64,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}
```

API methods: `get_repo_discussions` returns `impl Stream<Item = Result<Discussion>>` via `self.paginate()`. All others are standard async methods. `create_pull_request` posts to the same endpoint as `create_discussion` but with `"pullRequest": true` in the body.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat: add discussions feature module"
```

---

## Chunk 5: Webhooks (Feature 5)

### Feature: `webhooks`

### API Endpoints

| Method | HTTP | Endpoint |
|--------|------|----------|
| `list_webhooks` | GET | `/api/settings/webhooks` |
| `get_webhook` | GET | `/api/settings/webhooks/{webhook_id}` |
| `create_webhook` | POST | `/api/settings/webhooks` |
| `update_webhook` | PUT | `/api/settings/webhooks/{webhook_id}` |
| `delete_webhook` | DELETE | `/api/settings/webhooks/{webhook_id}` |
| `enable_webhook` | POST | `/api/settings/webhooks/{webhook_id}/enable` |
| `disable_webhook` | POST | `/api/settings/webhooks/{webhook_id}/disable` |

### Task 5.1: Types — `types/webhooks.rs`

**Files:**
- Create: `huggingface_hub/src/types/webhooks.rs`

- [ ] **Step 1: Create types file**

```rust
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookInfo {
    pub id: Option<String>,
    pub url: Option<String>,
    #[serde(default)]
    pub watched: Vec<WebhookWatchedItem>,
    #[serde(default)]
    pub domains: Vec<String>,
    pub secret: Option<String>,
    pub disabled: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookWatchedItem {
    #[serde(rename = "type")]
    pub item_type: Option<String>,
    pub name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_info_deserialize() {
        let json = r#"{
            "id": "wh-abc123",
            "url": "https://example.com/hook",
            "watched": [{"type": "user", "name": "john"}],
            "domains": ["repo"],
            "disabled": false
        }"#;
        let wh: WebhookInfo = serde_json::from_str(json).unwrap();
        assert_eq!(wh.id.as_deref(), Some("wh-abc123"));
        assert_eq!(wh.watched.len(), 1);
        assert_eq!(wh.disabled, Some(false));
    }
}
```

- [ ] **Step 2: Add params, API methods, feature flag**

Params:

```rust
#[cfg(feature = "webhooks")]
#[derive(TypedBuilder)]
pub struct CreateWebhookParams {
    #[builder(setter(into))]
    pub url: String,
    pub watched: Vec<serde_json::Value>,
    #[builder(default, setter(into, strip_option))]
    pub domains: Option<Vec<String>>,
    #[builder(default, setter(into, strip_option))]
    pub secret: Option<String>,
}

#[cfg(feature = "webhooks")]
#[derive(TypedBuilder)]
pub struct UpdateWebhookParams {
    #[builder(setter(into))]
    pub webhook_id: String,
    #[builder(default, setter(into, strip_option))]
    pub url: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub watched: Option<Vec<serde_json::Value>>,
    #[builder(default, setter(into, strip_option))]
    pub domains: Option<Vec<String>>,
    #[builder(default, setter(into, strip_option))]
    pub secret: Option<String>,
}
```

Simple CRUD methods: `list_webhooks() -> Result<Vec<WebhookInfo>>`, `get_webhook(id) -> Result<WebhookInfo>`, `create_webhook`, `update_webhook`, `delete_webhook(id) -> Result<()>`, `enable_webhook(id) -> Result<WebhookInfo>`, `disable_webhook(id) -> Result<WebhookInfo>`.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat: add webhooks feature module"
```

---

## Chunk 6: Jobs (Feature 6)

### Feature: `jobs`

### API Endpoints

| Method | HTTP | Endpoint |
|--------|------|----------|
| `run_job` | POST | `/api/jobs` |
| `list_jobs` | GET | `/api/jobs` |
| `inspect_job` | GET | `/api/jobs/{job_id}` |
| `cancel_job` | POST | `/api/jobs/{job_id}/cancel` |
| `fetch_job_logs` | GET | `/api/jobs/{job_id}/logs` |
| `fetch_job_metrics` | GET | `/api/jobs/{job_id}/metrics` |
| `list_job_hardware` | GET | `/api/jobs/hardware` |
| `create_scheduled_job` | POST | `/api/jobs/scheduled` |
| `list_scheduled_jobs` | GET | `/api/jobs/scheduled` |
| `inspect_scheduled_job` | GET | `/api/jobs/scheduled/{id}` |
| `delete_scheduled_job` | DELETE | `/api/jobs/scheduled/{id}` |
| `suspend_scheduled_job` | POST | `/api/jobs/scheduled/{id}/suspend` |
| `resume_scheduled_job` | POST | `/api/jobs/scheduled/{id}/resume` |

### Task 6.1: Types — `types/jobs.rs`

**Files:**
- Create: `huggingface_hub/src/types/jobs.rs`

- [ ] **Step 1: Create types file**

```rust
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
#[serde(rename_all = "camelCase")]
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
    pub flavor: Option<String>,
    pub description: Option<String>,
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
```

- [ ] **Step 2: Add params, API methods, feature flag**

Key params:

```rust
#[cfg(feature = "jobs")]
#[derive(TypedBuilder)]
pub struct RunJobParams {
    #[builder(setter(into))]
    pub image: String,
    pub command: Vec<String>,
    #[builder(default, setter(into, strip_option))]
    pub flavor: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub env: Option<std::collections::HashMap<String, String>>,
    #[builder(default, setter(into, strip_option))]
    pub secrets: Option<std::collections::HashMap<String, String>>,
    #[builder(default, setter(into, strip_option))]
    pub timeout: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub labels: Option<std::collections::HashMap<String, String>>,
    #[builder(default, setter(into, strip_option))]
    pub namespace: Option<String>,
}

#[cfg(feature = "jobs")]
#[derive(TypedBuilder)]
pub struct ListJobsParams {
    #[builder(default, setter(into, strip_option))]
    pub namespace: Option<String>,
}

#[cfg(feature = "jobs")]
#[derive(TypedBuilder)]
pub struct CreateScheduledJobParams {
    #[builder(setter(into))]
    pub image: String,
    pub command: Vec<String>,
    #[builder(setter(into))]
    pub schedule: String,
    #[builder(default, setter(into, strip_option))]
    pub flavor: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub env: Option<std::collections::HashMap<String, String>>,
    #[builder(default, setter(into, strip_option))]
    pub secrets: Option<std::collections::HashMap<String, String>>,
    #[builder(default, setter(into, strip_option))]
    pub timeout: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub namespace: Option<String>,
    #[builder(default, setter(strip_option))]
    pub suspend: Option<bool>,
    #[builder(default, setter(strip_option))]
    pub concurrency: Option<bool>,
}
```

API methods: `run_job`, `list_jobs`, `inspect_job(job_id)`, `cancel_job(job_id)`, `fetch_job_logs(job_id) -> Result<Vec<JobLogEntry>>`, `fetch_job_metrics(job_id) -> Result<Vec<JobMetrics>>`, `list_job_hardware() -> Result<Vec<JobHardware>>`, `create_scheduled_job`, `list_scheduled_jobs`, `inspect_scheduled_job`, `delete_scheduled_job`, `suspend_scheduled_job`, `resume_scheduled_job`.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat: add jobs feature module"
```

---

## Chunk 7: Access Requests (Feature 7)

### Feature: `access_requests`

### API Endpoints

| Method | HTTP | Endpoint |
|--------|------|----------|
| `list_pending_access_requests` | GET | `/api/{repo_type}s/{repo_id}/user-access-request/pending` |
| `list_accepted_access_requests` | GET | `/api/{repo_type}s/{repo_id}/user-access-request/accepted` |
| `list_rejected_access_requests` | GET | `/api/{repo_type}s/{repo_id}/user-access-request/rejected` |
| `accept_access_request` | POST | `/api/{repo_type}s/{repo_id}/user-access-request/handle` |
| `reject_access_request` | POST | `/api/{repo_type}s/{repo_id}/user-access-request/handle` |
| `cancel_access_request` | POST | `/api/{repo_type}s/{repo_id}/user-access-request/handle` |
| `grant_access` | POST | `/api/{repo_type}s/{repo_id}/user-access-request/grant` |

### Task 7.1: Types — `types/access_requests.rs`

**Files:**
- Create: `huggingface_hub/src/types/access_requests.rs`

- [ ] **Step 1: Create types file**

```rust
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessRequest {
    pub username: Option<String>,
    pub fullname: Option<String>,
    pub email: Option<String>,
    pub status: Option<String>,
    pub timestamp: Option<String>,
    pub fields: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_access_request_deserialize() {
        let json = r#"{
            "username": "user1",
            "email": "user1@example.com",
            "status": "pending",
            "timestamp": "2024-01-01T00:00:00Z"
        }"#;
        let req: AccessRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.username.as_deref(), Some("user1"));
        assert_eq!(req.status.as_deref(), Some("pending"));
    }
}
```

- [ ] **Step 2: Add params, API methods, feature flag**

Params:

```rust
#[cfg(feature = "access_requests")]
#[derive(TypedBuilder)]
pub struct ListAccessRequestsParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[cfg(feature = "access_requests")]
#[derive(TypedBuilder)]
pub struct HandleAccessRequestParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub user: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[cfg(feature = "access_requests")]
#[derive(TypedBuilder)]
pub struct GrantAccessParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(setter(into))]
    pub user: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}
```

API methods: `list_pending_access_requests`, `list_accepted_access_requests`, `list_rejected_access_requests` all return `Result<Vec<AccessRequest>>`. `accept_access_request`, `reject_access_request`, `cancel_access_request` all POST to the `/handle` endpoint with `{"user": ..., "status": "accepted"|"rejected"|"cancelled"}`. `grant_access` POSTs to `/grant`.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat: add access_requests feature module"
```

---

## Chunk 8: Likes & Interactions (Feature 9)

### Feature: `likes`

### API Endpoints

| Method | HTTP | Endpoint |
|--------|------|----------|
| `like` | POST | `/api/{repo_type}s/{repo_id}/like` |
| `unlike` | DELETE | `/api/{repo_type}s/{repo_id}/like` |
| `list_liked_repos` | GET | `/api/users/{username}/likes` |
| `list_repo_likers` | GET | `/api/{repo_type}s/{repo_id}/likers` |

### Task 8.1: Types — `types/likes.rs`

**Files:**
- Create: `huggingface_hub/src/types/likes.rs`

- [ ] **Step 1: Create types file**

```rust
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserLikes {
    pub user: Option<String>,
    pub total: Option<u64>,
    #[serde(default)]
    pub models: Vec<String>,
    #[serde(default)]
    pub datasets: Vec<String>,
    #[serde(default)]
    pub spaces: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_likes_deserialize() {
        let json = r#"{
            "user": "john",
            "total": 3,
            "models": ["gpt2", "bert-base"],
            "datasets": ["squad"],
            "spaces": []
        }"#;
        let likes: UserLikes = serde_json::from_str(json).unwrap();
        assert_eq!(likes.user.as_deref(), Some("john"));
        assert_eq!(likes.total, Some(3));
        assert_eq!(likes.models.len(), 2);
    }
}
```

- [ ] **Step 2: Add params, API methods, feature flag**

Params:

```rust
#[cfg(feature = "likes")]
#[derive(TypedBuilder)]
pub struct LikeParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}

#[cfg(feature = "likes")]
#[derive(TypedBuilder)]
pub struct ListLikedReposParams {
    #[builder(setter(into))]
    pub username: String,
}

#[cfg(feature = "likes")]
#[derive(TypedBuilder)]
pub struct ListRepoLikersParams {
    #[builder(setter(into))]
    pub repo_id: String,
    #[builder(default, setter(into, strip_option))]
    pub repo_type: Option<RepoType>,
}
```

API: `like` and `unlike` use the same `LikeParams`. `list_liked_repos` returns `Result<UserLikes>`. `list_repo_likers` returns paginated `impl Stream<Item = Result<User>>`.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat: add likes feature module"
```

---

## Chunk 9: Papers (Feature 12)

### Feature: `papers`

### API Endpoints

| Method | HTTP | Endpoint |
|--------|------|----------|
| `list_papers` | GET | `/api/papers/search` |
| `list_daily_papers` | GET | `/api/daily_papers` |
| `paper_info` | GET | `/api/papers/{paper_id}` |

### Task 9.1: Types — `types/papers.rs`

**Files:**
- Create: `huggingface_hub/src/types/papers.rs`

- [ ] **Step 1: Create types file**

```rust
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperInfo {
    pub id: String,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub authors: Option<Vec<PaperAuthor>>,
    pub published_at: Option<String>,
    pub upvotes: Option<u64>,
    pub num_comments: Option<u64>,
    #[serde(default)]
    pub raw: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperAuthor {
    pub name: Option<String>,
    #[serde(rename = "_id")]
    pub id: Option<String>,
    pub user: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyPaper {
    pub paper: Option<PaperInfo>,
    pub published_at: Option<String>,
    pub title: Option<String>,
    pub submitter: Option<String>,
    pub upvotes: Option<u64>,
    pub num_comments: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paper_info_deserialize() {
        let json = r#"{
            "id": "2307.09288",
            "title": "Llama 2",
            "upvotes": 100,
            "authors": [{"name": "Author One"}]
        }"#;
        let paper: PaperInfo = serde_json::from_str(json).unwrap();
        assert_eq!(paper.id, "2307.09288");
        assert_eq!(paper.upvotes, Some(100));
    }

    #[test]
    fn test_daily_paper_deserialize() {
        let json = r#"{
            "paper": {"id": "2307.09288", "title": "Test"},
            "publishedAt": "2024-01-01",
            "submitter": "user1",
            "upvotes": 5
        }"#;
        let dp: DailyPaper = serde_json::from_str(json).unwrap();
        assert_eq!(dp.paper.as_ref().unwrap().id, "2307.09288");
        assert_eq!(dp.upvotes, Some(5));
    }
}
```

- [ ] **Step 2: Add params, API methods, feature flag**

Params:

```rust
#[cfg(feature = "papers")]
#[derive(TypedBuilder)]
pub struct ListPapersParams {
    #[builder(default, setter(into, strip_option))]
    pub query: Option<String>,
    #[builder(default, setter(strip_option))]
    pub limit: Option<usize>,
}

#[cfg(feature = "papers")]
#[derive(TypedBuilder)]
pub struct ListDailyPapersParams {
    #[builder(default, setter(into, strip_option))]
    pub date: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub week: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub month: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub submitter: Option<String>,
    #[builder(default, setter(into, strip_option))]
    pub sort: Option<String>,
    #[builder(default, setter(strip_option))]
    pub p: Option<usize>,
    #[builder(default, setter(strip_option))]
    pub limit: Option<usize>,
}

#[cfg(feature = "papers")]
#[derive(TypedBuilder)]
pub struct PaperInfoParams {
    #[builder(setter(into))]
    pub paper_id: String,
}
```

API: `list_papers` → `GET /api/papers/search?q={query}` returns `Result<Vec<PaperInfo>>`. `list_daily_papers` → `GET /api/daily_papers` with query params, returns `Result<Vec<DailyPaper>>`. `paper_info` → `GET /api/papers/{paper_id}` returns `Result<PaperInfo>`.

- [ ] **Step 3: Integration tests**

```rust
#[cfg(feature = "papers")]
#[tokio::test]
async fn test_paper_info() {
    let Some(api) = api() else { return };
    let params = PaperInfoParams::builder().paper_id("2307.09288").build();
    let paper = api.paper_info(&params).await.unwrap();
    assert_eq!(paper.id, "2307.09288");
    assert!(paper.title.is_some());
}

#[cfg(feature = "papers")]
#[tokio::test]
async fn test_list_daily_papers() {
    let Some(api) = api() else { return };
    let params = ListDailyPapersParams::builder()
        .date("2024-10-29")
        .limit(5_usize)
        .build();
    let papers = api.list_daily_papers(&params).await.unwrap();
    assert!(!papers.is_empty());
}
```

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: add papers feature module"
```

---

## Final Steps

### Task F.1: Update CLAUDE.md Project Layout

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: Update the Project Layout section to document new modules**

Add to the `src/types/` section:
```
│   │   ├── spaces.rs           # SpaceRuntime, SpaceVariable (behind "spaces" feature)
│   │   ├── inference_endpoints.rs # InferenceEndpointInfo, status types (behind "inference_endpoints")
│   │   ├── collections.rs      # Collection, CollectionItem (behind "collections" feature)
│   │   ├── discussions.rs      # Discussion, DiscussionWithDetails, events (behind "discussions")
│   │   ├── webhooks.rs         # WebhookInfo, WebhookWatchedItem (behind "webhooks" feature)
│   │   ├── jobs.rs             # JobInfo, JobStatus, ScheduledJobInfo (behind "jobs" feature)
│   │   ├── access_requests.rs  # AccessRequest (behind "access_requests" feature)
│   │   ├── likes.rs            # UserLikes (behind "likes" feature)
│   │   └── papers.rs           # PaperInfo, DailyPaper (behind "papers" feature)
```

Add to the `src/api/` section:
```
│   │       ├── spaces.rs       # Space runtime, secrets, variables, hardware, pause/restart
│   │       ├── inference_endpoints.rs # IE create/get/list/update/delete/pause/resume
│   │       ├── collections.rs  # Collection CRUD, item management
│   │       ├── discussions.rs  # Discussions & PRs, comments, merge
│   │       ├── webhooks.rs     # Webhook CRUD, enable/disable
│   │       ├── jobs.rs         # Jobs run/list/inspect/cancel, scheduled jobs
│   │       ├── access_requests.rs # Gated repo access requests
│   │       ├── likes.rs        # Like/unlike, list liked repos/likers
│   │       └── papers.rs       # Paper search, daily papers, paper info
```

- [ ] **Step 2: Final format and lint with all features**

Run: `cargo +nightly fmt && cargo clippy -p huggingface-hub --all-features -- -D warnings`

- [ ] **Step 3: Run all unit tests**

Run: `cargo test -p huggingface-hub --all-features`

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "docs: update project layout with new feature modules"
```
