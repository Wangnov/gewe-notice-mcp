use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UploadServerError {
    #[error("无效请求: {0}")]
    BadRequest(String),
    #[error("文件超出大小限制({max_bytes} 字节)")]
    FileTooLarge { max_bytes: u64 },
    #[error("内部错误: {0}")]
    Internal(String),
    #[error("文件不存在或已过期")]
    NotFound,
}

impl IntoResponse for UploadServerError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            UploadServerError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            UploadServerError::FileTooLarge { max_bytes } => (
                StatusCode::BAD_REQUEST,
                format!("文件大小超过限制: 最大 {} 字节", max_bytes),
            ),
            UploadServerError::NotFound => (StatusCode::GONE, self.to_string()),
            UploadServerError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        let body = axum::Json(ErrorBody { message });
        (status, body).into_response()
    }
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    message: String,
}

impl From<anyhow::Error> for UploadServerError {
    fn from(value: anyhow::Error) -> Self {
        Self::Internal(value.to_string())
    }
}
