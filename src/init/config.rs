use std::fs::{self, OpenOptions};
use std::io::{self, ErrorKind, Write};
use std::path::Path;

use serde::Deserialize;

use super::default::DEFAULT_CONFIG;


// 定义配置结构体
#[derive(Debug, Clone, Deserialize)]
pub struct ConfigFile {
    pub server: ServerConfig,
    pub app: AppConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub enable: bool,
    pub debug: bool,
    pub domain: String,
    pub federal: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub access_token_ttl_minutes: u64,
    pub refresh_token_ttl_days: u64,
}

// 配置数据库数据格式
impl DatabaseConfig {
    pub fn connection_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username,
            self.password,
            self.host,
            self.port,
            self.database
        )
    }
}

pub fn has_file(dir: &str, file_name: &str) -> bool {
    let path = Path::new(dir).join(file_name);
    path.exists()
}


pub fn create_file(dir: &str, file_name: &str) -> io::Result<()> {
    let path = Path::new(dir).join(file_name);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
    {
        Ok(mut file) => {
            file.write_all(DEFAULT_CONFIG.as_bytes())?;
            Ok(())
        }
        Err(err) if err.kind() == ErrorKind::AlreadyExists => Ok(()), 
        Err(err) => Err(err),
    }
}

pub fn load_config(dir: &str, file_name: &str) -> ConfigFile {
    let path = Path::new(dir).join(file_name);
    let content = fs::read_to_string(&path).unwrap_or_else(|_| DEFAULT_CONFIG.to_string());
    toml::from_str::<ConfigFile>(&content).expect("无法解析配置文件")
}