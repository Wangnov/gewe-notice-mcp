use crate::errors::{ConfigValidationError, Result, TokenValidationError, WxIdValidationError};
use clap::Parser;
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ValidatedToken(Uuid);

impl ValidatedToken {
    pub fn new(s: &str) -> Result<Self> {
        Uuid::parse_str(s).map(ValidatedToken).map_err(|e| {
            ConfigValidationError::InvalidToken {
                reason: TokenValidationError::ParseError(e),
            }
            .into()
        })
    }

    pub fn as_str(&self) -> String {
        self.0.to_string()
    }
}

impl fmt::Display for ValidatedToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct AppId(String);

impl AppId {
    pub fn new(s: String) -> Result<Self> {
        if !s.starts_with("wx_") {
            return Err(ConfigValidationError::InvalidAppId { value: s }.into());
        }
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AppId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct WxId(String);

impl WxId {
    pub fn new(s: String) -> Result<Self> {
        if s.is_empty() {
            return Err(ConfigValidationError::InvalidWxId {
                reason: WxIdValidationError::Empty,
            }
            .into());
        }

        // "all" 是特殊的@全员标识符
        if s.eq_ignore_ascii_case("all") {
            return Ok(Self("all".to_string()));
        }

        if s.contains("@chatroom") && !s.ends_with("@chatroom") {
            return Err(ConfigValidationError::InvalidWxId {
                reason: WxIdValidationError::InvalidChatroomFormat,
            }
            .into());
        }

        Ok(Self(s))
    }

    pub fn is_chatroom(&self) -> bool {
        self.0.contains("@chatroom")
    }

    pub fn is_all(&self) -> bool {
        self.0 == "all"
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WxId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Parser)]
#[clap(
    name = "gewe-notice-mcp",
    about = "一个通过微信机器人发送AI任务状态通知的轻量级工具",
    version
)]
pub struct RawConfig {
    #[clap(
        long,
        env = "GEWE_NOTICE_BASE_URL",
        default_value = "https://www.geweapi.com",
        help = "Gewe API 基础URL"
    )]
    pub base_url: String,

    #[clap(long, env = "GEWE_NOTICE_TOKEN", help = "Gewe API Token (UUID格式)")]
    pub token: String,

    #[clap(long, env = "GEWE_NOTICE_APP_ID", help = "微信机器人App ID (wx_开头)")]
    pub app_id: String,

    #[clap(long, env = "GEWE_NOTICE_WXID", help = "接收者WXID (个人或群聊)")]
    pub wxid: String,

    #[clap(
        long,
        env = "GEWE_NOTICE_AT_LIST",
        value_delimiter = ',',
        help = "@列表 (逗号分隔的wxid或'all')"
    )]
    pub at_list: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub base_url: String,
    pub token: ValidatedToken,
    pub app_id: AppId,
    pub wxid: WxId,
    pub at_list: Option<Vec<WxId>>,
}

impl Config {
    pub fn parse() -> Result<Self> {
        let raw = RawConfig::parse();
        Self::from_raw(raw)
    }

    pub fn from_raw(raw: RawConfig) -> Result<Self> {
        let token = ValidatedToken::new(&raw.token)?;
        let app_id = AppId::new(raw.app_id)?;
        let wxid = WxId::new(raw.wxid)?;

        let at_list = match raw.at_list {
            Some(list) => {
                let mut validated = Vec::new();
                for id in list {
                    let trimmed = id.trim();
                    if !trimmed.is_empty() {
                        validated.push(WxId::new(trimmed.to_string())?);
                    }
                }
                if validated.is_empty() {
                    None
                } else {
                    Some(validated)
                }
            }
            None => None,
        };

        Ok(Self {
            base_url: raw.base_url,
            token,
            app_id,
            wxid,
            at_list,
        })
    }

    pub fn is_chatroom(&self) -> bool {
        self.wxid.is_chatroom()
    }

    pub fn redact(&self, value: &str, show_first: usize, show_last: usize) -> String {
        if value.len() <= show_first + show_last {
            "****".to_string()
        } else {
            format!(
                "{}****{}",
                &value[..show_first],
                &value[value.len() - show_last..]
            )
        }
    }

    pub fn normalized_at_list(&self) -> Option<Vec<String>> {
        self.at_list
            .as_ref()
            .map(|list| list.iter().map(|wxid| wxid.as_str().to_string()).collect())
    }

    pub fn token_str(&self) -> String {
        self.token.as_str()
    }

    pub fn app_id_str(&self) -> &str {
        self.app_id.as_str()
    }

    pub fn wxid_str(&self) -> &str {
        self.wxid.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_raw_config() -> RawConfig {
        RawConfig {
            base_url: "https://example.com".into(),
            token: "00000000-0000-0000-0000-000000000000".into(),
            app_id: "wx_app".into(),
            wxid: "wxid_sample".into(),
            at_list: None,
        }
    }

    #[test]
    fn normalized_handles_none() {
        let raw = base_raw_config();
        let config = Config::from_raw(raw).expect("valid config");
        assert!(config.normalized_at_list().is_none());
    }

    #[test]
    fn normalized_trims_and_filters_empty_entries() {
        let mut raw = base_raw_config();
        raw.at_list = Some(vec![" wxid_1 ".into(), " ".into(), "wxid_2".into()]);

        let config = Config::from_raw(raw).expect("valid config");
        let result = config.normalized_at_list().expect("normalized list");
        assert_eq!(result, vec!["wxid_1".to_string(), "wxid_2".to_string()]);
    }

    #[test]
    fn normalized_handles_all_case_insensitively() {
        let mut raw = base_raw_config();
        raw.at_list = Some(vec![" ALL ".into()]);

        let config = Config::from_raw(raw).expect("valid config");
        let result = config.normalized_at_list().expect("all entry");
        assert_eq!(result, vec!["all".to_string()]);
    }

    #[test]
    fn test_invalid_token() {
        let mut raw = base_raw_config();
        raw.token = "not-a-uuid".into();

        let result = Config::from_raw(raw);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_app_id() {
        let mut raw = base_raw_config();
        raw.app_id = "invalid_app_id".into();

        let result = Config::from_raw(raw);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_wxid() {
        let mut raw = base_raw_config();
        raw.wxid = "".into();

        let result = Config::from_raw(raw);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_chatroom_format() {
        let mut raw = base_raw_config();
        raw.wxid = "something@chatroommore".into();

        let result = Config::from_raw(raw);
        assert!(result.is_err());
    }
}
