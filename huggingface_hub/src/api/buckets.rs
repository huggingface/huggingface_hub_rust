use std::collections::VecDeque;

use futures::Stream;
use url::Url;

use crate::buckets::HFBucket;
use crate::error::{HFError, NotFoundContext};
use crate::pagination::parse_link_header_next;
use crate::types::{
    BatchOp, BatchResult, BucketCreated, BucketOverview, CreateBucketParams, ListTreeParams, ResolvedFile, TreeEntry,
    UpdateBucketParams, XetToken,
};
use crate::{HFClient, Result};

/// Maps HTTP status codes to `HFError` variants for bucket API responses.
/// Bucket-level 404s map to `RepoNotFound`; file-level 404s map to `EntryNotFound`.
pub(crate) async fn check_bucket_response(
    response: reqwest::Response,
    repo_id: &str,
    not_found_ctx: NotFoundContext,
) -> Result<reqwest::Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let url = response.url().to_string();
    let body = response.text().await.unwrap_or_default();
    Err(match status.as_u16() {
        401 => HFError::AuthRequired,
        403 => HFError::Forbidden,
        404 => match not_found_ctx {
            NotFoundContext::Repo => HFError::RepoNotFound {
                repo_id: repo_id.to_string(),
            },
            NotFoundContext::Entry { path } => HFError::EntryNotFound {
                path,
                repo_id: repo_id.to_string(),
            },
            _ => HFError::Http { status, url, body },
        },
        409 => HFError::Conflict(body),
        429 => HFError::RateLimited,
        _ => HFError::Http { status, url, body },
    })
}

impl HFBucket {
    fn repo_id(&self) -> String {
        format!("{}/{}", self.namespace, self.repo)
    }

    fn bucket_url(&self) -> String {
        format!("{}/api/buckets/{}/{}", self.client.inner.endpoint, self.namespace, self.repo)
    }

    /// Returns metadata about this bucket.
    pub async fn info(&self) -> Result<BucketOverview> {
        let resp = self
            .client
            .inner
            .client
            .get(self.bucket_url())
            .headers(self.client.auth_headers())
            .send()
            .await?;
        let resp = check_bucket_response(resp, &self.repo_id(), NotFoundContext::Repo).await?;
        Ok(resp.json().await?)
    }

    /// Updates visibility or CDN configuration for this bucket.
    pub async fn update_settings(&self, params: UpdateBucketParams) -> Result<()> {
        let resp = self
            .client
            .inner
            .client
            .put(format!("{}/settings", self.bucket_url()))
            .headers(self.client.auth_headers())
            .json(&params)
            .send()
            .await?;
        check_bucket_response(resp, &self.repo_id(), NotFoundContext::Repo).await?;
        Ok(())
    }

    /// Adds and/or removes files in a single atomic operation.
    ///
    /// All `AddFile` operations are sent before `DeleteFile` operations, as required
    /// by the batch protocol. The input order within each group is preserved.
    pub async fn batch_files(&self, ops: Vec<BatchOp>) -> Result<BatchResult> {
        let (adds, deletes): (Vec<_>, Vec<_>) = ops.into_iter().partition(|op| matches!(op, BatchOp::AddFile(_)));

        let ndjson = adds
            .iter()
            .chain(deletes.iter())
            .map(|op| serde_json::to_string(op).map(|s| s + "\n"))
            .collect::<std::result::Result<String, _>>()?;

        let resp = self
            .client
            .inner
            .client
            .post(format!("{}/batch", self.bucket_url()))
            .headers(self.client.auth_headers())
            .header("content-type", "application/x-ndjson")
            .body(ndjson)
            .send()
            .await?;

        let resp = check_bucket_response(resp, &self.repo_id(), NotFoundContext::Repo).await?;
        Ok(resp.json().await?)
    }

    /// Lists files and directories, yielding one entry at a time.
    ///
    /// Uses cursor-in-body pagination: the stream fetches the next page automatically
    /// when the current page's entries are exhausted. No request is made until the
    /// first item is polled.
    pub fn list_tree(&self, path: &str, params: ListTreeParams) -> Result<impl Stream<Item = Result<TreeEntry>> + '_> {
        let base_url = if path.is_empty() {
            format!("{}/api/buckets/{}/{}/tree", self.client.inner.endpoint, self.namespace, self.repo)
        } else {
            format!("{}/api/buckets/{}/{}/tree/{}", self.client.inner.endpoint, self.namespace, self.repo, path)
        };
        let repo_id = self.repo_id();
        let mut initial_url = Url::parse(&base_url)?;
        {
            let mut qp = initial_url.query_pairs_mut();
            if let Some(l) = params.limit {
                qp.append_pair("limit", l.to_string().as_str());
            }
            if params.recursive {
                qp.append_pair("recursive", "true");
            }
            qp.finish();
        }

        Ok(futures::stream::try_unfold(
            (VecDeque::<TreeEntry>::new(), Some(initial_url), false),
            move |(mut pending, next_url, fetched)| {
                let client = self.client.clone();
                let repo_id = repo_id.clone();
                async move {
                    if let Some(entry) = pending.pop_front() {
                        return Ok(Some((entry, (pending, next_url, fetched))));
                    }
                    let url = match next_url {
                        Some(url) => url,
                        None if fetched => return Ok(None),
                        None => {
                            // if !fetched
                            return Err(HFError::Other("Initial list Url not set".to_string()));
                        },
                    };
                    let req = client.inner.client.get(url).headers(client.auth_headers());
                    let resp = req.send().await?;
                    let resp = check_bucket_response(resp, &repo_id, NotFoundContext::Repo).await?;
                    let next_cursor = parse_link_header_next(resp.headers());
                    let entries: Vec<TreeEntry> = resp.json().await?;

                    pending.extend(entries);
                    if let Some(entry) = pending.pop_front() {
                        Ok(Some((entry, (pending, next_cursor, true))))
                    } else {
                        Ok(None)
                    }
                }
            },
        ))
    }

    /// Returns metadata for a batch of file paths.
    ///
    /// Paths that do not exist in the bucket are silently omitted from the result.
    pub async fn get_paths_info(&self, paths: Vec<String>) -> Result<Vec<TreeEntry>> {
        #[derive(serde::Serialize)]
        struct Body {
            paths: Vec<String>,
        }

        let resp = self
            .client
            .inner
            .client
            .post(format!("{}/paths-info", self.bucket_url()))
            .headers(self.client.auth_headers())
            .json(&Body { paths })
            .send()
            .await?;

        let resp = check_bucket_response(resp, &self.repo_id(), NotFoundContext::Entry { path: String::new() }).await?;
        Ok(resp.json().await?)
    }

    /// Returns a short-lived JWT for uploading files to the Xet CAS.
    /// Use the returned `cas_url` and `token` to push file bytes before calling `batch_files`.
    pub async fn get_xet_write_token(&self) -> Result<XetToken> {
        let resp = self
            .client
            .inner
            .client
            .get(format!("{}/xet-write-token", self.bucket_url()))
            .headers(self.client.auth_headers())
            .send()
            .await?;
        let resp = check_bucket_response(resp, &self.repo_id(), NotFoundContext::Repo).await?;
        Ok(resp.json().await?)
    }

    /// Returns a short-lived JWT for downloading files from the Xet CAS directly.
    pub async fn get_xet_read_token(&self) -> Result<XetToken> {
        let resp = self
            .client
            .inner
            .client
            .get(format!("{}/xet-read-token", self.bucket_url()))
            .headers(self.client.auth_headers())
            .send()
            .await?;
        let resp = check_bucket_response(resp, &self.repo_id(), NotFoundContext::Repo).await?;
        Ok(resp.json().await?)
    }

    /// Resolves a file path to a direct download URL.
    ///
    /// Uses the no-redirect client to capture the 302 `Location` header rather than
    /// following it. Metadata is extracted from response headers:
    /// `X-Linked-Size`, `X-XET-Hash`, `X-Linked-ETag`, `Last-Modified`, and `Link`.
    pub async fn resolve_file(&self, path: &str) -> Result<ResolvedFile> {
        let url = format!("{}/buckets/{}/{}/resolve/{}", self.client.inner.endpoint, self.namespace, self.repo, path);
        let resp = self
            .client
            .inner
            .no_redirect_client
            .get(&url)
            .headers(self.client.auth_headers())
            .send()
            .await?;

        if !resp.status().is_redirection() {
            return Err(check_bucket_response(
                resp,
                &self.repo_id(),
                NotFoundContext::Entry { path: path.to_string() },
            )
            .await
            .unwrap_err());
        }

        let headers = resp.headers();

        let location = headers
            .get("location")
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
            .ok_or_else(|| HFError::Http {
                status: resp.status(),
                url: url.clone(),
                body: "missing Location header".to_string(),
            })?;

        let size = headers
            .get("x-linked-size")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok());

        let xet_hash = headers.get("x-xet-hash").and_then(|v| v.to_str().ok()).map(str::to_owned);

        let etag = headers.get("x-linked-etag").and_then(|v| v.to_str().ok()).map(str::to_owned);

        let last_modified = headers.get("last-modified").and_then(|v| v.to_str().ok()).map(str::to_owned);

        let mut xet_auth_url = None;
        let mut xet_reconstruction_url = None;
        if let Some(link) = headers.get("link").and_then(|v| v.to_str().ok()) {
            for part in link.split(',') {
                let part = part.trim();
                if let Some((url_part, rel_part)) = part.split_once(';') {
                    let u = url_part.trim().trim_start_matches('<').trim_end_matches('>').to_string();
                    if rel_part.contains("xet-auth") {
                        xet_auth_url = Some(u);
                    } else if rel_part.contains("xet-reconstruction-info") {
                        xet_reconstruction_url = Some(u);
                    }
                }
            }
        }

        Ok(ResolvedFile {
            url: location,
            size,
            xet_hash,
            etag,
            last_modified,
            xet_auth_url,
            xet_reconstruction_url,
        })
    }

    /// Resolves a file path and returns Xet reconstruction metadata.
    ///
    /// Sends `Accept: application/vnd.xet-fileinfo+json` to request the JSON response
    /// instead of a redirect. Use the returned `reconstruction_url` to fetch chunk data
    /// from the Xet CAS directly.
    #[cfg(feature = "xet")]
    pub async fn xet_resolve_file(&self, path: &str) -> Result<crate::types::XetFileInfo> {
        let url = format!("{}/buckets/{}/{}/resolve/{}", self.client.inner.endpoint, self.namespace, self.repo, path);
        let resp = self
            .client
            .inner
            .client
            .get(&url)
            .headers(self.client.auth_headers())
            .header("accept", "application/vnd.xet-fileinfo+json")
            .send()
            .await?;
        let resp =
            check_bucket_response(resp, &self.repo_id(), NotFoundContext::Entry { path: path.to_string() }).await?;
        Ok(resp.json().await?)
    }
}

impl HFClient {
    /// Permanently deletes a bucket and all of its files.
    pub async fn delete_bucket(&self, namespace: &str, repo: &str) -> Result<()> {
        let url = format!("{}/api/buckets/{}/{}", self.inner.endpoint, namespace, repo);
        let repo_id = format!("{}/{}", namespace, repo);
        let resp = self.inner.client.delete(&url).headers(self.auth_headers()).send().await?;
        check_bucket_response(resp, &repo_id, NotFoundContext::Repo).await?;
        Ok(())
    }

    /// Creates a new bucket owned by `namespace`.
    pub async fn create_bucket(
        &self,
        namespace: &str,
        repo: &str,
        params: CreateBucketParams,
    ) -> Result<BucketCreated> {
        let url = format!("{}/api/buckets/{}/{}", self.inner.endpoint, namespace, repo);
        let resp = self
            .inner
            .client
            .post(&url)
            .headers(self.auth_headers())
            .json(&params)
            .send()
            .await?;
        let repo_id = format!("{}/{}", namespace, repo);
        let resp = check_bucket_response(resp, &repo_id, NotFoundContext::Repo).await?;
        Ok(resp.json().await?)
    }

    /// Returns a paginated stream of all buckets owned by `namespace`.
    /// Pagination is driven by `Link` response headers.
    pub fn list_buckets(&self, namespace: &str) -> Result<impl Stream<Item = Result<BucketOverview>> + '_> {
        let url = Url::parse(&format!("{}/api/buckets/{}", self.inner.endpoint, namespace))?;
        Ok(self.paginate(url, vec![], None))
    }
}

sync_api! {
    impl HFBucket -> HFBucketSync {
        fn info(&self) -> Result<BucketOverview>;
        fn update_settings(&self, params: UpdateBucketParams) -> Result<()>;
        fn batch_files(&self, ops: Vec<BatchOp>) -> Result<BatchResult>;
        fn get_paths_info(&self, paths: Vec<String>) -> Result<Vec<TreeEntry>>;
        fn get_xet_write_token(&self) -> Result<XetToken>;
        fn get_xet_read_token(&self) -> Result<XetToken>;
        fn resolve_file(&self, path: &str) -> Result<ResolvedFile>;
    }
}

sync_api_stream! {
    impl HFBucket -> HFBucketSync {
        fn list_tree(&self, path: &str, params: ListTreeParams) -> TreeEntry;
    }
}

sync_api! {
    #[cfg(feature = "xet")]
    impl HFBucket -> HFBucketSync {
        fn xet_resolve_file(&self, path: &str) -> Result<crate::types::XetFileInfo>;
    }
}

sync_api! {
    impl HFClient -> HFClientSync {
        fn delete_bucket(&self, namespace: &str, repo: &str) -> Result<()>;
        fn create_bucket(&self, namespace: &str, repo: &str, params: CreateBucketParams) -> Result<BucketCreated>;
    }
}

sync_api_stream! {
    impl HFClient -> HFClientSync {
        fn list_buckets(&self, namespace: &str) -> BucketOverview;
    }
}

#[cfg(test)]
mod tests {
    use crate::HFClientBuilder;

    #[test]
    fn bucket_constructor_sets_namespace_and_repo() {
        let client = HFClientBuilder::new().build().unwrap();
        let bucket = client.bucket("myuser", "my-bucket");
        assert_eq!(bucket.namespace, "myuser");
        assert_eq!(bucket.repo, "my-bucket");
    }

    #[test]
    fn get_bucket_url() {
        let client = HFClientBuilder::new().build().unwrap();
        let bucket = client.bucket("myuser", "my-bucket");
        let url = format!("{}/api/buckets/{}/{}", bucket.client.inner.endpoint, bucket.namespace, bucket.repo);
        assert!(url.ends_with("/api/buckets/myuser/my-bucket"));
    }

    #[test]
    fn update_settings_url() {
        let client = HFClientBuilder::new().build().unwrap();
        let bucket = client.bucket("myuser", "my-bucket");
        let url = format!("{}/api/buckets/{}/{}/settings", bucket.client.inner.endpoint, bucket.namespace, bucket.repo);
        assert!(url.ends_with("/api/buckets/myuser/my-bucket/settings"));
    }

    #[test]
    fn create_bucket_url() {
        let client = HFClientBuilder::new().build().unwrap();
        let url = format!("{}/api/buckets/{}/{}", client.inner.endpoint, "myuser", "new-bucket");
        assert!(url.ends_with("/api/buckets/myuser/new-bucket"));
    }

    #[test]
    fn list_buckets_url() {
        let client = HFClientBuilder::new().build().unwrap();
        let url = format!("{}/api/buckets/{}", client.inner.endpoint, "myuser");
        assert!(url.ends_with("/api/buckets/myuser"));
    }

    #[test]
    fn batch_files_ndjson_adds_before_deletes() {
        use crate::types::{AddFileOp, BatchOp, DeleteFileOp};

        let ops = vec![
            BatchOp::DeleteFile(DeleteFileOp {
                path: "old.parquet".to_string(),
            }),
            BatchOp::AddFile(AddFileOp {
                path: "new.parquet".to_string(),
                xet_hash: "abc".to_string(),
                content_type: "application/octet-stream".to_string(),
                mtime: None,
            }),
        ];
        let (adds, deletes): (Vec<_>, Vec<_>) = ops.into_iter().partition(|op| matches!(op, BatchOp::AddFile(_)));
        let ndjson: String = adds
            .iter()
            .chain(deletes.iter())
            .map(|op| serde_json::to_string(op).map(|s| s + "\n"))
            .collect::<Result<_, _>>()
            .unwrap();
        let lines: Vec<&str> = ndjson.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("addFile"), "first line must be addFile, got: {}", lines[0]);
        assert!(lines[1].contains("deleteFile"), "second line must be deleteFile");
    }

    #[test]
    fn batch_files_each_line_ends_with_newline() {
        use crate::types::{AddFileOp, BatchOp};
        let ops = vec![BatchOp::AddFile(AddFileOp {
            path: "f.parquet".to_string(),
            xet_hash: "h".to_string(),
            content_type: "application/octet-stream".to_string(),
            mtime: None,
        })];
        let (adds, deletes): (Vec<_>, Vec<_>) = ops.into_iter().partition(|op| matches!(op, BatchOp::AddFile(_)));
        let ndjson: String = adds
            .iter()
            .chain(deletes.iter())
            .map(|op| serde_json::to_string(op).map(|s| s + "\n"))
            .collect::<Result<_, _>>()
            .unwrap();
        assert!(ndjson.ends_with('\n'));
    }

    #[test]
    fn list_tree_url_empty_path() {
        let client = HFClientBuilder::new().build().unwrap();
        let bucket = client.bucket("myuser", "my-bucket");
        let url = if "".is_empty() {
            format!("{}/api/buckets/{}/{}/tree", bucket.client.inner.endpoint, bucket.namespace, bucket.repo)
        } else {
            format!(
                "{}/api/buckets/{}/{}/tree/{}",
                bucket.client.inner.endpoint, bucket.namespace, bucket.repo, "some/path"
            )
        };
        assert!(url.ends_with("/api/buckets/myuser/my-bucket/tree"));
    }

    #[test]
    fn list_tree_url_with_path() {
        let client = HFClientBuilder::new().build().unwrap();
        let bucket = client.bucket("myuser", "my-bucket");
        let path = "data/sub";
        let url =
            format!("{}/api/buckets/{}/{}/tree/{}", bucket.client.inner.endpoint, bucket.namespace, bucket.repo, path);
        assert!(url.ends_with("/api/buckets/myuser/my-bucket/tree/data/sub"));
    }

    #[test]
    fn xet_token_urls() {
        let client = HFClientBuilder::new().build().unwrap();
        let bucket = client.bucket("myuser", "my-bucket");
        let write_url = format!(
            "{}/api/buckets/{}/{}/xet-write-token",
            bucket.client.inner.endpoint, bucket.namespace, bucket.repo
        );
        let read_url =
            format!("{}/api/buckets/{}/{}/xet-read-token", bucket.client.inner.endpoint, bucket.namespace, bucket.repo);
        assert!(write_url.ends_with("/xet-write-token"));
        assert!(read_url.ends_with("/xet-read-token"));
    }

    #[test]
    fn paths_info_url() {
        let client = HFClientBuilder::new().build().unwrap();
        let bucket = client.bucket("myuser", "my-bucket");
        let url =
            format!("{}/api/buckets/{}/{}/paths-info", bucket.client.inner.endpoint, bucket.namespace, bucket.repo);
        assert!(url.ends_with("/paths-info"));
    }

    #[test]
    fn resolve_file_parses_link_header() {
        let link = r#"<https://auth.example.com/token>; rel="xet-auth", <https://xet.example.com/reconstruct/abc>; rel="xet-reconstruction-info""#;
        let mut xet_auth = None;
        let mut xet_reconstruction = None;
        for part in link.split(',') {
            let part = part.trim();
            if let Some((url_part, rel_part)) = part.split_once(';') {
                let url = url_part.trim().trim_start_matches('<').trim_end_matches('>').to_string();
                let rel = rel_part.trim();
                if rel.contains("xet-auth") {
                    xet_auth = Some(url);
                } else if rel.contains("xet-reconstruction-info") {
                    xet_reconstruction = Some(url);
                }
            }
        }
        assert_eq!(xet_auth.unwrap(), "https://auth.example.com/token");
        assert_eq!(xet_reconstruction.unwrap(), "https://xet.example.com/reconstruct/abc");
    }

    #[test]
    fn resolve_file_url() {
        let client = HFClientBuilder::new().build().unwrap();
        let bucket = client.bucket("myuser", "my-bucket");
        let url = format!(
            "{}/buckets/{}/{}/resolve/{}",
            bucket.client.inner.endpoint, bucket.namespace, bucket.repo, "data/train.parquet"
        );
        assert!(url.contains("/buckets/myuser/my-bucket/resolve/data/train.parquet"));
        assert!(!url.contains("/api/"));
    }

    #[cfg(feature = "xet")]
    #[test]
    fn xet_resolve_file_url() {
        let client = HFClientBuilder::new().build().unwrap();
        let bucket = client.bucket("myuser", "my-bucket");
        let url = format!(
            "{}/buckets/{}/{}/resolve/{}",
            bucket.client.inner.endpoint, bucket.namespace, bucket.repo, "data/train.parquet"
        );
        assert!(url.contains("/buckets/myuser/my-bucket/resolve/data/train.parquet"));
    }
}
