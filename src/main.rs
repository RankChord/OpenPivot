use std::env;
use std::path::PathBuf;


mod app;
mod api;
mod init;
mod core;
mod error;
mod models;
mod repository;

use app::{AppState, build_router};
use init::{config, db};


#[tokio::main]
async fn main() {
    // --------------------- 初始化检查 --------------------
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

    // 加载配置文件
    let cfg = config::load_config(&cfg_dir, "openpivot.conf");

    // 检查 app.enable 是否确认
    if !cfg.app.enable {
        println!("[初始化检查]: 如果你已经完成对配置文件的初始化, 请将配置文件的app->enable置为True");
        return;
    }

    // --------------------- 数据库配置 --------------------
    // 配置数据库
    let pool = db::create_pool(&cfg.database).await;

    // 初始化数据库
    db::run_migrations(&pool).await;


    // --------------------- 启动服务 --------------------
    // 创建 app state
    let state = AppState {
        db: pool,
        config: cfg.clone(),
    };

    // 构建路由
    let app = build_router(state);

    // 启动服务
    let listener = tokio::net::TcpListener::bind((
        cfg.server.host.as_str(),
        cfg.server.port,
    ))
    .await
    .unwrap();

    
    print!("正在监听{}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
