use crate::client::HfApi;
use crate::error::Result;
use crate::types::{
    DailyPaper, ListDailyPapersParams, ListPapersParams, PaperInfo, PaperInfoParams,
    PaperSearchResult,
};

impl HfApi {
    pub async fn list_papers(&self, params: &ListPapersParams) -> Result<Vec<PaperSearchResult>> {
        let url = format!("{}/api/papers/search", self.inner.endpoint);
        let mut query: Vec<(String, String)> = Vec::new();
        if let Some(ref q) = params.query {
            query.push(("q".into(), q.clone()));
        }
        if let Some(limit) = params.limit {
            query.push(("limit".into(), limit.to_string()));
        }
        let response = self
            .inner
            .client
            .get(&url)
            .headers(self.auth_headers())
            .query(&query)
            .send()
            .await?;
        let response = self
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn list_daily_papers(
        &self,
        params: &ListDailyPapersParams,
    ) -> Result<Vec<DailyPaper>> {
        let url = format!("{}/api/daily_papers", self.inner.endpoint);
        let mut query: Vec<(String, String)> = Vec::new();
        if let Some(ref date) = params.date {
            query.push(("date".into(), date.clone()));
        }
        if let Some(ref week) = params.week {
            query.push(("week".into(), week.clone()));
        }
        if let Some(ref month) = params.month {
            query.push(("month".into(), month.clone()));
        }
        if let Some(ref submitter) = params.submitter {
            query.push(("submitter".into(), submitter.clone()));
        }
        if let Some(ref sort) = params.sort {
            query.push(("sort".into(), sort.clone()));
        }
        if let Some(p) = params.p {
            query.push(("p".into(), p.to_string()));
        }
        if let Some(limit) = params.limit {
            query.push(("limit".into(), limit.to_string()));
        }
        let response = self
            .inner
            .client
            .get(&url)
            .headers(self.auth_headers())
            .query(&query)
            .send()
            .await?;
        let response = self
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn paper_info(&self, params: &PaperInfoParams) -> Result<PaperInfo> {
        let url = format!("{}/api/papers/{}", self.inner.endpoint, params.paper_id);
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
}

sync_api! {
    impl HfApiSync {
        fn list_papers(&self, params: &ListPapersParams) -> Result<Vec<PaperSearchResult>>;
        fn list_daily_papers(&self, params: &ListDailyPapersParams) -> Result<Vec<DailyPaper>>;
        fn paper_info(&self, params: &PaperInfoParams) -> Result<PaperInfo>;
    }
}
