//! Gateway server — listens on Unix socket and dispatches to Core

use crate::protocol::*;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::Mutex;

type RequestHandler = Box<dyn Fn(serde_json::Value) -> BoxFuture<'static, Result<serde_json::Value, String>> + Send + Sync>;

pub type BoxFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

pub struct GatewayServer {
    socket_path: std::path::PathBuf,
    handlers: Arc<Mutex<HashMap<String, RequestHandler>>>,
}

impl GatewayServer {
    pub fn new(socket_path: &Path) -> Self {
        // Remove stale socket
        let _ = std::fs::remove_file(socket_path);
        
        Self {
            socket_path: socket_path.into(),
            handlers: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    pub fn register_handler<F, Fut>(&mut self, method: &str, handler: F)
    where
        F: Fn(serde_json::Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<serde_json::Value, String>> + Send + 'static,
    {
        let handler: RequestHandler = Box::new(move |params| {
            let fut = handler(params);
            Box::pin(fut)
        });
        self.handlers.try_lock().unwrap().insert(method.into(), handler);
    }
    
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let listener = UnixListener::bind(&self.socket_path)?;
        let handlers = self.handlers.clone();
        
        tracing::info!("Gateway listening on {:?}", self.socket_path);
        
        let mut conn_id = 0u64;
        loop {
            let (stream, _) = listener.accept().await?;
            let handlers = handlers.clone();
            let id = conn_id;
            conn_id += 1;
            
            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, handlers, id).await {
                    tracing::error!("Connection {} error: {}", id, e);
                }
            });
        }
    }
}

async fn handle_connection(
    stream: UnixStream,
    handlers: Arc<Mutex<HashMap<String, RequestHandler>>>,
    conn_id: u64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    
    loop {
        line.clear();
        let bytes = reader.read_line(&mut line).await?;
        if bytes == 0 {
            tracing::debug!("Connection {} closed", conn_id);
            break;
        }
        
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        
        match serde_json::from_str::<JsonRpcRequest>(trimmed) {
            Ok(req) => {
                let handlers = handlers.lock().await;
                let response = if let Some(handler) = handlers.get(&req.method) {
                    match handler(req.params).await {
                        Ok(result) => JsonRpcResponse::result(req.id, result),
                        Err(e) => JsonRpcResponse::error(req.id, error_codes::INTERNAL_ERROR, &e),
                    }
                } else {
                    JsonRpcResponse::error(req.id, error_codes::METHOD_NOT_FOUND, &format!("Unknown method: {}", req.method))
                };
                
                let json = serde_json::to_string(&response)?;
                writer.write_all(json.as_bytes()).await?;
                writer.write_all(b"\n").await?;
                writer.flush().await?;
            }
            Err(e) => {
                let error = JsonRpcResponse::error(0, error_codes::PARSE_ERROR, &format!("Invalid JSON: {}", e));
                let json = serde_json::to_string(&error)?;
                writer.write_all(json.as_bytes()).await?;
                writer.write_all(b"\n").await?;
            }
        }
    }
    
    Ok(())
}
