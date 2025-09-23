use reqwest::{header, Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{error, info, warn};

use crate::config::Config;
use crate::errors::{GeweNoticeError, Result};

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

#[derive(Debug, Clone, PartialEq, Eq)]
enum FailureCode {
    NotInGroup,
    ChatroomMissing,
    PermissionDenied,
    Unknown(String),
}

impl From<&str> for FailureCode {
    fn from(value: &str) -> Self {
        match value {
            "-219" => Self::NotInGroup,
            "-104" => Self::ChatroomMissing,
            "-2" => Self::PermissionDenied,
            other => Self::Unknown(other.to_string()),
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

    fn failure_code(&self) -> Option<FailureCode> {
        self.data
            .as_ref()
            .and_then(|data| data.code.as_deref())
            .map(FailureCode::from)
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
}

impl GeweApiClient {
    pub fn new(config: Config) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(GeweNoticeError::from)?;

        Ok(Self { client, config })
    }

    pub async fn check_online(&self) -> Result<bool> {
        info!("正在检查微信机器人在线状态...");

        let url = format!("{}/gewe/v2/api/login/checkOnline", self.config.base_url);
        let request = CheckOnlineRequest {
            app_id: self.config.app_id.clone(),
        };

        let response = self
            .client
            .post(&url)
            .header("X-GEWE-TOKEN", &self.config.token)
            .json(&request)
            .send()
            .await?;

        if response.status().is_success() {
            let data: CheckOnlineResponse = response.json().await?;

            if data.ret != 200 {
                error!(
                    "在线状态检查返回异常 (ret: {}, msg: {})",
                    data.ret, data.msg
                );
                return Err(GeweNoticeError::ApiError {
                    code: data.ret,
                    message: data.msg,
                });
            }

            if let Some(true) = data.data {
                info!("机器人在线，准备就绪。");
                Ok(true)
            } else {
                error!("机器人当前不在线。");
                error!("   - App ID: {}", self.config.app_id);
                Err(GeweNoticeError::BotOffline)
            }
        } else {
            let status = response.status();
            let text = response.text().await?;
            error!("在线状态检查失败，HTTP 状态码: {}", status);
            error!("   - 响应内容: {}", text);
            Err(GeweNoticeError::ApiError {
                code: status.as_u16() as i32,
                message: text,
            })
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
            app_id: self.config.app_id.clone(),
            chatroom_id: chatroom_id.to_string(),
        };

        let response = self
            .client
            .post(&url)
            .header("X-GEWE-TOKEN", &self.config.token)
            .header(header::CONTENT_TYPE, "application/json")
            .json(&request)
            .send()
            .await?;

        if response.status().is_success() {
            let data: GetChatroomMemberListResponse = response.json().await?;

            if data.ret != 200 {
                if data.ret == 500 && data.msg == "获取群成员列表异常:null" {
                    error!(
                        "获取群成员列表失败: 你可能已不在群 {} 内或该群聊不存在。",
                        chatroom_id
                    );
                } else {
                    error!(
                        "获取群成员列表失败 (ret: {}, msg: {})，chatroom_id: {}",
                        data.ret, data.msg, chatroom_id
                    );
                }
                return Err(GeweNoticeError::ApiError {
                    code: data.ret,
                    message: data.msg,
                });
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
            let text = response.text().await?;
            error!("获取群成员列表失败，状态码: {}, 响应: {}", status, text);
            Err(GeweNoticeError::ApiError {
                code: status.as_u16() as i32,
                message: text,
            })
        }
    }

    pub async fn post_text(&self, content: &str) -> Result<()> {
        info!("准备发送通知: '{}'", content);

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
                    match self.get_chatroom_member_names(&self.config.wxid).await {
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
            app_id: self.config.app_id.clone(),
            to_wxid: self.config.wxid.clone(),
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
                (ApiRet::Failure(500), Some(FailureCode::PermissionDenied))
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

        let error_message = match failure_code.as_ref() {
            Some(FailureCode::NotInGroup) => "你已不在该群内".to_string(),
            Some(FailureCode::ChatroomMissing) => "该群聊不存在".to_string(),
            Some(FailureCode::PermissionDenied) if !self.config.is_chatroom() => {
                "对方不是你的好友或该微信用户不存在".to_string()
            }
            Some(FailureCode::PermissionDenied) => {
                "操作无权限（如@非好友）或遇到未知群聊错误".to_string()
            }
            Some(FailureCode::Unknown(raw)) => {
                format!("接口返回未知错误码 {}: {}", raw, call.response.msg)
            }
            None => call.response.msg.clone(),
        };

        error!("通知发送失败: {} 原始响应: {}", error_message, call.body);
        let api_code = if matches!(ret_status, ApiRet::Success) {
            call.status.as_u16() as i32
        } else {
            ret_status.code()
        };

        Err(GeweNoticeError::ApiError {
            code: api_code,
            message: error_message,
        })
    }
}

impl GeweApiClient {
    async fn execute_post_text(
        &self,
        url: &str,
        request: &PostTextRequest,
    ) -> Result<PostTextCall> {
        let response = self
            .client
            .post(url)
            .header("X-GEWE-TOKEN", &self.config.token)
            .header(header::CONTENT_TYPE, "application/json")
            .json(request)
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;
        let parsed = serde_json::from_str::<PostTextResponse>(&body)?;

        Ok(PostTextCall {
            status,
            body,
            response: parsed,
        })
    }
}
