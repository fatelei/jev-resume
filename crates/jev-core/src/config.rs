//! 应用配置：config_dir/yueli/config.toml（unix 0600）。

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};

pub const CONFIG_DIR_NAME: &str = "yueli";
pub const CONFIG_FILE_NAME: &str = "config.toml";
pub const CRITERIA_FILE_NAME: &str = "criteria.toml";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppConfig {
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_base_url")]
    pub base_url: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_concurrency")]
    pub concurrency: u32,
}

fn default_base_url() -> String {
    "https://api.typesafe.ai".into()
}

fn default_model() -> String {
    "jev-latest".into()
}

fn default_concurrency() -> u32 {
    4
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: default_base_url(),
            model: default_model(),
            concurrency: default_concurrency(),
        }
    }
}

pub fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join(CONFIG_DIR_NAME))
}

pub fn config_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join(CONFIG_FILE_NAME))
}

pub fn criteria_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join(CRITERIA_FILE_NAME))
}

#[cfg(unix)]
fn write_private(path: &PathBuf, content: &str) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    f.write_all(content.as_bytes())
}

#[cfg(not(unix))]
fn write_private(path: &PathBuf, content: &str) -> std::io::Result<()> {
    std::fs::write(path, content)
}

pub fn load() -> CoreResult<AppConfig> {
    let Some(path) = config_path() else {
        return Ok(AppConfig::default());
    };
    match std::fs::read_to_string(&path) {
        Ok(raw) => {
            let cfg: AppConfig = toml::from_str(&raw)
                .map_err(|e| CoreError::NotConfigured(format!("配置文件损坏: {e}")))?;
            Ok(cfg)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(AppConfig::default()),
        Err(e) => Err(CoreError::Io(e)),
    }
}

/// 首次安装时把内置默认判据写到用户配置目录（已存在则不动）。
pub fn ensure_criteria_file(default_toml: &str) -> CoreResult<Option<PathBuf>> {
    let Some(dir) = config_dir() else {
        return Ok(None);
    };
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(CRITERIA_FILE_NAME);
    if !path.exists() {
        std::fs::write(&path, default_toml)?;
        return Ok(Some(path));
    }
    Ok(Some(path))
}

pub fn save(config: &AppConfig) -> CoreResult<()> {
    let Some(path) = config_path() else {
        return Err(CoreError::NotConfigured("无法定位用户配置目录".into()));
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let raw = toml::to_string_pretty(config)
        .map_err(|e| CoreError::NotConfigured(format!("配置序列化: {e}")))?;
    let tmp = path.with_extension("toml.tmp");
    write_private(&tmp, &raw)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_roundtrip() {
        let c = AppConfig::default();
        let raw = toml::to_string_pretty(&c).unwrap();
        let c2: AppConfig = toml::from_str(&raw).unwrap();
        assert_eq!(c, c2);
        assert_eq!(c.concurrency, 4);
        assert_eq!(c.base_url, "https://api.typesafe.ai");
    }

    #[test]
    fn partial_toml_uses_defaults() {
        let c2: AppConfig = toml::from_str("api_key = \"k\"\nconcurrency = 2\n").unwrap();
        assert_eq!(c2.model, "jev-latest");
        assert_eq!(c2.concurrency, 2);
    }
}
