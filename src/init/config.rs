use std::fs::{self, OpenOptions};
use std::io::{self, ErrorKind, Write};
use std::path::Path;

use serde::Deserialize;

use super::default::DEFAULT_CONFIG;

#[derive(Debug, Deserialize)]
struct ConfigFile {
    app: AppConfig,
}

#[derive(Debug, Deserialize)]
struct AppConfig {
    enable: bool,
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

