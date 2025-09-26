use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};

use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use tokio::fs;
use tracing::info;

use super::config::SanitizedConfig;
use super::error::UploadServerError;
use super::storage::{percent_encode_filename, FileStore};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<SanitizedConfig>,
    pub store: FileStore,
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/upload", post(handle_upload))
        .route("/files/:id/:filename", get(handle_download))
        .route("/files/:id", delete(handle_delete))
        .route("/health", get(handle_health))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UploadPayload {
    file_name: String,
    content_base64: String,
    #[serde(default)]
    content_type: Option<String>,
    #[serde(default)]
    ttl_seconds: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UploadResponse {
    file_id: String,
    file_url: String,
    expires_at: u64,
    size: u64,
}

async fn handle_upload(
    State(state): State<AppState>,
    Json(payload): Json<UploadPayload>,
) -> Result<impl IntoResponse, UploadServerError> {
    if payload.file_name.trim().is_empty() {
        return Err(UploadServerError::BadRequest("file_name 不能为空".into()));
    }

    let bytes = STANDARD
        .decode(payload.content_base64.as_bytes())
        .map_err(|_| UploadServerError::BadRequest("content_base64 不是有效的 base64".into()))?;

    let ttl = payload.ttl_seconds.map(Duration::from_secs);

    let saved = state
        .store
        .save(
            payload.file_name.trim(),
            &bytes,
            payload.content_type.clone(),
            ttl,
        )
        .await?;

    let url = format!(
        "{}/files/{}/{}",
        state.config.public_base_url,
        saved.id,
        percent_encode_filename(&saved.file_name)
    );

    let expires_at = saved
        .expires_at
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    info!("文件上传成功: {} -> {}", saved.file_name, url);

    Ok((
        StatusCode::CREATED,
        Json(UploadResponse {
            file_id: saved.id,
            file_url: url,
            expires_at,
            size: saved.size,
        }),
    ))
}

async fn handle_download(
    State(state): State<AppState>,
    Path((id, _filename)): Path<(String, String)>,
) -> Result<Response, UploadServerError> {
    let record = state.store.get(&id).await?;
    let data = fs::read(record.path())
        .await
        .map_err(|err| UploadServerError::Internal(format!("读取文件失败: {err}")))?;

    let mut headers = HeaderMap::new();
    let content_type = record
        .content_type()
        .unwrap_or("application/octet-stream")
        .to_string();
    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_str(&content_type)
            .unwrap_or_else(|_| header::HeaderValue::from_static("application/octet-stream")),
    );
    let disposition = format!("attachment; filename=\"{}\"", record.file_name());
    headers.insert(
        header::CONTENT_DISPOSITION,
        header::HeaderValue::from_str(&disposition)
            .unwrap_or_else(|_| header::HeaderValue::from_static("attachment")),
    );

    Ok((StatusCode::OK, headers, data).into_response())
}

async fn handle_delete(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, UploadServerError> {
    state.store.delete(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn handle_health() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}
