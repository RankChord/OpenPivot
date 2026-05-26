use notify::RecursiveMode;
use notify::Watcher;
use std::path::Path;
use std::sync::mpsc::channel;
use tokio::spawn;

/// PluginWatcher listens for dylib changes in a directory
pub fn watch_plugins(dir: &Path, callback: impl Fn(String) + Send + Sync + 'static) {
    let (tx, rx) = channel();
    let mut watcher = notify::recommended_watcher(tx).unwrap();

    watcher.watch(dir, RecursiveMode::Recursive).unwrap();

    spawn(async move {
        loop {
            if let Ok(event) = rx.recv() {
                match event {
                    Ok(event) => {
                        if event.kind.is_modify() || event.kind.is_create() {
                            if let Some(path) = event.paths.first() {
                                if let Some(filename) = path.file_stem() {
                                    let name = filename.to_string_lossy().to_string();
                                    if name.starts_with("lib") && (name.ends_with(".so") || name.ends_with(".dylib")) {
                                        let tool_name = name.strip_prefix("lib").unwrap_or(&name);
                                        // In real impl, invoke callback
                                        println!("Reloading plugin: {}", tool_name);
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => println!("Watch error: {}", e),
                }
            }
        }
    });
}
