use crate::HFClient;

/// Handle for operations on a single HuggingFace Storage Bucket.
///
/// Obtain via [`HFClient::bucket`]. Every method adds `Authorization: Bearer <token>`
/// using the token configured on the client.
///
/// # Example
///
/// ```rust,no_run
/// # #[tokio::main]
/// # async fn main() -> huggingface_hub::Result<()> {
/// let client = huggingface_hub::HFClient::new()?;
/// let bucket = client.bucket("my-org", "my-bucket");
/// let overview = bucket.info().await?;
/// println!("Bucket size: {} bytes", overview.size);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct HFBucket {
    pub(crate) client: HFClient,
    /// The namespace (user or organization) that owns the bucket.
    pub namespace: String,
    /// The bucket name within the namespace.
    pub bucket: String,
}

impl HFClient {
    /// Creates a handle for operations on a single Storage Bucket.
    ///
    /// No I/O is performed — this simply binds the namespace and name to the client.
    pub fn bucket(&self, namespace: impl Into<String>, repo: impl Into<String>) -> HFBucket {
        HFBucket {
            client: self.clone(),
            namespace: namespace.into(),
            bucket: repo.into(),
        }
    }
}
