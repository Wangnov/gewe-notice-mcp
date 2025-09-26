use std::sync::Arc;

use clap::Parser;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;

use gewe_notice_mcp::upload_server::{config::ServerConfig, http, storage};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    let raw_config = ServerConfig::parse();
    let config = raw_config.sanitize()?;

    info!("启动 gewe-notice-server...");
    info!("监听地址: {}", config.bind);
    info!("文件基址: {}", config.public_base_url);
    info!("缓存目录: {}", config.storage_dir.display());
    info!("最大文件大小: {} 字节", config.max_file_bytes);

    let store = storage::FileStore::new(&config).await?;
    storage::schedule_cleanup(store.clone_handle(), config.cleanup_interval);

    let state = http::AppState {
        config: Arc::new(config.clone()),
        store: store.clone_handle(),
    };

    let router = http::build_router(state);

    let listener = TcpListener::bind(config.bind).await?;
    axum::serve(listener, router)
        .await
        .map_err(|err| anyhow::anyhow!("服务运行失败: {err}"))
}
