use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
    routing::get,
    Router,
};

use crate::app::AppState;

// WebSocket 路由
pub fn routes() -> Router<AppState> {
    Router::new().route("/ws", get(handler))
}

// 升级 HTTP 连接为 WebSocket
async fn handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_socket)
}

// 处理单个连接的消息循环
async fn handle_socket(mut socket: WebSocket) {
    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            // 收到文本，原样回显
            Message::Text(text) => {
                if socket.send(Message::Text(text)).await.is_err() {
                    break; // 客户端已断开
                }
            }
            // 客户端请求关闭连接
            Message::Close(_) => break,
            // Ping/Pong/Binary 这里先忽略
            _ => {}
        }
    }
}