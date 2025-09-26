use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use tokio::fs;
use tokio::sync::RwLock;
use tokio::time::Instant;
use uuid::Uuid;

use super::config::SanitizedConfig;
use super::error::UploadServerError;

#[derive(Debug, Clone)]
pub struct FileStore {
    inner: Arc<RwLock<HashMap<String, FileRecord>>>,
    root: PathBuf,
    max_file_bytes: u64,
    default_ttl: Duration,
    min_ttl: Duration,
}

#[derive(Debug, Clone)]
struct FileRecord {
    path: PathBuf,
    file_name: String,
    content_type: Option<String>,
    size: u64,
    expires_at: SystemTime,
}

#[derive(Debug, Clone)]
pub struct SavedFile {
    pub id: String,
    pub file_name: String,
    pub size: u64,
    pub content_type: Option<String>,
    pub expires_at: SystemTime,
}

#[derive(Debug, Clone)]
pub struct StoredFile {
    pub file_name: String,
    pub content_type: Option<String>,
    pub size: u64,
    pub path: PathBuf,
    pub expires_at: SystemTime,
}

impl FileStore {
    pub async fn new(config: &SanitizedConfig) -> anyhow::Result<Self> {
        if !config.storage_dir.exists() {
            fs::create_dir_all(&config.storage_dir).await?;
        }

        Ok(Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
            root: config.storage_dir.clone(),
            max_file_bytes: config.max_file_bytes,
            default_ttl: config.default_ttl,
            min_ttl: config.min_ttl,
        })
    }

    pub fn clone_handle(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            root: self.root.clone(),
            max_file_bytes: self.max_file_bytes,
            default_ttl: self.default_ttl,
            min_ttl: self.min_ttl,
        }
    }

    pub async fn save(
        &self,
        file_name: &str,
        bytes: &[u8],
        content_type: Option<String>,
        ttl: Option<Duration>,
    ) -> Result<SavedFile, UploadServerError> {
        let size = bytes.len() as u64;
        if size > self.max_file_bytes {
            return Err(UploadServerError::FileTooLarge {
                max_bytes: self.max_file_bytes,
            });
        }

        let safe_name = sanitize_filename::sanitize(file_name);
        if safe_name.is_empty() {
            return Err(UploadServerError::BadRequest("文件名不能为空".to_string()));
        }

        let id = Uuid::new_v4().simple().to_string();
        let file_path = self.root.join(&id);
        fs::write(&file_path, bytes)
            .await
            .map_err(|err| UploadServerError::Internal(format!("写入文件失败: {err}")))?;

        let expires_at = compute_expiry(ttl, self.default_ttl, self.min_ttl);

        let record = FileRecord {
            path: file_path,
            file_name: file_name.to_string(),
            content_type,
            size,
            expires_at,
        };

        self.inner.write().await.insert(id.clone(), record.clone());

        Ok(SavedFile {
            id,
            file_name: record.file_name,
            size,
            content_type: record.content_type,
            expires_at,
        })
    }

    pub async fn get(&self, id: &str) -> Result<StoredFile, UploadServerError> {
        let guard = self.inner.read().await;
        if let Some(record) = guard.get(id) {
            if record.expires_at <= SystemTime::now() {
                drop(guard);
                self.delete(id).await.ok();
                return Err(UploadServerError::NotFound);
            }
            Ok(StoredFile {
                file_name: record.file_name.clone(),
                content_type: record.content_type.clone(),
                size: record.size,
                path: record.path.clone(),
                expires_at: record.expires_at,
            })
        } else {
            Err(UploadServerError::NotFound)
        }
    }

    pub async fn delete(&self, id: &str) -> Result<(), UploadServerError> {
        let mut guard = self.inner.write().await;
        if let Some(record) = guard.remove(id) {
            if let Err(err) = fs::remove_file(&record.path).await {
                if err.kind() != std::io::ErrorKind::NotFound {
                    return Err(UploadServerError::Internal(format!("删除文件失败: {err}")));
                }
            }
        }
        Ok(())
    }

    pub async fn cleanup_expired(&self) {
        let ids: Vec<String> = {
            let guard = self.inner.read().await;
            guard
                .iter()
                .filter_map(|(id, record)| {
                    if record.expires_at <= SystemTime::now() {
                        Some(id.clone())
                    } else {
                        None
                    }
                })
                .collect()
        };

        for id in ids {
            let _ = self.delete(&id).await;
        }
    }
}

fn compute_expiry(ttl: Option<Duration>, default_ttl: Duration, min_ttl: Duration) -> SystemTime {
    let ttl = ttl.unwrap_or(default_ttl);
    let ttl = ttl.max(min_ttl);
    SystemTime::now() + ttl
}

pub fn schedule_cleanup(store: FileStore, interval: Duration) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval_at(Instant::now() + interval, interval);
        loop {
            ticker.tick().await;
            store.cleanup_expired().await;
        }
    });
}

pub fn percent_encode_filename(name: &str) -> String {
    use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
    utf8_percent_encode(name, NON_ALPHANUMERIC).to_string()
}

impl StoredFile {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    pub fn content_type(&self) -> Option<&str> {
        self.content_type.as_deref()
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn expires_at(&self) -> SystemTime {
        self.expires_at
    }
}
