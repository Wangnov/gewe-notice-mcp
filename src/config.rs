use crate::errors::{GeweNoticeError, Result};
use clap::Parser;
use uuid::Uuid;

#[derive(Debug, Clone, Parser)]
#[clap(
    name = "gewe-notice-mcp",
    about = "一个通过微信机器人发送AI任务状态通知的轻量级工具",
    version
)]
pub struct Config {
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

impl Config {
    pub fn validate(&self) -> Result<()> {
        Uuid::parse_str(&self.token).map_err(|_| {
            GeweNoticeError::UuidError(
                "GEWE_NOTICE_TOKEN 格式无效，应该是一个有效的 UUID".to_string(),
            )
        })?;

        if !self.app_id.starts_with("wx_") {
            return Err(GeweNoticeError::ConfigError(
                "GEWE_NOTICE_APP_ID 格式无效，应该以 'wx_' 开头".to_string(),
            ));
        }

        if self.wxid.is_empty() {
            return Err(GeweNoticeError::ConfigError(
                "GEWE_NOTICE_WXID 不能为空".to_string(),
            ));
        }

        if self.wxid.contains("@chatroom") && !self.wxid.ends_with("@chatroom") {
            return Err(GeweNoticeError::ConfigError(
                "GEWE_NOTICE_WXID 格式无效，群聊ID应该以 '@chatroom' 结尾".to_string(),
            ));
        }

        Ok(())
    }

    pub fn is_chatroom(&self) -> bool {
        self.wxid.contains("@chatroom")
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
        let raw = self.at_list.as_ref()?;
        let cleaned: Vec<String> = raw
            .iter()
            .map(|item| item.trim())
            .filter(|item| !item.is_empty())
            .map(|item| {
                if item.eq_ignore_ascii_case("all") {
                    "all".to_string()
                } else {
                    item.to_string()
                }
            })
            .collect();

        if cleaned.is_empty() {
            return None;
        }

        Some(cleaned)
    }
}

#[cfg(test)]
mod tests {
    use super::Config;

    fn base_config() -> Config {
        Config {
            base_url: "https://example.com".into(),
            token: "00000000-0000-0000-0000-000000000000".into(),
            app_id: "wx_app".into(),
            wxid: "wxid_sample".into(),
            at_list: None,
        }
    }

    #[test]
    fn normalized_handles_none() {
        let config = base_config();
        assert!(config.normalized_at_list().is_none());
    }

    #[test]
    fn normalized_trims_and_filters_empty_entries() {
        let mut config = base_config();
        config.at_list = Some(vec![" wxid_1 ".into(), " ".into(), "wxid_2".into()]);

        let result = config.normalized_at_list().expect("normalized list");
        assert_eq!(result, vec!["wxid_1".to_string(), "wxid_2".to_string()]);
    }

    #[test]
    fn normalized_handles_all_case_insensitively() {
        let mut config = base_config();
        config.at_list = Some(vec![" ALL ".into()]);

        let result = config.normalized_at_list().expect("all entry");
        assert_eq!(result, vec!["all".to_string()]);
    }
}
