use std::time::{Duration, SystemTime};

use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::debug;

use super::{FileUploader, UploadError, UploadRequest, UploadResult};
use crate::config::ServerUploadConfig;
use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;

#[derive(Clone)]
pub struct ServerUploader {
    client: reqwest::Client,
    endpoint: reqwest::Url,
    api_key: Option<String>,
}

impl ServerUploader {
    pub fn new(config: ServerUploadConfig) -> anyhow::Result<Self> {
        let client = reqwest::Client::builder()
            .pool_max_idle_per_host(2)
            .build()?;

        Ok(Self {
            client,
            endpoint: config.endpoint,
            api_key: config.api_key,
        })
    }
}

#[async_trait]
impl FileUploader for ServerUploader {
    async fn upload(&self, request: UploadRequest<'_>) -> Result<UploadResult, UploadError> {
        let payload = ServerUploadPayload {
            file_name: request.file_name,
            content_base64: STANDARD.encode(request.content),
            content_type: request.content_type.map(|s| s.to_string()),
            ttl_seconds: request.ttl.map(|ttl| ttl.as_secs()),
        };

        let mut builder = self.client.post(self.endpoint.clone()).json(&payload);
        if let Some(api_key) = &self.api_key {
            builder = builder.header("X-API-Key", api_key);
        }

        let response = builder
            .send()
            .await
            .map_err(|err| UploadError::Http(err.to_string()))?;

        if response.status() == StatusCode::CREATED {
            let resp: ServerUploadResponse = response
                .json()
                .await
                .map_err(|err| UploadError::InvalidResponse(err.to_string()))?;

            let expires_at =
                SystemTime::UNIX_EPOCH.checked_add(Duration::from_secs(resp.expires_at));
            debug!("文件上传成功: {}", resp.file_url);
            Ok(UploadResult {
                file_url: resp.file_url,
                expires_at,
                size: resp.size,
            })
        } else {
            let status = response.status();
            let text = response
                .text()
                .await
                .unwrap_or_else(|_| "<failed to read body>".into());
            Err(UploadError::Http(format!(
                "上传服务器返回非 201 状态码 {}: {}",
                status, text
            )))
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServerUploadPayload<'a> {
    file_name: &'a str,
    content_base64: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ttl_seconds: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServerUploadResponse {
    file_url: String,
    expires_at: u64,
    size: u64,
}
