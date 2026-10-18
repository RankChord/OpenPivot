//! 服务配置：把运行参数与业务逻辑分开，便于主项目替换为自己的配置系统。
use std::{env, net::SocketAddr, path::PathBuf};

pub const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;
pub const DEMO_ADMIN_TOKEN: &str = "demo-admin-token";

#[derive(Clone)]
pub struct Config {
    pub bind: SocketAddr,
    pub database: PathBuf,
    pub admin_token: String,
    pub allowed_hosts: Option<Vec<String>>,
    pub allowed_origins: Option<Vec<String>>,
    /// 可选：为已有 Agent 导入外部管理的凭证，不负责创建或限制 Agent 数量。
    pub agent_token_import: Option<PathBuf>,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let config = Self {
            bind: env::var("AGENT_SPACE_BIND")
                .unwrap_or_else(|_| "127.0.0.1:8080".into())
                .parse()
                .map_err(|_| "AGENT_SPACE_BIND must be an IP socket address")?,
            database: env::var("AGENT_SPACE_DB")
                .unwrap_or_else(|_| "data/workspace.sqlite3".into())
                .into(),
            admin_token: env::var("AGENT_SPACE_ADMIN_TOKEN")
                .unwrap_or_else(|_| DEMO_ADMIN_TOKEN.into()),
            allowed_hosts: comma_list("AGENT_SPACE_ALLOWED_HOSTS")?,
            allowed_origins: comma_list("AGENT_SPACE_ALLOWED_ORIGINS")?,
            agent_token_import: env::var("AGENT_SPACE_TOKENS").ok().map(PathBuf::from),
        };
        config.validate()?;
        Ok(config)
    }

    /// 默认弱凭证只能用于本地 demo；对外监听必须显式配置强管理凭证和 Host。
    pub fn validate(&self) -> Result<(), String> {
        if self.admin_token.is_empty()
            || !self.admin_token.bytes().all(|byte| byte.is_ascii_graphic())
        {
            return Err(
                "AGENT_SPACE_ADMIN_TOKEN must contain visible ASCII without whitespace".into(),
            );
        }
        if !self.bind.ip().is_loopback()
            && (self.admin_token.len() < 32
                || self.admin_token == DEMO_ADMIN_TOKEN
                || self.allowed_hosts.is_none())
        {
            return Err("non-loopback bind requires a custom admin token of at least 32 characters and explicit AGENT_SPACE_ALLOWED_HOSTS".into());
        }
        Ok(())
    }
}

fn comma_list(name: &str) -> Result<Option<Vec<String>>, String> {
    let Ok(value) = env::var(name) else {
        return Ok(None);
    };
    let values: Vec<_> = value
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();
    if values.is_empty() {
        return Err(format!("{name} must not be empty"));
    }
    Ok(Some(values))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn public_bind_requires_strong_admin_and_host_configuration() {
        let mut config = Config {
            bind: "0.0.0.0:8080".parse().unwrap(),
            database: "unused.db".into(),
            admin_token: DEMO_ADMIN_TOKEN.into(),
            allowed_hosts: None,
            allowed_origins: None,
            agent_token_import: None,
        };
        assert!(config.validate().is_err());
        config.admin_token = "a".repeat(40);
        assert!(config.validate().is_err());
        config.allowed_hosts = Some(vec!["workspace.example.test".into()]);
        assert!(config.validate().is_ok());
    }
}
