use thiserror::Error;

#[derive(Error, Debug)]
pub enum GeweNoticeError {
    #[error("配置错误: {0}")]
    ConfigError(String),

    #[error("网络请求错误: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("JSON解析错误: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("UUID格式错误: {0}")]
    UuidError(String),

    #[error("微信机器人不在线")]
    BotOffline,

    #[error("API调用失败: {code} - {message}")]
    ApiError { code: i32, message: String },
}

pub type Result<T> = std::result::Result<T, GeweNoticeError>;
