use rmcp::{
    service::{serve_server, QuitReason},
    transport::stdio,
};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use gewe_notice_mcp::config::Config;
use gewe_notice_mcp::gewe_api::GeweApiClient;
use gewe_notice_mcp::server::GeweNoticeServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .with_target(false)
        .init();

    info!("Starting gewe-notice-mcp MCP 服务器...");

    let config = match Config::parse() {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("配置错误: {}", e);
            error!("请检查您的 MCP 配置文件中的环境变量。");
            std::process::exit(1);
        }
    };

    let api_client = match GeweApiClient::new(config.clone()) {
        Ok(client) => client,
        Err(e) => {
            error!("初始化 HTTP 客户端失败: {}", e);
            std::process::exit(1);
        }
    };

    info!("初始化阶段将在会话建立后检查机器人在线状态...");

    info!("配置加载成功 (来自环境变量):");
    info!("   - Base URL: {}", config.base_url);
    info!(
        "   - Token:    {}",
        config.redact(&config.token_str(), 2, 2)
    );
    info!(
        "   - App ID:   {}",
        config.redact(config.app_id_str(), 3, 4)
    );
    info!("   - WXID:     {}", config.redact(config.wxid_str(), 2, 2));
    if let Some(at_list) = config.normalized_at_list() {
        if at_list.len() == 1 && at_list[0] == "all" {
            info!("   - At List:  all");
        } else {
            let redacted_list: Vec<String> =
                at_list.iter().map(|at| config.redact(at, 2, 2)).collect();
            info!("   - At List:  {:?}", redacted_list);
        }
    }
    info!("{}", "-".repeat(20));

    let server = GeweNoticeServer::new(api_client);
    let (stdin, stdout) = stdio();

    info!("MCP 服务器已启动，等待连接...");

    let running_service = match serve_server(server, (stdin, stdout)).await {
        Ok(service) => service,
        Err(e) => {
            error!("MCP 服务器错误: {:?}", e);
            std::process::exit(1);
        }
    };

    match running_service.waiting().await {
        Ok(QuitReason::Closed) => {
            info!("MCP 服务器正常关闭");
        }
        Ok(QuitReason::Cancelled) => {
            info!("MCP 服务器已取消");
        }
        Ok(QuitReason::JoinError(err)) | Err(err) => {
            error!("MCP 服务器任务异常: {}", err);
            std::process::exit(1);
        }
    }

    Ok(())
}
