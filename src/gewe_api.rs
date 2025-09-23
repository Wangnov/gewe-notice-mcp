use reqwest::{header, Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::{sleep, timeout};
use tracing::{error, info, warn};

use crate::config::Config;
use crate::errors::{ApiBusinessError, ApiErrorCode, NetworkError, Result};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CheckOnlineRequest {
    app_id: String,
}

#[derive(Debug, Deserialize)]
struct CheckOnlineResponse {
    ret: i32,
    msg: String,
    data: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GetChatroomMemberListRequest {
    app_id: String,
    chatroom_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChatroomMember {
    wxid: String,
    nick_name: String,
    display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChatroomMemberData {
    member_list: Vec<ChatroomMember>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetChatroomMemberListResponse {
    ret: i32,
    msg: String,
    data: Option<ChatroomMemberData>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct PostTextRequest {
    app_id: String,
    to_wxid: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    ats: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PostTextResponse {
    ret: i32,
    msg: String,
    data: Option<PostTextData>,
}

#[derive(Debug, Deserialize)]
struct PostTextData {
    code: Option<String>,
}

#[derive(Debug, Clone)]
struct RetryPolicy {
    max_retries: u8,
    initial_delay: Duration,
    max_delay: Duration,
    exponential_base: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            exponential_base: 2.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ApiRet {
    Success,
    Failure(i32),
}

impl ApiRet {
    fn is_success(self) -> bool {
        matches!(self, Self::Success)
    }

    fn code(self) -> i32 {
        match self {
            Self::Success => 200,
            Self::Failure(code) => code,
        }
    }
}

impl From<i32> for ApiRet {
    fn from(value: i32) -> Self {
        if value == 200 {
            Self::Success
        } else {
            Self::Failure(value)
        }
    }
}

impl PostTextResponse {
    fn ret_status(&self) -> ApiRet {
        ApiRet::from(self.ret)
    }

    fn is_success(&self) -> bool {
        self.ret_status().is_success() && !self.msg.contains("失败")
    }

    fn failure_code(&self) -> Option<ApiErrorCode> {
        self.data
            .as_ref()
            .and_then(|data| data.code.as_deref())
            .and_then(|code| code.parse::<i32>().ok())
            .and_then(ApiErrorCode::from_code)
    }
}

struct PostTextCall {
    status: StatusCode,
    body: String,
    response: PostTextResponse,
}

#[derive(Clone)]
pub struct GeweApiClient {
    client: Client,
    config: Config,
    semaphore: Arc<Semaphore>,
    retry_policy: RetryPolicy,
    request_timeout: Duration,
}

impl GeweApiClient {
    pub fn new(config: Config) -> Result<Self> {
        let client = Client::builder()
            .pool_max_idle_per_host(5)
            .pool_idle_timeout(Duration::from_secs(30))
            .tcp_keepalive(Duration::from_secs(60))
            .build()
            .map_err(NetworkError::from)?;

        Ok(Self {
            client,
            config,
            semaphore: Arc::new(Semaphore::new(10)),
            retry_policy: RetryPolicy::default(),
            request_timeout: Duration::from_secs(10),
        })
    }

    pub async fn check_online(&self) -> Result<bool> {
        info!("正在检查微信机器人在线状态...");

        let url = format!("{}/gewe/v2/api/login/checkOnline", self.config.base_url);
        let request = CheckOnlineRequest {
            app_id: self.config.app_id_str().to_string(),
        };

        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|_| NetworkError::ConnectionRefused)?;

        let response = timeout(
            self.request_timeout,
            self.client
                .post(&url)
                .header("X-GEWE-TOKEN", self.config.token_str())
                .json(&request)
                .send(),
        )
        .await
        .map_err(|_| NetworkError::Timeout {
            duration: self.request_timeout,
        })?
        .map_err(NetworkError::from)?;

        if response.status().is_success() {
            let data: CheckOnlineResponse = response.json().await.map_err(NetworkError::from)?;

            if data.ret != 200 {
                error!(
                    "在线状态检查返回异常 (ret: {}, msg: {})",
                    data.ret, data.msg
                );
                return Err(ApiBusinessError::UnknownError {
                    code: data.ret,
                    message: data.msg,
                }
                .into());
            }

            if let Some(true) = data.data {
                info!("机器人在线，准备就绪。");
                Ok(true)
            } else {
                error!("机器人当前不在线。");
                error!("   - App ID: {}", self.config.app_id_str());
                Err(ApiBusinessError::BotOffline.into())
            }
        } else {
            let status = response.status();
            let text = response.text().await.map_err(NetworkError::from)?;
            error!("在线状态检查失败，HTTP 状态码: {}", status);
            error!("   - 响应内容: {}", text);
            Err(NetworkError::HttpError {
                status: status.as_u16(),
                body: Some(text),
            }
            .into())
        }
    }

    async fn get_chatroom_member_names(
        &self,
        chatroom_id: &str,
    ) -> Result<HashMap<String, String>> {
        info!("正在为群 {} 获取成员列表...", chatroom_id);

        let url = format!(
            "{}/gewe/v2/api/group/getChatroomMemberList",
            self.config.base_url
        );
        let request = GetChatroomMemberListRequest {
            app_id: self.config.app_id_str().to_string(),
            chatroom_id: chatroom_id.to_string(),
        };

        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|_| NetworkError::ConnectionRefused)?;

        let response = timeout(
            self.request_timeout,
            self.client
                .post(&url)
                .header("X-GEWE-TOKEN", self.config.token_str())
                .header(header::CONTENT_TYPE, "application/json")
                .json(&request)
                .send(),
        )
        .await
        .map_err(|_| NetworkError::Timeout {
            duration: self.request_timeout,
        })?
        .map_err(NetworkError::from)?;

        if response.status().is_success() {
            let data: GetChatroomMemberListResponse =
                response.json().await.map_err(NetworkError::from)?;

            if data.ret != 200 {
                if data.ret == 500 && data.msg == "获取群成员列表异常:null" {
                    error!(
                        "获取群成员列表失败: 你可能已不在群 {} 内或该群聊不存在。",
                        chatroom_id
                    );
                    return Err(ApiBusinessError::KnownError {
                        code: ApiErrorCode::NotInGroup,
                    }
                    .into());
                } else {
                    error!(
                        "获取群成员列表失败 (ret: {}, msg: {})，chatroom_id: {}",
                        data.ret, data.msg, chatroom_id
                    );
                }
                return Err(ApiBusinessError::UnknownError {
                    code: data.ret,
                    message: data.msg,
                }
                .into());
            }

            if let Some(member_data) = data.data {
                let name_map: HashMap<String, String> = member_data
                    .member_list
                    .into_iter()
                    .map(|member| {
                        let name = member.display_name.unwrap_or(member.nick_name);
                        (member.wxid, name)
                    })
                    .collect();

                info!("成功获取并解析群成员列表。");
                Ok(name_map)
            } else {
                warn!("警告: 获取到空的群成员列表。");
                Ok(HashMap::new())
            }
        } else {
            let status = response.status();
            let text = response.text().await.map_err(NetworkError::from)?;
            error!("获取群成员列表失败，状态码: {}, 响应: {}", status, text);
            Err(NetworkError::HttpError {
                status: status.as_u16(),
                body: Some(text),
            }
            .into())
        }
    }

    pub async fn post_text(&self, content: &str) -> Result<()> {
        info!("准备发送通知: '{}'", content);

        let operation = timeout(Duration::from_secs(30), self.post_text_with_retry(content));

        operation.await.map_err(|_| NetworkError::Timeout {
            duration: Duration::from_secs(30),
        })?
    }

    async fn post_text_with_retry(&self, content: &str) -> Result<()> {
        let mut attempts = 0;
        let mut last_error = None;

        while attempts < self.retry_policy.max_retries {
            match self.post_text_internal(content).await {
                Ok(()) => return Ok(()),
                Err(e) if e.is_retryable() => {
                    last_error = Some(e.clone());
                    attempts += 1;

                    if attempts < self.retry_policy.max_retries {
                        let delay = self.calculate_backoff(attempts);
                        warn!(
                            "重试 {}/{}: 等待 {:?}",
                            attempts, self.retry_policy.max_retries, delay
                        );
                        sleep(delay).await;
                    }
                }
                Err(e) => return Err(e),
            }
        }

        Err(last_error.unwrap_or_else(|| {
            NetworkError::HttpError {
                status: 0,
                body: Some("最大重试次数已超过".to_string()),
            }
            .into()
        }))
    }

    fn calculate_backoff(&self, attempt: u8) -> Duration {
        let exponential = self.retry_policy.initial_delay.as_millis() as f64
            * self.retry_policy.exponential_base.powi(attempt as i32 - 1);

        let jittered = exponential * (0.5 + rand::random::<f64>() * 0.5);

        Duration::from_millis(jittered.min(self.retry_policy.max_delay.as_millis() as f64) as u64)
    }

    async fn post_text_internal(&self, content: &str) -> Result<()> {
        let mut final_content = content.to_string();
        let mut ats_payload = None;
        let normalized_at_list = self.config.normalized_at_list();
        let is_at_all = matches!(normalized_at_list.as_ref(), Some(list) if list.len() == 1 && list[0] == "all");

        if self.config.is_chatroom() {
            if let Some(ref at_list) = normalized_at_list {
                info!("检测到群聊 @ 请求，正在处理...");

                if is_at_all {
                    ats_payload = Some("notify@all".to_string());
                    final_content = format!("@所有人 {}", content);
                    info!("已将 @ 全体成员，并在内容中添加 @ 所有人。");
                } else {
                    match self.get_chatroom_member_names(self.config.wxid_str()).await {
                        Ok(member_map) => {
                            let mut at_names = Vec::new();
                            let mut valid_wxids = Vec::new();

                            for wxid in at_list {
                                if let Some(name) = member_map.get(wxid) {
                                    at_names.push(format!("@{}", name));
                                    valid_wxids.push(wxid.clone());
                                } else {
                                    warn!("警告: 在群成员列表中未找到 wxid: {}", wxid);
                                }
                            }

                            if !at_names.is_empty() {
                                let mention_string = format!("{} ", at_names.join(" "));
                                final_content = format!("{}{}", mention_string, content);
                                ats_payload = Some(valid_wxids.join(","));
                                info!("最终 @ 内容: {}", mention_string.trim_end());
                                info!("最终 ats 参数: {:?}", ats_payload);
                            }
                        }
                        Err(err) => {
                            error!("获取群成员列表失败，无法执行 @ 操作: {}", err);
                            return Err(err);
                        }
                    }
                }
            }
        }

        let url = format!("{}/gewe/v2/api/message/postText", self.config.base_url);
        let request = PostTextRequest {
            app_id: self.config.app_id_str().to_string(),
            to_wxid: self.config.wxid_str().to_string(),
            content: final_content,
            ats: ats_payload,
        };

        let mut call = self.execute_post_text(&url, &request).await?;
        let mut ret_status = call.response.ret_status();
        let mut failure_code = call.response.failure_code();

        let should_retry_at_all = self.config.is_chatroom()
            && is_at_all
            && matches!(
                (ret_status, failure_code.as_ref()),
                (ApiRet::Failure(500), Some(ApiErrorCode::PermissionDenied))
            );

        if should_retry_at_all {
            warn!("警告: @ 全体成员失败，无权限，将尝试不 @ 全体成员重试。");

            let mut retry_request = request.clone();
            retry_request.content = content.to_string();
            retry_request.ats = None;

            call = self.execute_post_text(&url, &retry_request).await?;
            ret_status = call.response.ret_status();
            failure_code = call.response.failure_code();
        }

        if call.status.is_success() && call.response.is_success() {
            info!("通知发送成功");
            return Ok(());
        }

        let error = match failure_code {
            Some(code) => ApiBusinessError::KnownError { code },
            None if !call.response.msg.is_empty() => ApiBusinessError::UnknownError {
                code: ret_status.code(),
                message: call.response.msg.clone(),
            },
            None => {
                return Err(NetworkError::HttpError {
                    status: call.status.as_u16(),
                    body: Some(call.body.clone()),
                }
                .into())
            }
        };

        error!("通知发送失败: {:?} 原始响应: {}", error, call.body);
        Err(error.into())
    }
}

impl GeweApiClient {
    async fn execute_post_text(
        &self,
        url: &str,
        request: &PostTextRequest,
    ) -> Result<PostTextCall> {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|_| NetworkError::ConnectionRefused)?;

        let response = timeout(
            self.request_timeout,
            self.client
                .post(url)
                .header("X-GEWE-TOKEN", self.config.token_str())
                .header(header::CONTENT_TYPE, "application/json")
                .json(request)
                .send(),
        )
        .await
        .map_err(|_| NetworkError::Timeout {
            duration: self.request_timeout,
        })?
        .map_err(NetworkError::from)?;

        let status = response.status();
        let body = response.text().await.map_err(NetworkError::from)?;
        let parsed = serde_json::from_str::<PostTextResponse>(&body)?;

        Ok(PostTextCall {
            status,
            body,
            response: parsed,
        })
    }
}
