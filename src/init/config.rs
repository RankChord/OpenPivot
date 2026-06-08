use std::fs::{self, OpenOptions};
use std::io::{self, ErrorKind, Write};
use std::path::Path;

use serde::Deserialize;

use super::default::DEFAULT_CONFIG;


// 配置结构体区域
#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    pub server: ServerConfig,
    pub app: AppConfig,
    pub database: DatabaseConfig,
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub enable: bool,
    pub debug: bool,
    pub domain: String,
    pub federal: bool,
}

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: String,
    pub max_connections: u32,
}

pub fn has_file(dir: &str, file_name: &str) -> bool {
    let path = Path::new(dir).join(file_name);
    path.exists()
}

pub fn is_enable(dir: &str, file_name: &str) -> bool {
    let path = Path::new(dir).join(file_name);

    let content = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(_) => DEFAULT_CONFIG.to_string(),
    };

    match toml::from_str::<ConfigFile>(&content) {
        Ok(cfg) => cfg.app.enable,
        Err(_) => false, // 解析失败时默认 false
    }
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

pub fn read_config(dir: &str, file_name: &str) -> String {
    let path = Path::new(dir).join(file_name);
    fs::read_to_string(&path).unwrap_or_else(|_| DEFAULT_CONFIG.to_string())
}

pub fn get_database_config(dir: &str, file_name: &str) -> DatabaseConfig {
    let content = read_config(dir, file_name);
    toml::from_str::<DatabaseConfig>(&content).expect("无法解析数据库配置")
}