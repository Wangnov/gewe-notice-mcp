use rmcp::{
    handler::server::ServerHandler,
    model::{
        CallToolRequestMethod, CallToolRequestParam, CallToolResult, Content, ErrorData,
        Implementation, InitializeRequestParam, InitializeResult, JsonObject, ListToolsResult,
        LoggingLevel, LoggingMessageNotificationParam, PaginatedRequestParam, ServerCapabilities,
        SetLevelRequestParam, Tool, ToolsCapability,
    },
    service::{RequestContext, RoleServer},
};
use serde_json::{json, Value};
use std::future::Future;
use std::sync::{
    atomic::{AtomicU8, Ordering},
    Arc,
};
use std::time::Duration;
#[cfg(test)]
use tokio::sync::Mutex;
use tokio::sync::RwLock;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;

use crate::gewe_api::GeweApiClient;
use crate::uploader::{DynUploader, UploadRequest};

#[derive(Clone)]
pub struct GeweNoticeServer {
    api_client: Arc<GeweApiClient>,
    file_uploader: Option<DynUploader>,
    peer: Arc<RwLock<Option<rmcp::service::Peer<RoleServer>>>>,
    min_log_level: Arc<AtomicU8>,
    #[cfg(test)]
    log_tap: Arc<Mutex<Vec<LoggingMessageNotificationParam>>>,
}

impl GeweNoticeServer {
    pub fn new(api_client: GeweApiClient, file_uploader: Option<DynUploader>) -> Self {
        Self {
            api_client: Arc::new(api_client),
            file_uploader,
            peer: Arc::new(RwLock::new(None)),
            min_log_level: Arc::new(AtomicU8::new(Self::level_value(LoggingLevel::Info))),
            #[cfg(test)]
            log_tap: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn has_file_uploader(&self) -> bool {
        self.file_uploader.is_some()
    }

    fn spawn_online_check(&self) {
        let server = self.clone();
        let api_client = self.api_client.clone();

        tokio::spawn(async move {
            tokio::task::yield_now().await;
            server
                .emit_log_message(LoggingLevel::Info, "正在检查微信机器人在线状态...")
                .await;

            match api_client.check_online().await {
                Ok(true) => {
                    server
                        .emit_log_message(LoggingLevel::Info, "机器人在线，准备就绪。")
                        .await;
                }
                Ok(false) => {
                    server
                        .emit_log_message(
                            LoggingLevel::Error,
                            "机器人当前不在线。请检查微信客户端或 Gewe 服务。",
                        )
                        .await;
                }
                Err(e) => {
                    server
                        .emit_log_message(LoggingLevel::Error, format!("在线状态检查失败: {}", e))
                        .await;
                }
            }
        });
    }

    #[cfg(test)]
    pub fn with_log_tap(
        api_client: GeweApiClient,
        file_uploader: Option<DynUploader>,
    ) -> (Self, Arc<Mutex<Vec<LoggingMessageNotificationParam>>>) {
        let server = Self {
            api_client: Arc::new(api_client),
            file_uploader,
            peer: Arc::new(RwLock::new(None)),
            min_log_level: Arc::new(AtomicU8::new(Self::level_value(LoggingLevel::Info))),
            log_tap: Arc::new(Mutex::new(Vec::new())),
        };
        let tap = server.log_tap.clone();
        (server, tap)
    }

    fn level_value(level: LoggingLevel) -> u8 {
        match level {
            LoggingLevel::Debug => 0,
            LoggingLevel::Info => 1,
            LoggingLevel::Notice => 2,
            LoggingLevel::Warning => 3,
            LoggingLevel::Error => 4,
            LoggingLevel::Critical => 5,
            LoggingLevel::Alert => 6,
            LoggingLevel::Emergency => 7,
        }
    }

    #[cfg(test)]
    pub fn test_level_value(level: LoggingLevel) -> u8 {
        Self::level_value(level)
    }

    fn store_min_level(&self, level: LoggingLevel) {
        self.min_log_level
            .store(Self::level_value(level), Ordering::Relaxed);
    }

    #[cfg(test)]
    pub fn test_set_min_level(&self, level: LoggingLevel) {
        self.store_min_level(level);
    }

    #[cfg(test)]
    pub fn test_min_level_value(&self) -> u8 {
        self.min_log_level.load(Ordering::Relaxed)
    }

    fn should_emit(&self, level: LoggingLevel) -> bool {
        let threshold = self.min_log_level.load(Ordering::Relaxed);
        Self::level_value(level) >= threshold
    }

    async fn emit_log_message(&self, level: LoggingLevel, message: impl Into<String>) {
        let message = message.into();
        match level {
            LoggingLevel::Debug => tracing::debug!("{}", message),
            LoggingLevel::Info | LoggingLevel::Notice => tracing::info!("{}", message),
            LoggingLevel::Warning => tracing::warn!("{}", message),
            _ => tracing::error!("{}", message),
        }

        let should_emit = self.should_emit(level);

        #[cfg(test)]
        if should_emit {
            self.log_tap
                .lock()
                .await
                .push(LoggingMessageNotificationParam {
                    level,
                    logger: Some("gewe-notice-mcp".to_string()),
                    data: Value::String(message.clone()),
                });
        }

        if !should_emit {
            return;
        }

        if let Some(peer) = self.peer.read().await.clone() {
            let notification = LoggingMessageNotificationParam {
                level,
                logger: Some("gewe-notice-mcp".to_string()),
                data: Value::String(message),
            };
            if let Err(err) = peer.notify_logging_message(notification).await {
                tracing::debug!(error = %err, "unable to forward MCP logging notification");
            }
        }
    }

    async fn set_peer(&self, peer: rmcp::service::Peer<RoleServer>) {
        self.peer.write().await.replace(peer);
    }

    #[cfg(test)]
    pub async fn test_emit_log(&self, level: LoggingLevel, message: impl Into<String>) {
        self.emit_log_message(level, message).await;
    }

    async fn handle_post_text(
        &self,
        params: serde_json::Value,
    ) -> Result<CallToolResult, ErrorData> {
        let content = params["content"]
            .as_str()
            .ok_or_else(|| ErrorData::invalid_params("content parameter is required", None))?;

        self.emit_log_message(LoggingLevel::Info, format!("收到发送通知请求: {}", content))
            .await;

        match self.api_client.post_text(content).await {
            Ok(_) => {
                self.emit_log_message(LoggingLevel::Info, format!("通知发送成功: {}", content))
                    .await;
                Ok(CallToolResult {
                    content: vec![Content::text(format!("通知已成功发送: {}", content))],
                    is_error: None,
                    meta: None,
                    structured_content: None,
                })
            }
            Err(e) => {
                self.emit_log_message(LoggingLevel::Error, format!("发送通知失败: {}", e))
                    .await;
                Err(ErrorData::internal_error(
                    format!("发送通知失败: {}", e),
                    None,
                ))
            }
        }
    }

    async fn handle_post_file(
        &self,
        params: serde_json::Value,
    ) -> Result<CallToolResult, ErrorData> {
        let uploader = self
            .file_uploader
            .as_ref()
            .cloned()
            .ok_or_else(ErrorData::method_not_found::<CallToolRequestMethod>)?;

        let file_name = params
            .get("file_name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| ErrorData::invalid_params("file_name 参数必须为非空字符串", None))?;

        let content_base64 = params
            .get("content_base64")
            .and_then(Value::as_str)
            .ok_or_else(|| ErrorData::invalid_params("content_base64 参数缺失", None))?;

        let bytes = STANDARD
            .decode(content_base64)
            .map_err(|_| ErrorData::invalid_params("content_base64 不是有效的 Base64", None))?;

        if bytes.is_empty() {
            return Err(ErrorData::invalid_params("文件内容不能为空", None));
        }

        let content_type = params
            .get("content_type")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty());

        let ttl = match params.get("ttl_seconds") {
            Some(Value::Number(num)) => {
                let secs = num
                    .as_u64()
                    .ok_or_else(|| ErrorData::invalid_params("ttl_seconds 必须为非负整数", None))?;
                Some(Duration::from_secs(secs))
            }
            Some(Value::Null) | None => None,
            _ => {
                return Err(ErrorData::invalid_params(
                    "ttl_seconds 必须为非负整数",
                    None,
                ))
            }
        };

        self.emit_log_message(
            LoggingLevel::Info,
            format!("收到文件上传请求: {}", file_name),
        )
        .await;

        let upload_result = match uploader
            .upload(UploadRequest {
                file_name,
                content: &bytes,
                content_type,
                ttl,
            })
            .await
        {
            Ok(result) => result,
            Err(err) => {
                self.emit_log_message(LoggingLevel::Error, format!("上传文件到存储失败: {}", err))
                    .await;
                return Err(ErrorData::internal_error(
                    format!("上传文件失败: {err}"),
                    None,
                ));
            }
        };

        let gwe_result = match self
            .api_client
            .post_file(file_name, &upload_result.file_url)
            .await
        {
            Ok(info) => info,
            Err(err) => {
                self.emit_log_message(
                    LoggingLevel::Error,
                    format!("调用 GeWe postFile 失败: {}", err),
                )
                .await;
                return Err(ErrorData::internal_error(
                    format!("调用 GeWe postFile 失败: {err}"),
                    None,
                ));
            }
        };

        let summary = format!("文件已发送: {} -> {}", file_name, upload_result.file_url);
        self.emit_log_message(LoggingLevel::Info, &summary).await;

        let expires_at = upload_result
            .expires_at
            .and_then(|ts| ts.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|dur| dur.as_secs());

        let structured = json!({
            "file_name": file_name,
            "file_url": upload_result.file_url,
            "file_size": upload_result.size,
            "expires_at": expires_at,
            "message": {
                "to_wxid": gwe_result.to_wxid,
                "msg_id": gwe_result.msg_id,
                "new_msg_id": gwe_result.new_msg_id,
                "type": gwe_result.message_type,
                "create_time": gwe_result.create_time,
            }
        });

        Ok(CallToolResult {
            content: vec![Content::text(summary)],
            is_error: None,
            meta: None,
            structured_content: Some(structured),
        })
    }
}

fn build_post_text_tool() -> Tool {
    let mut schema = serde_json::Map::new();
    schema.insert("type".to_string(), serde_json::json!("object"));

    let mut properties = serde_json::Map::new();
    let mut content_prop = serde_json::Map::new();
    content_prop.insert("type".to_string(), serde_json::json!("string"));
    content_prop.insert(
        "description".to_string(),
        serde_json::json!("要发送的通知文本内容"),
    );
    properties.insert(
        "content".to_string(),
        serde_json::Value::Object(content_prop),
    );

    schema.insert(
        "properties".to_string(),
        serde_json::Value::Object(properties),
    );
    schema.insert("required".to_string(), serde_json::json!(["content"]));

    Tool {
        name: "post_text".into(),
        title: Some("发送通知".into()),
        description: Some(
            "发送 AI 任务状态通知。Agent 应在任务完成或发生关键错误时调用此工具。".into(),
        ),
        input_schema: Arc::new(schema),
        output_schema: None,
        annotations: None,
        icons: None,
    }
}

fn build_post_file_tool() -> Tool {
    let mut schema = serde_json::Map::new();
    schema.insert("type".to_string(), serde_json::json!("object"));

    let mut properties = serde_json::Map::new();

    let mut name_prop = serde_json::Map::new();
    name_prop.insert("type".to_string(), serde_json::json!("string"));
    name_prop.insert(
        "description".to_string(),
        serde_json::json!("文件名，将在微信端展示"),
    );
    properties.insert(
        "file_name".to_string(),
        serde_json::Value::Object(name_prop),
    );

    let mut content_prop = serde_json::Map::new();
    content_prop.insert("type".to_string(), serde_json::json!("string"));
    content_prop.insert(
        "description".to_string(),
        serde_json::json!("文件内容的 Base64 编码"),
    );
    properties.insert(
        "content_base64".to_string(),
        serde_json::Value::Object(content_prop),
    );

    let mut mime_prop = serde_json::Map::new();
    mime_prop.insert("type".to_string(), serde_json::json!("string"));
    mime_prop.insert(
        "description".to_string(),
        serde_json::json!("可选的 MIME 类型"),
    );
    properties.insert(
        "content_type".to_string(),
        serde_json::Value::Object(mime_prop),
    );

    let mut ttl_prop = serde_json::Map::new();
    ttl_prop.insert("type".to_string(), serde_json::json!("integer"));
    ttl_prop.insert(
        "description".to_string(),
        serde_json::json!("可选的下载有效期（秒）"),
    );
    ttl_prop.insert("minimum".to_string(), serde_json::json!(0));
    properties.insert(
        "ttl_seconds".to_string(),
        serde_json::Value::Object(ttl_prop),
    );

    schema.insert(
        "properties".to_string(),
        serde_json::Value::Object(properties),
    );
    schema.insert(
        "required".to_string(),
        serde_json::json!(["file_name", "content_base64"]),
    );

    Tool {
        name: "post_file".into(),
        title: Some("发送文件".into()),
        description: Some("上传文件并通过微信机器人发送".into()),
        input_schema: Arc::new(schema),
        output_schema: None,
        annotations: None,
        icons: None,
    }
}

impl ServerHandler for GeweNoticeServer {
    fn initialize(
        &self,
        request: InitializeRequestParam,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<InitializeResult, ErrorData>> + Send + '_ {
        let server = self.clone();
        async move {
            if context.peer.peer_info().is_none() {
                context.peer.set_peer_info(request);
            }
            server.set_peer(context.peer.clone()).await;
            server.spawn_online_check();
            let log_server = server.clone();
            tokio::spawn(async move {
                tokio::task::yield_now().await;
                log_server
                    .emit_log_message(LoggingLevel::Info, "MCP 会话已初始化")
                    .await;
            });
            Ok(server.get_info())
        }
    }

    fn set_level(
        &self,
        request: SetLevelRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<(), ErrorData>> + Send + '_ {
        let server = self.clone();
        async move {
            server
                .emit_log_message(
                    LoggingLevel::Info,
                    format!("日志级别已设置为 {:?}", request.level),
                )
                .await;
            server.store_min_level(request.level);
            Ok(())
        }
    }

    fn get_info(&self) -> InitializeResult {
        InitializeResult {
            protocol_version: Default::default(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability { list_changed: None }),
                prompts: None,
                resources: None,
                completions: None,
                experimental: None,
                logging: Some(JsonObject::new()),
            },
            server_info: Implementation {
                name: "Gewe Notice Server".to_string(),
                version: "1.0.1".to_string(),
                title: None,
                website_url: None,
                icons: None,
            },
            instructions: Some("一个通过微信机器人发送通知的 MCP 服务器。".into()),
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let mut tools = vec![build_post_text_tool()];
        if self.has_file_uploader() {
            tools.push(build_post_file_tool());
        }

        Ok(ListToolsResult {
            tools,
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        match request.name.as_ref() {
            "post_text" => {
                let arguments = request.arguments.unwrap_or_default();
                self.handle_post_text(serde_json::Value::Object(arguments))
                    .await
            }
            "post_file" => {
                if self.has_file_uploader() {
                    let arguments = request.arguments.unwrap_or_default();
                    self.handle_post_file(serde_json::Value::Object(arguments))
                        .await
                } else {
                    Err(ErrorData::method_not_found::<CallToolRequestMethod>())
                }
            }
            _ => Err(ErrorData::method_not_found::<CallToolRequestMethod>()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, UploadConfig};
    use crate::gewe_api::GeweApiClient;
    use serde_json::Value;

    fn test_config() -> Config {
        use crate::config::{AppId, ValidatedToken, WxId};

        Config {
            base_url: "http://example.com".into(),
            token: ValidatedToken::new("00000000-0000-0000-0000-000000000000").expect("valid UUID"),
            app_id: AppId::new("wx_test".to_string()).expect("valid app_id"),
            wxid: WxId::new("wxid_test".to_string()).expect("valid wxid"),
            at_list: None,
            upload: UploadConfig::None,
        }
    }

    #[tokio::test]
    async fn log_respects_min_level_threshold() {
        let config = test_config();
        let client = GeweApiClient::new(config.clone()).expect("client");
        let (server, tap) = GeweNoticeServer::with_log_tap(client, None);

        server
            .test_emit_log(LoggingLevel::Debug, "debug message")
            .await;
        assert!(tap.lock().await.is_empty());

        server.test_set_min_level(LoggingLevel::Debug);
        server
            .test_emit_log(LoggingLevel::Debug, "debug message")
            .await;

        let events = tap.lock().await.clone();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].level, LoggingLevel::Debug);
        assert_eq!(events[0].data, Value::String("debug message".into()));
    }

    #[test]
    fn store_min_level_updates_atomic() {
        let config = test_config();
        let client = GeweApiClient::new(config.clone()).expect("client");
        let server = GeweNoticeServer::new(client, None);

        assert_eq!(
            server.test_min_level_value(),
            GeweNoticeServer::test_level_value(LoggingLevel::Info)
        );

        server.test_set_min_level(LoggingLevel::Warning);
        assert_eq!(
            server.test_min_level_value(),
            GeweNoticeServer::test_level_value(LoggingLevel::Warning)
        );
    }
}
