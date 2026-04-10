use futures::Stream;
use url::Url;

use crate::client::HFClient;
use crate::error::{NotFoundContext, Result};
use crate::types::{BucketInfo, BucketUrl, CreateBucketParams};

impl HFClient {
    /// Create a new bucket on the Hub.
    ///
    /// Endpoint: `POST /api/buckets/{namespace}/{name}`
    pub async fn create_bucket(&self, params: &CreateBucketParams) -> Result<BucketUrl> {
        let url = format!("{}/api/buckets/{}/{}", self.endpoint(), params.namespace, params.name);

        let mut body = serde_json::json!({});
        if params.private {
            body["private"] = serde_json::Value::Bool(true);
        }
        if let Some(ref rg) = params.resource_group_id {
            body["resourceGroupId"] = serde_json::Value::String(rg.clone());
        }

        let response = self
            .http_client()
            .post(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;

        let bucket_id = format!("{}/{}", params.namespace, params.name);

        if response.status().as_u16() == 409 && params.exist_ok {
            return Ok(BucketUrl {
                url: format!("{}/buckets/{}", self.endpoint(), bucket_id),
            });
        }

        let response = self
            .check_response(response, Some(&bucket_id), NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    /// Delete a bucket from the Hub.
    ///
    /// Endpoint: `DELETE /api/buckets/{bucket_id}`
    pub async fn delete_bucket(&self, bucket_id: &str, missing_ok: bool) -> Result<()> {
        let url = self.bucket_api_url(bucket_id);

        let response = self.http_client().delete(&url).headers(self.auth_headers()).send().await?;

        if response.status().as_u16() == 404 && missing_ok {
            return Ok(());
        }

        self.check_response(response, Some(bucket_id), NotFoundContext::Bucket).await?;
        Ok(())
    }

    /// List buckets in a namespace.
    ///
    /// Endpoint: `GET /api/buckets/{namespace}` (paginated)
    pub fn list_buckets(&self, namespace: &str) -> Result<impl Stream<Item = Result<BucketInfo>> + '_> {
        let url = Url::parse(&format!("{}/api/buckets/{}", self.endpoint(), namespace))?;
        Ok(self.paginate(url, vec![], None))
    }

    /// Move (rename) a bucket.
    ///
    /// Endpoint: `POST /api/repos/move` with `type: "bucket"`
    pub async fn move_bucket(&self, from_id: &str, to_id: &str) -> Result<()> {
        let url = format!("{}/api/repos/move", self.endpoint());
        let body = serde_json::json!({
            "fromRepo": from_id,
            "toRepo": to_id,
            "type": "bucket",
        });

        let response = self
            .http_client()
            .post(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;

        self.check_response(response, None, NotFoundContext::Generic).await?;
        Ok(())
    }
}
