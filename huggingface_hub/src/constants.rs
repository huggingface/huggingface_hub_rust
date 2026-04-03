/// Default Hugging Face Hub endpoint
pub const DEFAULT_HF_ENDPOINT: &str = "https://huggingface.co";

/// Default revision (branch)
pub const DEFAULT_REVISION: &str = "main";

pub const HF_ENDPOINT: &str = "HF_ENDPOINT";
pub const HF_TOKEN: &str = "HF_TOKEN";
pub const HF_TOKEN_PATH: &str = "HF_TOKEN_PATH";
pub const HF_HOME: &str = "HF_HOME";
pub const HF_HUB_CACHE: &str = "HF_HUB_CACHE";
pub const HUGGINGFACE_HUB_CACHE: &str = "HUGGINGFACE_HUB_CACHE";
pub const XDG_CACHE_HOME: &str = "XDG_CACHE_HOME";
pub const HF_HUB_DISABLE_IMPLICIT_TOKEN: &str = "HF_HUB_DISABLE_IMPLICIT_TOKEN";
pub const HF_HUB_USER_AGENT_ORIGIN: &str = "HF_HUB_USER_AGENT_ORIGIN";

/// Token filename within HF_HOME
pub const TOKEN_FILENAME: &str = "token";

pub const HEADER_X_XET_HASH: &str = "x-xet-hash";

pub const CACHE_LOCK_TIMEOUT_SECS: u64 = 10;
pub const HEADER_X_REPO_COMMIT: &str = "x-repo-commit";
pub const HEADER_X_LINKED_ETAG: &str = "x-linked-etag";

/// URL prefixes for different repo types
/// Models have no prefix, datasets use "datasets/", spaces use "spaces/", kernels use "kernels/"
pub fn repo_type_url_prefix(repo_type: Option<crate::types::repo::RepoType>) -> &'static str {
    match repo_type {
        None | Some(crate::types::repo::RepoType::Model) => "",
        Some(crate::types::repo::RepoType::Dataset) => "datasets/",
        Some(crate::types::repo::RepoType::Space) => "spaces/",
        Some(crate::types::repo::RepoType::Kernel) => "kernels/",
    }
}

/// API path segment for repo types: "models", "datasets", "spaces", "kernels"
pub fn repo_type_api_segment(repo_type: Option<crate::types::repo::RepoType>) -> &'static str {
    match repo_type {
        None | Some(crate::types::repo::RepoType::Model) => "models",
        Some(crate::types::repo::RepoType::Dataset) => "datasets",
        Some(crate::types::repo::RepoType::Space) => "spaces",
        Some(crate::types::repo::RepoType::Kernel) => "kernels",
    }
}

#[cfg(test)]
mod tests {
    use super::{repo_type_api_segment, repo_type_url_prefix};
    use crate::types::repo::RepoType;

    #[test]
    fn test_repo_type_url_prefix() {
        assert_eq!(repo_type_url_prefix(None), "");
        assert_eq!(repo_type_url_prefix(Some(RepoType::Model)), "");
        assert_eq!(repo_type_url_prefix(Some(RepoType::Dataset)), "datasets/");
        assert_eq!(repo_type_url_prefix(Some(RepoType::Space)), "spaces/");
        assert_eq!(repo_type_url_prefix(Some(RepoType::Kernel)), "kernels/");
    }

    #[test]
    fn test_repo_type_api_segment() {
        assert_eq!(repo_type_api_segment(None), "models");
        assert_eq!(repo_type_api_segment(Some(RepoType::Model)), "models");
        assert_eq!(repo_type_api_segment(Some(RepoType::Dataset)), "datasets");
        assert_eq!(repo_type_api_segment(Some(RepoType::Space)), "spaces");
        assert_eq!(repo_type_api_segment(Some(RepoType::Kernel)), "kernels");
    }
}
