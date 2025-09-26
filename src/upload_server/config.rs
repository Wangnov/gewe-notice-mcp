use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;

#[derive(Debug, Clone, Parser)]
#[command(name = "gewe-notice-server", about = "GeWe 通知文件上传服务", version)]
pub struct ServerConfig {
    /// HTTP 监听地址
    #[arg(long, env = "GEWE_SERVER_BIND", default_value = "127.0.0.1:8989")]
    pub bind: SocketAddr,

    /// 对外可访问的基础 URL（用于拼接下载链接）
    #[arg(
        long,
        env = "GEWE_SERVER_PUBLIC_BASE_URL",
        help = "对外暴露文件的基础 URL，示例：https://files.example.com"
    )]
    pub public_base_url: String,

    /// 文件缓存目录
    #[arg(
        long,
        env = "GEWE_SERVER_STORAGE_DIR",
        default_value = "gewe_cached_files"
    )]
    pub storage_dir: PathBuf,

    /// 单个文件大小上限（字节）
    #[arg(
        long,
        env = "GEWE_SERVER_MAX_BYTES",
        default_value_t = 20 * 1024 * 1024
    )]
    pub max_file_bytes: u64,

    /// 默认过期时间（秒）
    #[arg(long, env = "GEWE_SERVER_DEFAULT_TTL", default_value_t = 300)]
    pub default_ttl_secs: u64,

    /// 最小允许的过期时间（秒）
    #[arg(long, env = "GEWE_SERVER_MIN_TTL", default_value_t = 60)]
    pub min_ttl_secs: u64,

    /// 清理任务执行间隔（秒）
    #[arg(long, env = "GEWE_SERVER_CLEANUP_INTERVAL", default_value_t = 60)]
    pub cleanup_interval_secs: u64,
}

impl ServerConfig {
    pub fn sanitize(self) -> anyhow::Result<SanitizedConfig> {
        if self.public_base_url.trim().is_empty() {
            anyhow::bail!("GEWE_SERVER_PUBLIC_BASE_URL 不能为空");
        }
        let public_base_url = self.public_base_url.trim_end_matches('/').to_string();

        let storage_dir = if self.storage_dir.is_absolute() {
            self.storage_dir
        } else {
            std::env::current_dir()?.join(self.storage_dir)
        };

        if self.max_file_bytes == 0 {
            anyhow::bail!("GEWE_SERVER_MAX_BYTES 必须大于 0");
        }

        if self.min_ttl_secs == 0 {
            anyhow::bail!("GEWE_SERVER_MIN_TTL 必须大于 0");
        }

        let default_ttl_secs = self.default_ttl_secs.max(self.min_ttl_secs);

        Ok(SanitizedConfig {
            bind: self.bind,
            public_base_url,
            storage_dir,
            max_file_bytes: self.max_file_bytes,
            default_ttl: Duration::from_secs(default_ttl_secs),
            min_ttl: Duration::from_secs(self.min_ttl_secs),
            cleanup_interval: Duration::from_secs(self.cleanup_interval_secs.max(10)),
        })
    }
}

#[derive(Debug, Clone)]
pub struct SanitizedConfig {
    pub bind: SocketAddr,
    pub public_base_url: String,
    pub storage_dir: PathBuf,
    pub max_file_bytes: u64,
    pub default_ttl: Duration,
    pub min_ttl: Duration,
    pub cleanup_interval: Duration,
}
