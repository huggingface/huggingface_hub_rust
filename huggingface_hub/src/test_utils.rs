// Shared constants and helpers for integration tests.

// --- Environment variable names ---

pub const HF_TOKEN: &str = "HF_TOKEN";
pub const HF_CI_TOKEN: &str = "HF_CI_TOKEN";
pub const HF_PROD_TOKEN: &str = "HF_PROD_TOKEN";
pub const HF_TEST_WRITE: &str = "HF_TEST_WRITE";
pub const HF_ENDPOINT: &str = "HF_ENDPOINT";
pub const GITHUB_ACTIONS: &str = "GITHUB_ACTIONS";
pub const HF_HUB_CACHE: &str = "HF_HUB_CACHE";
pub const HF_HOME: &str = "HF_HOME";
pub const XDG_CACHE_HOME: &str = "XDG_CACHE_HOME";

// --- Endpoints ---

pub const PROD_ENDPOINT: &str = "https://huggingface.co";
pub const HUB_CI_ENDPOINT: &str = "https://hub-ci.huggingface.co";

// --- Common helpers ---

pub fn is_ci() -> bool {
    std::env::var(GITHUB_ACTIONS).is_ok()
}

pub fn write_enabled() -> bool {
    std::env::var(HF_TEST_WRITE).ok().is_some_and(|v| v == "1")
}

/// Resolve a token suitable for the current environment.
/// CI: tries HF_CI_TOKEN. Local: tries HF_TOKEN.
pub fn resolve_ci_token() -> Option<String> {
    std::env::var(HF_TOKEN).or_else(|_| std::env::var(HF_CI_TOKEN)).ok()
}

/// Resolve a token for production access.
/// CI: tries HF_PROD_TOKEN. Local: tries HF_TOKEN.
pub fn resolve_prod_token() -> Option<String> {
    if is_ci() {
        std::env::var(HF_PROD_TOKEN).ok()
    } else {
        std::env::var(HF_TOKEN).ok()
    }
}
