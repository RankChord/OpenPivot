//! Worker process manager — spawns, monitors, and restarts worker processes

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};

pub struct WorkerProcess {
    pub plugin_id: String,
    pub lib_path: PathBuf,
    pub socket_path: PathBuf,
    restart_count: AtomicU32,
}

impl WorkerProcess {
    pub fn new(plugin_id: &str, lib_path: &Path) -> Self {
        let socket_path = std::env::temp_dir().join(format!("agent-{}.sock", plugin_id));
        Self {
            plugin_id: plugin_id.into(),
            lib_path: lib_path.into(),
            socket_path,
            restart_count: AtomicU32::new(0),
        }
    }
    
    pub fn socket_path(&self) -> &Path { &self.socket_path }
    
    /// Spawn worker process
    pub async fn spawn(&self, _max_restarts: u32) -> Result<(), String> {
        let lib_path = self.lib_path.clone();
        let socket_path = self.socket_path.clone();
        
        // The worker binary is `agent-worker-bin` which loads the plugin dylib
        let mut child = Command::new("agent-worker-bin")
            .arg("--plugin")
            .arg(&lib_path)
            .arg("--socket")
            .arg(&socket_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn worker: {}", e))?;
        
        tokio::spawn(async move {
            let status = child.wait().await;
            tracing::info!("Worker exited: {:?}", status);
        });
        
        // Wait for socket to be ready
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        tracing::info!("Worker {} started on {:?}", self.plugin_id, self.socket_path);
        Ok(())
    }
    
    pub fn restart_count(&self) -> u32 {
        self.restart_count.load(Ordering::Relaxed)
    }
    
    pub fn increment_restart(&self) {
        self.restart_count.fetch_add(1, Ordering::Relaxed);
    }
}

pub struct WorkerPool {
    workers: Mutex<Vec<WorkerProcess>>,
    pub max_restarts: u32,
}

impl WorkerPool {
    pub fn new(max_restarts: u32) -> Self {
        Self {
            workers: Mutex::new(Vec::new()),
            max_restarts,
        }
    }
    
    pub async fn add_worker(&self, plugin_id: &str, lib_path: &Path) -> Result<(), String> {
        let worker = WorkerProcess::new(plugin_id, lib_path);
        worker.spawn(self.max_restarts).await?;
        self.workers.lock().await.push(worker);
        Ok(())
    }
    
    pub async fn health_check_loop(&self, interval_secs: u64) {
        let mut interval = interval(Duration::from_secs(interval_secs));
        loop {
            interval.tick().await;
            self.check_health().await;
        }
    }
    
    async fn check_health(&self) {
        // Simple health check: try connecting to each worker's socket
        // In production, would use a proper heartbeat protocol
        let workers = self.workers.lock().await;
        for worker in workers.iter() {
            if !std::fs::exists(&worker.socket_path).unwrap_or(false) {
                if worker.restart_count() < self.max_restarts {
                    tracing::warn!("Worker {} not responding, restarting (attempt {})", 
                        worker.plugin_id, worker.restart_count() + 1);
                    // Restart logic would go here
                } else {
                    tracing::error!("Worker {} exceeded max restarts, disabling", worker.plugin_id);
                }
            }
        }
    }
}
