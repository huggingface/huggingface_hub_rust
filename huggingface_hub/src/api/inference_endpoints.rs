use crate::client::HFClient;
use crate::error::Result;
use crate::types::{
    CreateInferenceEndpointParams, DeleteInferenceEndpointParams, GetInferenceEndpointParams, InferenceEndpointInfo,
    ListInferenceEndpointsParams, PauseInferenceEndpointParams, ResumeInferenceEndpointParams,
    ScaleToZeroInferenceEndpointParams, UpdateInferenceEndpointParams,
};

const IE_API_BASE: &str = "https://api.endpoints.huggingface.cloud/v2/endpoint";

impl HFClient {
    async fn resolve_ie_namespace(&self, namespace: &Option<String>) -> Result<String> {
        match namespace {
            Some(ns) => Ok(ns.clone()),
            None => {
                let user = self.whoami().await?;
                Ok(user.username)
            },
        }
    }

    pub async fn create_inference_endpoint(
        &self,
        params: &CreateInferenceEndpointParams,
    ) -> Result<InferenceEndpointInfo> {
        let ns = self.resolve_ie_namespace(&params.namespace).await?;
        let url = format!("{IE_API_BASE}/{ns}");
        let mut body = serde_json::json!({
            "name": params.name,
            "model": {
                "repository": params.repository,
                "framework": params.framework,
                "task": params.task,
            },
            "provider": {
                "vendor": params.vendor,
                "region": params.region,
            },
            "compute": {
                "accelerator": params.accelerator,
                "instanceSize": params.instance_size,
                "instanceType": params.instance_type,
            },
        });
        if let Some(ref revision) = params.revision {
            body["model"]["revision"] = serde_json::json!(revision);
        }
        if let Some(min) = params.min_replica {
            body["compute"]["scaling"] = serde_json::json!({"minReplica": min});
        }
        if let Some(max) = params.max_replica {
            body["compute"]["scaling"]["maxReplica"] = serde_json::json!(max);
        }
        if let Some(ref endpoint_type) = params.endpoint_type {
            body["type"] = serde_json::json!(endpoint_type);
        }
        if let Some(ref custom_image) = params.custom_image {
            body["model"]["image"] = custom_image.clone();
        }
        if let Some(ref secrets) = params.secrets {
            body["model"]["secrets"] = serde_json::json!(secrets);
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

    pub async fn get_inference_endpoint(&self, params: &GetInferenceEndpointParams) -> Result<InferenceEndpointInfo> {
        let ns = self.resolve_ie_namespace(&params.namespace).await?;
        let url = format!("{IE_API_BASE}/{ns}/{}", params.name);
        let response = self.inner.client.get(&url).headers(self.auth_headers()).send().await?;
        let response = self
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn list_inference_endpoints(
        &self,
        params: &ListInferenceEndpointsParams,
    ) -> Result<Vec<InferenceEndpointInfo>> {
        let ns = self.resolve_ie_namespace(&params.namespace).await?;
        let url = format!("{IE_API_BASE}/{ns}");
        let response = self.inner.client.get(&url).headers(self.auth_headers()).send().await?;
        let response = self
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        let wrapper: serde_json::Value = response.json().await?;
        let items = wrapper.get("items").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let endpoints: Vec<InferenceEndpointInfo> = serde_json::from_value(serde_json::json!(items))?;
        Ok(endpoints)
    }

    pub async fn update_inference_endpoint(
        &self,
        params: &UpdateInferenceEndpointParams,
    ) -> Result<InferenceEndpointInfo> {
        let ns = self.resolve_ie_namespace(&params.namespace).await?;
        let url = format!("{IE_API_BASE}/{ns}/{}", params.name);
        let mut body = serde_json::Map::new();
        let mut compute = serde_json::Map::new();
        let mut model = serde_json::Map::new();
        if let Some(ref acc) = params.accelerator {
            compute.insert("accelerator".into(), serde_json::json!(acc));
        }
        if let Some(ref size) = params.instance_size {
            compute.insert("instanceSize".into(), serde_json::json!(size));
        }
        if let Some(ref itype) = params.instance_type {
            compute.insert("instanceType".into(), serde_json::json!(itype));
        }
        if params.min_replica.is_some() || params.max_replica.is_some() || params.scale_to_zero_timeout.is_some() {
            let mut scaling = serde_json::Map::new();
            if let Some(min) = params.min_replica {
                scaling.insert("minReplica".into(), serde_json::json!(min));
            }
            if let Some(max) = params.max_replica {
                scaling.insert("maxReplica".into(), serde_json::json!(max));
            }
            if let Some(timeout) = params.scale_to_zero_timeout {
                scaling.insert("scaleToZeroTimeout".into(), serde_json::json!(timeout));
            }
            compute.insert("scaling".into(), serde_json::json!(scaling));
        }
        if let Some(ref repo) = params.repository {
            model.insert("repository".into(), serde_json::json!(repo));
        }
        if let Some(ref fw) = params.framework {
            model.insert("framework".into(), serde_json::json!(fw));
        }
        if let Some(ref rev) = params.revision {
            model.insert("revision".into(), serde_json::json!(rev));
        }
        if let Some(ref task) = params.task {
            model.insert("task".into(), serde_json::json!(task));
        }
        if let Some(ref img) = params.custom_image {
            model.insert("image".into(), img.clone());
        }
        if let Some(ref secrets) = params.secrets {
            model.insert("secrets".into(), serde_json::json!(secrets));
        }
        if !compute.is_empty() {
            body.insert("compute".into(), serde_json::json!(compute));
        }
        if !model.is_empty() {
            body.insert("model".into(), serde_json::json!(model));
        }
        let response = self
            .inner
            .client
            .put(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await?;
        let response = self
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn delete_inference_endpoint(&self, params: &DeleteInferenceEndpointParams) -> Result<()> {
        let ns = self.resolve_ie_namespace(&params.namespace).await?;
        let url = format!("{IE_API_BASE}/{ns}/{}", params.name);
        let response = self.inner.client.delete(&url).headers(self.auth_headers()).send().await?;
        self.check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(())
    }

    pub async fn pause_inference_endpoint(
        &self,
        params: &PauseInferenceEndpointParams,
    ) -> Result<InferenceEndpointInfo> {
        let ns = self.resolve_ie_namespace(&params.namespace).await?;
        let url = format!("{IE_API_BASE}/{ns}/{}/pause", params.name);
        let response = self.inner.client.post(&url).headers(self.auth_headers()).send().await?;
        let response = self
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn resume_inference_endpoint(
        &self,
        params: &ResumeInferenceEndpointParams,
    ) -> Result<InferenceEndpointInfo> {
        let ns = self.resolve_ie_namespace(&params.namespace).await?;
        let url = format!("{IE_API_BASE}/{ns}/{}/resume", params.name);
        let response = self.inner.client.post(&url).headers(self.auth_headers()).send().await?;
        let response = self
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }

    pub async fn scale_to_zero_inference_endpoint(
        &self,
        params: &ScaleToZeroInferenceEndpointParams,
    ) -> Result<InferenceEndpointInfo> {
        let ns = self.resolve_ie_namespace(&params.namespace).await?;
        let url = format!("{IE_API_BASE}/{ns}/{}/scale-to-zero", params.name);
        let response = self.inner.client.post(&url).headers(self.auth_headers()).send().await?;
        let response = self
            .check_response(response, None, crate::error::NotFoundContext::Generic)
            .await?;
        Ok(response.json().await?)
    }
}

sync_api! {
    impl HFClientSync {
        fn create_inference_endpoint(&self, params: &CreateInferenceEndpointParams) -> Result<InferenceEndpointInfo>;
        fn get_inference_endpoint(&self, params: &GetInferenceEndpointParams) -> Result<InferenceEndpointInfo>;
        fn list_inference_endpoints(&self, params: &ListInferenceEndpointsParams) -> Result<Vec<InferenceEndpointInfo>>;
        fn update_inference_endpoint(&self, params: &UpdateInferenceEndpointParams) -> Result<InferenceEndpointInfo>;
        fn delete_inference_endpoint(&self, params: &DeleteInferenceEndpointParams) -> Result<()>;
        fn pause_inference_endpoint(&self, params: &PauseInferenceEndpointParams) -> Result<InferenceEndpointInfo>;
        fn resume_inference_endpoint(&self, params: &ResumeInferenceEndpointParams) -> Result<InferenceEndpointInfo>;
        fn scale_to_zero_inference_endpoint(&self, params: &ScaleToZeroInferenceEndpointParams) -> Result<InferenceEndpointInfo>;
    }
}
