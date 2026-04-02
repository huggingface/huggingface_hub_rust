use futures::Stream;
use url::Url;

use crate::client::HFClient;
use crate::constants;
use crate::error::Result;
use crate::types::{LikeParams, LikedRepo, ListLikedReposParams, ListRepoLikersParams, User};

impl crate::repository::HFRepository {
    pub async fn like(&self, params: &LikeParams) -> Result<()> {
        let url = format!("{}/like", self.client.api_url(params.repo_type, &params.repo_id));
        let response = self
            .client
            .inner
            .client
            .post(&url)
            .headers(self.client.auth_headers())
            .send()
            .await?;
        self.client
            .check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(())
    }

    pub async fn unlike(&self, params: &LikeParams) -> Result<()> {
        let url = format!("{}/like", self.client.api_url(params.repo_type, &params.repo_id));
        let response = self
            .client
            .inner
            .client
            .delete(&url)
            .headers(self.client.auth_headers())
            .send()
            .await?;
        self.client
            .check_response(response, Some(&params.repo_id), crate::error::NotFoundContext::Repo)
            .await?;
        Ok(())
    }

    pub fn list_repo_likers(&self, params: &ListRepoLikersParams) -> Result<impl Stream<Item = Result<User>> + '_> {
        let segment = constants::repo_type_api_segment(params.repo_type);
        let url_str = format!("{}/api/{}/{}/likers", self.client.inner.endpoint, segment, params.repo_id);
        let url = Url::parse(&url_str)?;
        Ok(self.client.paginate(url, vec![], params.max_items))
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
