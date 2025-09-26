mod s3;
mod server;

use std::sync::Arc;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;

use crate::config::UploadConfig;

pub use s3::S3Uploader;
pub use server::ServerUploader;

#[derive(Debug, thiserror::Error)]
pub enum UploadError {
    #[error("HTTP 请求失败: {0}")]
    Http(String),
    #[error("响应解析失败: {0}")]
    InvalidResponse(String),
    #[error("上传失败: {0}")]
    Other(String),
}

pub struct UploadRequest<'a> {
    pub file_name: &'a str,
    pub content: &'a [u8],
    pub content_type: Option<&'a str>,
    pub ttl: Option<Duration>,
}

pub struct UploadResult {
    pub file_url: String,
    pub expires_at: Option<SystemTime>,
    pub size: u64,
}

#[async_trait]
pub trait FileUploader: Send + Sync {
    async fn upload(&self, request: UploadRequest<'_>) -> Result<UploadResult, UploadError>;
}

pub type DynUploader = Arc<dyn FileUploader>;

pub async fn build_uploader(config: &UploadConfig) -> anyhow::Result<Option<DynUploader>> {
    match config {
        UploadConfig::None => Ok(None),
        UploadConfig::Server(server_cfg) => {
            let uploader = ServerUploader::new(server_cfg.clone())?;
            Ok(Some(Arc::new(uploader)))
        }
        UploadConfig::S3(s3_cfg) => {
            let uploader = S3Uploader::new(s3_cfg.clone()).await?;
            Ok(Some(Arc::new(uploader)))
        }
    }
}
