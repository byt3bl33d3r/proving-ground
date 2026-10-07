//! This worktree's harness state: `.harness/app.json` (written by the server) and
//! `.harness/stack.json` (written by `just up`).

use std::path::{Path, PathBuf};

pub use demo_app_runtime::config::AppInfo;
use demo_app_runtime::config::env_value;
use serde_json::Value;

/// The harness directory: `$APP_HARNESS_DIR`, else the nearest `.harness` directory in the
/// current directory or its ancestors.
pub fn dir() -> Option<PathBuf> {
    if let Ok(Some(dir)) = env_value("APP_HARNESS_DIR") {
        return Some(PathBuf::from(dir));
    }
    let cwd = std::env::current_dir().ok()?;
    cwd.ancestors()
        .map(|dir| dir.join(".harness"))
        .find(|dir| dir.is_dir())
}

/// Reads `app.json` from the harness directory.
pub fn app(dir: &Path) -> Option<AppInfo> {
    AppInfo::read(dir)
}

/// Reads `stack.json` from the harness directory, as raw JSON.
pub fn stack(dir: &Path) -> Option<Value> {
    serde_json::from_str(&std::fs::read_to_string(dir.join("stack.json")).ok()?).ok()
}
