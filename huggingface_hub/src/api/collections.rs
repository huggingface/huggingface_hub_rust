use crate::client::HfApi;
use crate::error::Result;
use crate::types::{
    AddCollectionItemParams, Collection, CollectionItem, CreateCollectionParams,
    DeleteCollectionItemParams, DeleteCollectionParams, GetCollectionParams, ListCollectionsParams,
    UpdateCollectionItemParams, UpdateCollectionMetadataParams,
};

impl HfApi {
    pub async fn get_collection(&self, params: &GetCollectionParams) -> Result<Collection> {
        let url = format!("{}/api/collections/{}", self.inner.endpoint, params.slug);
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

    pub async fn list_collections(
        &self,
        params: &ListCollectionsParams,
    ) -> Result<Vec<Collection>> {
        let url = format!("{}/api/collections", self.inner.endpoint);
        let mut query: Vec<(String, String)> = Vec::new();
        if let Some(ref owner) = params.owner {
            query.push(("owner".into(), owner.clone()));
        }
        if let Some(ref item) = params.item {
            query.push(("item".into(), item.clone()));
        }
        if let Some(ref item_type) = params.item_type {
            query.push(("item_type".into(), item_type.clone()));
        }
        if let Some(limit) = params.limit {
            query.push(("limit".into(), limit.to_string()));
        }
        if let Some(offset) = params.offset {
            query.push(("offset".into(), offset.to_string()));
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

    pub async fn create_collection(&self, params: &CreateCollectionParams) -> Result<Collection> {
        let url = format!("{}/api/collections", self.inner.endpoint);
        let mut body = serde_json::json!({ "title": params.title });
        if let Some(ref desc) = params.description {
            body["description"] = serde_json::json!(desc);
        }
        if let Some(private) = params.private {
            body["private"] = serde_json::json!(private);
        }
        if let Some(ref ns) = params.namespace {
            body["namespace"] = serde_json::json!(ns);
        }
        let response = self
            .inner
            .client
            .post(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;
        let response = self
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn update_collection_metadata(
        &self,
        params: &UpdateCollectionMetadataParams,
    ) -> Result<Collection> {
        let url = format!("{}/api/collections/{}", self.inner.endpoint, params.slug);
        let mut body = serde_json::Map::new();
        if let Some(ref title) = params.title {
            body.insert("title".into(), serde_json::json!(title));
        }
        if let Some(ref desc) = params.description {
            body.insert("description".into(), serde_json::json!(desc));
        }
        if let Some(private) = params.private {
            body.insert("private".into(), serde_json::json!(private));
        }
        if let Some(position) = params.position {
            body.insert("position".into(), serde_json::json!(position));
        }
        if let Some(ref theme) = params.theme {
            body.insert("theme".into(), serde_json::json!(theme));
        }
        let response = self
            .inner
            .client
            .patch(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;
        let response = self
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn delete_collection(&self, params: &DeleteCollectionParams) -> Result<()> {
        let url = format!("{}/api/collections/{}", self.inner.endpoint, params.slug);
        let response = self
            .inner
            .client
            .delete(&url)
            .headers(self.auth_headers())
            .send()
            .await?;
        if response.status().as_u16() == 404 && params.missing_ok {
            return Ok(());
        }
        self.check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(())
    }

    pub async fn add_collection_item(
        &self,
        params: &AddCollectionItemParams,
    ) -> Result<Collection> {
        let url = format!(
            "{}/api/collections/{}/items",
            self.inner.endpoint, params.slug
        );
        let mut body = serde_json::json!({
            "item_id": params.item_id,
            "item_type": params.item_type,
        });
        if let Some(ref note) = params.note {
            body["note"] = serde_json::json!(note);
        }
        let response = self
            .inner
            .client
            .post(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;
        if response.status().as_u16() == 409 && params.exists_ok {
            return self
                .get_collection(&GetCollectionParams::builder().slug(&params.slug).build())
                .await;
        }
        let response = self
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn update_collection_item(
        &self,
        params: &UpdateCollectionItemParams,
    ) -> Result<CollectionItem> {
        let url = format!(
            "{}/api/collections/{}/items/{}",
            self.inner.endpoint, params.slug, params.item_object_id
        );
        let mut body = serde_json::Map::new();
        if let Some(ref note) = params.note {
            body.insert("note".into(), serde_json::json!(note));
        }
        if let Some(position) = params.position {
            body.insert("position".into(), serde_json::json!(position));
        }
        let response = self
            .inner
            .client
            .patch(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;
        let response = self
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn delete_collection_item(&self, params: &DeleteCollectionItemParams) -> Result<()> {
        let url = format!(
            "{}/api/collections/{}/items/{}",
            self.inner.endpoint, params.slug, params.item_object_id
        );
        let response = self
            .inner
            .client
            .delete(&url)
            .headers(self.auth_headers())
            .send()
            .await?;
        self.check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(())
    }
}
