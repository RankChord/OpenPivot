use std::env;
use std::path::PathBuf;
use axum::{Router, routing::get};

mod api;
mod init;

use api::v1::system::health;
use init::{config, db};

use serde::Deserialize;

fn init_check(dir: &str, file_name: &str) -> bool {
    // 配置文件检查并初始化
    if !config::has_file(dir, file_name) {
        println!("[初始化检查]: 缺少配置文件，正在创建默认配置文件");
        config::create_file(dir, file_name);
        return true;
    }

    // 检查用户是否配置完成
    if !config::is_enable(dir, file_name) {
        println!("[初始化检查]: 如果你已经完成对配置文件的初始化, 请将配置文件的app->enable置为True");
        return false;
    }

    true
}

#[tokio::main]
async fn main() {
    // 配置文件路径
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let cfg_dir = PathBuf::from(home)
        .join(".config")
        .join("openpivot")
        .to_string_lossy()
        .into_owned();

    // 初始化检查
    if !init_check(&cfg_dir, "openpivot.conf") { return; }

    // 配置数据库
    let db_cfg = config::get_database_config(&cfg_dir, "openpivot.conf");
    let pool = db::create_pool(db_cfg.url).await;

    // 初始化数据库
    let _ = db::init_db(&pool);

    let app = Router::new().route("/health", get(health));
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 3000))
        .await
        .unwrap();

    
    print!("正在监听{}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();

   
}
