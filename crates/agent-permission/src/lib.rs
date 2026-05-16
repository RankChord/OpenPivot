use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct PathPolicy {
    pub always_allow: Vec<PathBuf>,
    pub always_deny: Vec<PathBuf>,
    pub allow_read: Vec<PathBuf>,
    pub allow_write: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileMode {
    Read,
    Write,
    Execute,
    Delete,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PermissionDecision {
    Allow,
    Deny,
    Ask,
}

#[derive(Debug, Error)]
pub enum PermissionError {
    #[error("path not allowed: {0}")]
    PathDenied(PathBuf),
}

impl PathPolicy {
    pub fn new(workspace: &Path) -> Self {
        Self {
            always_allow: vec![],
            always_deny: vec![
                PathBuf::from("/etc"),
                PathBuf::from("/usr"),
                #[cfg(unix)]
                dirs::home_dir().unwrap_or_default(),
            ],
            allow_read: vec![workspace.to_path_buf()],
            allow_write: vec![workspace.to_path_buf()],
        }
    }

    pub fn check(&self, path: &Path, operation: FileMode) -> PermissionDecision {
        // 1. Check explicit denies first
        if self.is_denied(path) {
            return PermissionDecision::Deny;
        }

        // 2. Check explicit allows
        if self.is_always_allowed(path) {
            return PermissionDecision::Allow;
        }

        // 3. Check operation-specific permissions
        match operation {
            FileMode::Read => {
                if self.is_in_list(path, &self.allow_read) {
                    PermissionDecision::Allow
                } else {
                    PermissionDecision::Ask
                }
            }
            FileMode::Write | FileMode::Delete => {
                if self.is_in_list(path, &self.allow_write) {
                    PermissionDecision::Allow
                } else {
                    PermissionDecision::Ask
                }
            }
            FileMode::Execute => PermissionDecision::Ask,
        }
    }

    fn is_denied(&self, path: &Path) -> bool {
        self.always_deny.iter().any(|d| path.starts_with(d))
    }

    fn is_always_allowed(&self, path: &Path) -> bool {
        self.always_allow.iter().any(|a| path.starts_with(a))
    }

    fn is_in_list(&self, path: &Path, list: &[PathBuf]) -> bool {
        list.iter().any(|dir| path.starts_with(dir))
    }
}
