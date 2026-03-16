use crate::client::HfApi;
use crate::constants;
use crate::error::Result;
use crate::types::{LikeParams, LikedRepo, ListLikedReposParams, ListRepoLikersParams, User};
use futures::Stream;
use url::Url;

impl HfApi {
    pub async fn like(&self, params: &LikeParams) -> Result<()> {
        let url = format!("{}/like", self.api_url(params.repo_type, &params.repo_id));
        let response = self
            .inner
            .client
            .post(&url)
            .headers(self.auth_headers())
            .send()
            .await?;
        self.check_response(
            response,
            Some(&params.repo_id),
            crate::error::NotFoundContext::Repo,
        )
        .await?;
        Ok(())
    }

    pub async fn unlike(&self, params: &LikeParams) -> Result<()> {
        let url = format!("{}/like", self.api_url(params.repo_type, &params.repo_id));
        let response = self
            .inner
            .client
            .delete(&url)
            .headers(self.auth_headers())
            .send()
            .await?;
        self.check_response(
            response,
            Some(&params.repo_id),
            crate::error::NotFoundContext::Repo,
        )
        .await?;
        Ok(())
    }

    pub async fn list_liked_repos(&self, params: &ListLikedReposParams) -> Result<Vec<LikedRepo>> {
        let url = format!(
            "{}/api/users/{}/likes",
            self.inner.endpoint, params.username
        );
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

    pub fn list_repo_likers(
        &self,
        params: &ListRepoLikersParams,
    ) -> impl Stream<Item = Result<User>> + '_ {
        let segment = constants::repo_type_api_segment(params.repo_type);
        let url_str = format!(
            "{}/api/{}/{}/likers",
            self.inner.endpoint, segment, params.repo_id
        );
        let url = Url::parse(&url_str).unwrap();
        self.paginate(url, vec![])
    }
}
