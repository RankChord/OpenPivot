use std::env;
use std::path::PathBuf;

mod api;
mod app;
mod init;
mod core;
mod error;
mod models;
mod repository;

use app::{AppState, build_router};
use init::{config, db};


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
    if !config::has_file(&cfg_dir, "openpivot.conf") {
        println!("[初始化检查]: 缺少配置文件，正在创建默认配置文件");
        let _ = config::create_file(&cfg_dir, "openpivot.conf");
        return;
    }

    let cfg = config::load_config(&cfg_dir, "openpivot.conf");

    if !cfg.app.enable {
        println!("[初始化检查]: 如果你已经完成对配置文件的初始化, 请将配置文件的app->enable置为True");
        return;
    }

    // 配置数据库
    let pool = db::create_pool(&cfg.database).await;

    // 初始化数据库
    db::run_migrations(&pool).await;

    let state = AppState {
        db: pool,
        config: cfg.clone(),
    };

    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind((
        cfg.server.host.as_str(),
        cfg.server.port,
    ))
    .await
    .unwrap();

    
    print!("正在监听{}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
