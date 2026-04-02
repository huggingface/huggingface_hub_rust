use futures::Stream;
use url::Url;

use crate::client::HFClient;
use crate::constants;
use crate::error::Result;
use crate::types::{LikedRepo, ListLikedReposParams, User};

impl crate::repository::HFRepository {
    /// Like this repository.
    pub async fn like(&self) -> Result<()> {
        let repo_path = self.repo_path();
        let url = format!("{}/like", self.client.api_url(Some(self.repo_type), &repo_path));
        let response = self
            .client
            .inner
            .client
            .post(&url)
            .headers(self.client.auth_headers())
            .send()
            .await?;
        self.client
            .check_response(response, Some(&repo_path), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(())
    }

    /// Unlike this repository.
    pub async fn unlike(&self) -> Result<()> {
        let repo_path = self.repo_path();
        let url = format!("{}/like", self.client.api_url(Some(self.repo_type), &repo_path));
        let response = self
            .client
            .inner
            .client
            .delete(&url)
            .headers(self.client.auth_headers())
            .send()
            .await?;
        self.client
            .check_response(response, Some(&repo_path), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(())
    }

    /// Stream users who have liked this repository.
    ///
    /// Returns `Result<impl Stream<Item = Result<User>>>`. Pass `max_items` to cap the total
    /// number of users yielded.
    pub fn list_likers(&self, max_items: Option<usize>) -> Result<impl Stream<Item = Result<User>> + '_> {
        let segment = constants::repo_type_api_segment(Some(self.repo_type));
        let url_str = format!("{}/api/{}/{}/likers", self.client.inner.endpoint, segment, self.repo_path());
        let url = Url::parse(&url_str)?;
        Ok(self.client.paginate(url, vec![], max_items))
    }
}

impl HFClient {
    pub async fn list_liked_repos(&self, params: &ListLikedReposParams) -> Result<Vec<LikedRepo>> {
        let url = format!("{}/api/users/{}/likes", self.inner.endpoint, params.username);
        let response = self.inner.client.get(&url).headers(self.auth_headers()).send().await?;
        let response = self
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }
}

sync_api! {
    impl HfApiSync {
        fn list_liked_repos(&self, params: &ListLikedReposParams) -> Result<Vec<LikedRepo>>;
    }
}
