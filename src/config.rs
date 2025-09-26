use crate::errors::{ConfigValidationError, Result, TokenValidationError, WxIdValidationError};
use clap::Parser;
use reqwest::Url;
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ValidatedToken(Uuid);

impl ValidatedToken {
    pub fn new(s: &str) -> Result<Self> {
        Uuid::parse_str(s).map(ValidatedToken).map_err(|e| {
            ConfigValidationError::InvalidToken {
                reason: TokenValidationError::ParseError(e),
            }
            .into()
        })
    }

    pub fn as_str(&self) -> String {
        self.0.to_string()
    }
}

impl fmt::Display for ValidatedToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct AppId(String);

impl AppId {
    pub fn new(s: String) -> Result<Self> {
        if !s.starts_with("wx_") {
            return Err(ConfigValidationError::InvalidAppId { value: s }.into());
        }
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AppId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct WxId(String);

impl WxId {
    pub fn new(s: String) -> Result<Self> {
        if s.is_empty() {
            return Err(ConfigValidationError::InvalidWxId {
                reason: WxIdValidationError::Empty,
            }
            .into());
        }

        // "all" 是特殊的@全员标识符
        if s.eq_ignore_ascii_case("all") {
            return Ok(Self("all".to_string()));
        }

        if s.contains("@chatroom") && !s.ends_with("@chatroom") {
            return Err(ConfigValidationError::InvalidWxId {
                reason: WxIdValidationError::InvalidChatroomFormat,
            }
            .into());
        }

        Ok(Self(s))
    }

    pub fn is_chatroom(&self) -> bool {
        self.0.contains("@chatroom")
    }

    pub fn is_all(&self) -> bool {
        self.0 == "all"
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WxId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Parser)]
#[clap(
    name = "gewe-notice-mcp",
    about = "一个通过微信机器人发送AI任务状态通知的轻量级工具",
    version
)]
pub struct RawConfig {
    #[clap(
        long,
        env = "GEWE_NOTICE_BASE_URL",
        default_value = "https://www.geweapi.com",
        help = "Gewe API 基础URL"
    )]
    pub base_url: String,

    #[clap(long, env = "GEWE_NOTICE_TOKEN", help = "Gewe API Token (UUID格式)")]
    pub token: String,

    #[clap(long, env = "GEWE_NOTICE_APP_ID", help = "微信机器人App ID (wx_开头)")]
    pub app_id: String,

    #[clap(long, env = "GEWE_NOTICE_WXID", help = "接收者WXID (个人或群聊)")]
    pub wxid: String,

    #[clap(
        long,
        env = "GEWE_NOTICE_AT_LIST",
        value_delimiter = ',',
        help = "@列表 (逗号分隔的wxid或'all')"
    )]
    pub at_list: Option<Vec<String>>,

    #[clap(long, env = "GEWE_NOTICE_UPLOAD_MODE", help = "上传模式: server 或 s3")]
    pub upload_mode: Option<String>,

    #[clap(
        long,
        env = "GEWE_NOTICE_UPLOAD_SERVER_URL",
        help = "gewe-notice-server 的上传接口 URL，例如 https://files.example.com/upload"
    )]
    pub upload_server_url: Option<String>,

    #[clap(
        long,
        env = "GEWE_NOTICE_UPLOAD_SERVER_API_KEY",
        help = "调用上传服务器时附带的 API Key，可选"
    )]
    pub upload_server_api_key: Option<String>,

    #[clap(long, env = "GEWE_NOTICE_S3_BUCKET", help = "S3 Bucket 名称")]
    pub s3_bucket: Option<String>,

    #[clap(
        long,
        env = "GEWE_NOTICE_S3_REGION",
        help = "S3 所在区域，如 ap-southeast-1"
    )]
    pub s3_region: Option<String>,

    #[clap(long, env = "GEWE_NOTICE_S3_PREFIX", help = "S3 对象键前缀，可选")]
    pub s3_prefix: Option<String>,

    #[clap(
        long,
        env = "GEWE_NOTICE_S3_ENDPOINT",
        help = "S3 兼容存储 Endpoint，可选"
    )]
    pub s3_endpoint: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub base_url: String,
    pub token: ValidatedToken,
    pub app_id: AppId,
    pub wxid: WxId,
    pub at_list: Option<Vec<WxId>>,
    pub upload: UploadConfig,
}

impl Config {
    pub fn parse() -> Result<Self> {
        let raw = RawConfig::parse();
        Self::from_raw(raw)
    }

    pub fn from_raw(raw: RawConfig) -> Result<Self> {
        let RawConfig {
            base_url,
            token,
            app_id,
            wxid,
            at_list,
            upload_mode,
            upload_server_url,
            upload_server_api_key,
            s3_bucket,
            s3_region,
            s3_prefix,
            s3_endpoint,
        } = raw;

        let upload = UploadConfig::from_parts(
            upload_mode,
            upload_server_url,
            upload_server_api_key,
            s3_bucket,
            s3_region,
            s3_prefix,
            s3_endpoint,
        )?;

        let token = ValidatedToken::new(&token)?;
        let app_id = AppId::new(app_id)?;
        let wxid = WxId::new(wxid)?;
        let at_list = parse_at_list(at_list)?;

        Ok(Self {
            base_url,
            token,
            app_id,
            wxid,
            at_list,
            upload,
        })
    }

    pub fn is_chatroom(&self) -> bool {
        self.wxid.is_chatroom()
    }

    pub fn redact(&self, value: &str, show_first: usize, show_last: usize) -> String {
        if value.len() <= show_first + show_last {
            "****".to_string()
        } else {
            format!(
                "{}****{}",
                &value[..show_first],
                &value[value.len() - show_last..]
            )
        }
    }

    pub fn normalized_at_list(&self) -> Option<Vec<String>> {
        self.at_list
            .as_ref()
            .map(|list| list.iter().map(|wxid| wxid.as_str().to_string()).collect())
    }

    pub fn token_str(&self) -> String {
        self.token.as_str()
    }

    pub fn app_id_str(&self) -> &str {
        self.app_id.as_str()
    }

    pub fn wxid_str(&self) -> &str {
        self.wxid.as_str()
    }
}

fn parse_at_list(raw: Option<Vec<String>>) -> Result<Option<Vec<WxId>>> {
    match raw {
        Some(list) => {
            let mut validated = Vec::new();
            for id in list {
                let trimmed = id.trim();
                if trimmed.is_empty() {
                    continue;
                }
                validated.push(WxId::new(trimmed.to_string())?);
            }
            if validated.is_empty() {
                Ok(None)
            } else {
                Ok(Some(validated))
            }
        }
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_raw_config() -> RawConfig {
        RawConfig {
            base_url: "https://example.com".into(),
            token: "00000000-0000-0000-0000-000000000000".into(),
            app_id: "wx_app".into(),
            wxid: "wxid_sample".into(),
            at_list: None,
            upload_mode: None,
            upload_server_url: None,
            upload_server_api_key: None,
            s3_bucket: None,
            s3_region: None,
            s3_prefix: None,
            s3_endpoint: None,
        }
    }

    #[test]
    fn normalized_handles_none() {
        let raw = base_raw_config();
        let config = Config::from_raw(raw).expect("valid config");
        assert!(config.normalized_at_list().is_none());
        assert!(matches!(config.upload, UploadConfig::None));
    }

    #[test]
    fn normalized_trims_and_filters_empty_entries() {
        let mut raw = base_raw_config();
        raw.at_list = Some(vec![" wxid_1 ".into(), " ".into(), "wxid_2".into()]);

        let config = Config::from_raw(raw).expect("valid config");
        let result = config.normalized_at_list().expect("normalized list");
        assert_eq!(result, vec!["wxid_1".to_string(), "wxid_2".to_string()]);
    }

    #[test]
    fn normalized_handles_all_case_insensitively() {
        let mut raw = base_raw_config();
        raw.at_list = Some(vec![" ALL ".into()]);

        let config = Config::from_raw(raw).expect("valid config");
        let result = config.normalized_at_list().expect("all entry");
        assert_eq!(result, vec!["all".to_string()]);
    }

    #[test]
    fn test_invalid_token() {
        let mut raw = base_raw_config();
        raw.token = "not-a-uuid".into();

        let result = Config::from_raw(raw);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_app_id() {
        let mut raw = base_raw_config();
        raw.app_id = "invalid_app_id".into();

        let result = Config::from_raw(raw);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_wxid() {
        let mut raw = base_raw_config();
        raw.wxid = "".into();

        let result = Config::from_raw(raw);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_chatroom_format() {
        let mut raw = base_raw_config();
        raw.wxid = "something@chatroommore".into();

        let result = Config::from_raw(raw);
        assert!(result.is_err());
    }

    #[test]
    fn upload_server_requires_url() {
        let mut raw = base_raw_config();
        raw.upload_mode = Some("server".into());

        let result = Config::from_raw(raw);
        assert!(result.is_err());
    }

    #[test]
    fn upload_s3_requires_bucket_and_region() {
        let mut raw = base_raw_config();
        raw.upload_mode = Some("s3".into());
        raw.s3_bucket = Some("".into());
        raw.s3_region = Some("ap-southeast-1".into());

        assert!(Config::from_raw(raw).is_err());
    }
}

#[derive(Debug, Clone)]
pub enum UploadConfig {
    None,
    Server(ServerUploadConfig),
    S3(S3UploadConfig),
}

#[derive(Debug, Clone)]
pub struct ServerUploadConfig {
    pub endpoint: Url,
    pub api_key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct S3UploadConfig {
    pub bucket: String,
    pub region: String,
    pub prefix: Option<String>,
    pub endpoint: Option<String>,
}

impl UploadConfig {
    #[allow(clippy::too_many_arguments)]
    pub fn from_parts(
        mode: Option<String>,
        server_url: Option<String>,
        server_api_key: Option<String>,
        s3_bucket: Option<String>,
        s3_region: Option<String>,
        s3_prefix: Option<String>,
        s3_endpoint: Option<String>,
    ) -> std::result::Result<Self, ConfigValidationError> {
        let has_server_fields = server_url
            .as_ref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false)
            || server_api_key
                .as_ref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false);

        let has_s3_fields = s3_bucket
            .as_ref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false)
            || s3_region
                .as_ref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false)
            || s3_prefix
                .as_ref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false)
            || s3_endpoint
                .as_ref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false);

        let mode = mode
            .as_ref()
            .map(|m| m.trim().to_ascii_lowercase())
            .filter(|m| !m.is_empty());

        match mode.as_deref() {
            None => {
                if has_server_fields || has_s3_fields {
                    return Err(ConfigValidationError::ConflictingUploadConfig {
                        reason: "未指定 GEWE_NOTICE_UPLOAD_MODE，但存在上传相关配置".into(),
                    });
                }
                Ok(UploadConfig::None)
            }
            Some("server") => {
                if has_s3_fields {
                    return Err(ConfigValidationError::ConflictingUploadConfig {
                        reason: "server 模式下不应设置 S3 相关环境变量".into(),
                    });
                }

                let endpoint_raw = server_url.ok_or(ConfigValidationError::MissingUploadField {
                    field: "GEWE_NOTICE_UPLOAD_SERVER_URL",
                })?;
                let endpoint_trimmed = endpoint_raw.trim();
                let endpoint = Url::parse(endpoint_trimmed).map_err(|_| {
                    ConfigValidationError::InvalidUrl {
                        value: endpoint_raw.clone(),
                    }
                })?;
                if endpoint.scheme() != "http" && endpoint.scheme() != "https" {
                    return Err(ConfigValidationError::InvalidUrl {
                        value: endpoint_raw,
                    });
                }

                let api_key = server_api_key.and_then(|k| {
                    let trimmed = k.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                });

                Ok(UploadConfig::Server(ServerUploadConfig {
                    endpoint,
                    api_key,
                }))
            }
            Some("s3") => {
                #[cfg(not(feature = "upload-s3"))]
                {
                    return Err(ConfigValidationError::InvalidUploadMode {
                        value: "s3 (需要启用 upload-s3 特性)".into(),
                    });
                }

                #[cfg(feature = "upload-s3")]
                {
                    if has_server_fields {
                        return Err(ConfigValidationError::ConflictingUploadConfig {
                            reason: "s3 模式下不应设置上传服务器相关环境变量".into(),
                        });
                    }

                    let bucket_raw =
                        s3_bucket.ok_or(ConfigValidationError::MissingUploadField {
                            field: "GEWE_NOTICE_S3_BUCKET",
                        })?;
                    let bucket = bucket_raw.trim();
                    if bucket.is_empty() {
                        return Err(ConfigValidationError::MissingUploadField {
                            field: "GEWE_NOTICE_S3_BUCKET",
                        });
                    }

                    let region_raw =
                        s3_region.ok_or(ConfigValidationError::MissingUploadField {
                            field: "GEWE_NOTICE_S3_REGION",
                        })?;
                    let region = region_raw.trim();
                    if region.is_empty() {
                        return Err(ConfigValidationError::MissingUploadField {
                            field: "GEWE_NOTICE_S3_REGION",
                        });
                    }

                    let prefix = s3_prefix.and_then(|p| {
                        let trimmed = p.trim();
                        if trimmed.is_empty() {
                            None
                        } else {
                            Some(trimmed.trim_matches('/').to_string())
                        }
                    });

                    let endpoint = s3_endpoint.map(|e| {
                        let trimmed = e.trim();
                        if trimmed.is_empty() {
                            return Ok(String::new());
                        }
                        Url::parse(trimmed)
                            .map(|_| trimmed.to_string())
                            .map_err(|_| ConfigValidationError::InvalidUrl {
                                value: trimmed.to_string(),
                            })
                    });

                    let endpoint = match endpoint {
                        Some(Ok(v)) if v.is_empty() => None,
                        Some(Ok(v)) => Some(v),
                        Some(Err(e)) => return Err(e),
                        None => None,
                    };

                    Ok(UploadConfig::S3(S3UploadConfig {
                        bucket: bucket.to_string(),
                        region: region.to_string(),
                        prefix,
                        endpoint,
                    }))
                }
            }
            Some(other) => Err(ConfigValidationError::InvalidUploadMode {
                value: other.to_string(),
            }),
        }
    }
}
