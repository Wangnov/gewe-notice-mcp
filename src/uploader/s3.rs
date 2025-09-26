use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use aws_config::meta::region::RegionProviderChain;
use aws_config::BehaviorVersion;
use aws_sdk_s3::config::Region;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use tracing::debug;

use super::{FileUploader, UploadError, UploadRequest, UploadResult};
use crate::config::S3UploadConfig;

#[derive(Clone)]
pub struct S3Uploader {
    client: Client,
    bucket: String,
    prefix: Option<String>,
    presign_max: Duration,
}

impl S3Uploader {
    pub async fn new(config: S3UploadConfig) -> anyhow::Result<Self> {
        let region_provider = RegionProviderChain::first_try(Region::new(config.region.clone()))
            .or_default_provider();

        let mut loader = aws_config::defaults(BehaviorVersion::latest()).region(region_provider);
        if let Some(endpoint) = &config.endpoint {
            loader = loader.endpoint_url(endpoint.clone());
        }

        let shared = loader.load().await;
        let client = Client::new(&shared);

        Ok(Self {
            client,
            bucket: config.bucket,
            prefix: config.prefix,
            presign_max: Duration::from_secs(60 * 60 * 24 * 7), // AWS presign 最大 7 天
        })
    }
}

#[async_trait]
impl FileUploader for S3Uploader {
    async fn upload(&self, request: UploadRequest<'_>) -> Result<UploadResult, UploadError> {
        let key = build_object_key(self.prefix.as_deref(), request.file_name);
        let body = ByteStream::from(request.content.to_vec());

        let mut put = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(body);

        if let Some(content_type) = request.content_type {
            put = put.content_type(content_type);
        }

        put.send()
            .await
            .map_err(|err| UploadError::Http(err.to_string()))?;

        let ttl = request.ttl.unwrap_or(Duration::from_secs(300));
        let ttl = ttl.min(self.presign_max);
        let expires_at = SystemTime::now() + ttl;

        let presign_cfg = PresigningConfig::expires_in(ttl)
            .map_err(|err| UploadError::Other(format!("生成预签名配置失败: {err}")))?;

        let presigned = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .presigned(presign_cfg)
            .await
            .map_err(|err| UploadError::Other(format!("生成下载链接失败: {err}")))?;

        let url = presigned.uri().to_string();
        debug!("S3 上传完成: {}", url);

        Ok(UploadResult {
            file_url: url,
            expires_at: Some(expires_at),
            size: request.content.len() as u64,
        })
    }
}

fn build_object_key(prefix: Option<&str>, file_name: &str) -> String {
    let safe_name = sanitize_filename::sanitize(file_name);
    let unique = uuid::Uuid::new_v4().simple().to_string();
    match prefix {
        Some(p) if !p.is_empty() => format!("{}/{}_{}", p, unique, safe_name),
        _ => format!("{}_{}", unique, safe_name),
    }
}
