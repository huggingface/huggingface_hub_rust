use futures::stream::{Stream, StreamExt};
use url::Url;
use crate::client::HfApi;
use crate::constants;
use crate::error::Result;
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
