//! Gateway client — Core side of IPC, sends requests to Worker

use crate::protocol::*;
use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

pub struct GatewayClient {
    socket_path: std::path::PathBuf,
}

impl GatewayClient {
    pub fn new(socket_path: &Path) -> Self {
        Self {
            socket_path: socket_path.into(),
        }
    }
    
    pub async fn call(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value, String> {
        let stream = UnixStream::connect(&self.socket_path).await
            .map_err(|e| format!("Failed to connect: {}", e))?;
        
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        
        let req = JsonRpcRequest::new(1, method, &params);
        let json = serde_json::to_string(&req).map_err(|e| format!("Serialize error: {}", e))?;
        
        writer.write_all(json.as_bytes()).await
            .map_err(|e| format!("Write error: {}", e))?;
        writer.write_all(b"\n").await
            .map_err(|e| format!("Write error: {}", e))?;
        writer.flush().await
            .map_err(|e| format!("Flush error: {}", e))?;
        
        let mut response_line = String::new();
        reader.read_line(&mut response_line).await
            .map_err(|e| format!("Read error: {}", e))?;
        
        let response: JsonRpcResponse = serde_json::from_str(&response_line)
            .map_err(|e| format!("Parse error: {}", e))?;
        
        if let Some(error) = response.error {
            return Err(format!("RPC error {}: {}", error.code, error.message));
        }
        
        Ok(response.result.unwrap_or(serde_json::json!(null)))
    }
    
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }
}
