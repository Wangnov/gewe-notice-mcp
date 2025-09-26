use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum ConfigValidationError {
    #[error("Token格式无效: {reason}")]
    InvalidToken { reason: TokenValidationError },

    #[error("AppId格式无效: 应以 'wx_' 开头，实际值: {value}")]
    InvalidAppId { value: String },

    #[error("WxId格式无效: {reason}")]
    InvalidWxId { reason: WxIdValidationError },

    #[error("上传模式无效: {value}")]
    InvalidUploadMode { value: String },

    #[error("上传配置缺少字段: {field}")]
    MissingUploadField { field: &'static str },

    #[error("上传配置冲突: {reason}")]
    ConflictingUploadConfig { reason: String },

    #[error("URL 无效: {value}")]
    InvalidUrl { value: String },
}

#[derive(Error, Debug, Clone)]
pub enum TokenValidationError {
    #[error("不是有效的UUID格式")]
    NotUuid,

    #[error("UUID解析失败: {0}")]
    ParseError(#[from] uuid::Error),
}

#[derive(Error, Debug, Clone)]
pub enum WxIdValidationError {
    #[error("WxId不能为空")]
    Empty,

    #[error("群聊ID格式无效: 应以 '@chatroom' 结尾")]
    InvalidChatroomFormat,
}

#[derive(Error, Debug, Clone)]
pub enum NetworkError {
    #[error("请求超时 (持续时间: {duration:?})")]
    Timeout { duration: Duration },

    #[error("连接被拒绝")]
    ConnectionRefused,

    #[error("DNS解析失败: {host}")]
    DnsResolution { host: String },

    #[error("TLS/SSL错误")]
    TlsError,

    #[error("HTTP错误 (状态码: {status}, 响应: {body:?})")]
    HttpError { status: u16, body: Option<String> },

    #[error("底层网络错误: {0}")]
    Underlying(String),
}

impl From<reqwest::Error> for NetworkError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            NetworkError::Timeout {
                duration: Duration::from_secs(10),
            }
        } else if err.is_connect() {
            NetworkError::ConnectionRefused
        } else if let Some(url) = err.url() {
            let error_str = err.to_string();
            if error_str.contains("dns") || error_str.contains("resolve") {
                NetworkError::DnsResolution {
                    host: url.host_str().unwrap_or("unknown").to_string(),
                }
            } else if error_str.contains("ssl")
                || error_str.contains("tls")
                || error_str.contains("certificate")
            {
                NetworkError::TlsError
            } else {
                NetworkError::Underlying(error_str)
            }
        } else {
            NetworkError::Underlying(err.to_string())
        }
    }
}

#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiErrorCode {
    #[error("不在群内")]
    NotInGroup = -219,

    #[error("群聊不存在")]
    ChatroomMissing = -104,

    #[error("无权限")]
    PermissionDenied = -2,
}

impl ApiErrorCode {
    pub fn from_code(code: i32) -> Option<Self> {
        match code {
            -219 => Some(Self::NotInGroup),
            -104 => Some(Self::ChatroomMissing),
            -2 => Some(Self::PermissionDenied),
            _ => None,
        }
    }

    pub fn as_i32(self) -> i32 {
        self as i32
    }
}

#[derive(Error, Debug, Clone)]
pub enum ApiBusinessError {
    #[error("微信机器人不在线")]
    BotOffline,

    #[error("已知API错误: {code}")]
    KnownError { code: ApiErrorCode },

    #[error("未知API错误 (代码: {code}, 消息: {message})")]
    UnknownError { code: i32, message: String },

    #[error("请求被限流 (重试时间: {retry_after:?})")]
    RateLimited { retry_after: Option<Duration> },
}

#[derive(Error, Debug, Clone)]
pub enum GeweNoticeError {
    #[error("配置验证失败: {0}")]
    Config(#[from] ConfigValidationError),

    #[error("网络错误: {0}")]
    Network(#[from] NetworkError),

    #[error("业务逻辑错误: {0}")]
    Business(#[from] ApiBusinessError),

    #[error("JSON解析错误: {0}")]
    Json(String),

    #[error("任务被取消")]
    Cancelled,
}

impl From<serde_json::Error> for GeweNoticeError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err.to_string())
    }
}

impl GeweNoticeError {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Network(NetworkError::Timeout { .. })
                | Self::Network(NetworkError::ConnectionRefused)
                | Self::Business(ApiBusinessError::RateLimited { .. })
        )
    }

    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            Self::Config(_)
                | Self::Business(ApiBusinessError::BotOffline)
                | Self::Business(ApiBusinessError::KnownError {
                    code: ApiErrorCode::NotInGroup | ApiErrorCode::ChatroomMissing
                })
        )
    }

    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::Business(ApiBusinessError::RateLimited { retry_after }) => *retry_after,
            Self::Network(NetworkError::Timeout { .. }) => Some(Duration::from_secs(1)),
            _ => None,
        }
    }
}

pub type Result<T> = std::result::Result<T, GeweNoticeError>;
