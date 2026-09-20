//! 判定结果缓存：data_dir/jev-resume/results/{criteria_version}/{content_hash}.json
//! 目录名带 criteria_version —— 改判据即整目录换位，旧缓存自然失效。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};
use crate::jev::JevResult;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CacheFileInfo {
    pub name: String,
    pub size: u64,
    pub format: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CacheRecord {
    pub schema: u32,
    pub criteria_version: u64,
    pub questions_version: u64,
    pub model: String,
    pub content_hash: String,
    pub file: CacheFileInfo,
    pub state_text: String,
    pub result: JevResult,
    pub elapsed_ms: u64,
    pub created_at: u64,
}

pub const CACHE_SCHEMA: u32 = 1;
pub const DATA_DIR_ENV: &str = "JEV_RESUME_DATA_DIR";

/// 缓存根目录；`JEV_RESUME_DATA_DIR` 可重定向（便携部署 / 测试隔离）。
pub fn cache_root() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var(DATA_DIR_ENV) {
        if !dir.trim().is_empty() {
            return Some(PathBuf::from(dir).join("results"));
        }
    }
    dirs::data_dir().map(|d| d.join("jev-resume").join("results"))
}

pub fn cache_dir_for(root: &Path, criteria_version: u64) -> PathBuf {
    root.join(format!("v{criteria_version}"))
}

pub fn record_path(dir: &Path, content_hash: &str) -> PathBuf {
    dir.join(format!("{content_hash}.json"))
}

pub fn get(dir: &Path, content_hash: &str) -> CoreResult<Option<CacheRecord>> {
    let path = record_path(dir, content_hash);
    match std::fs::read_to_string(&path) {
        Ok(raw) => {
            let record: CacheRecord = serde_json::from_str(&raw)
                .map_err(|e| CoreError::Cache(format!("缓存损坏 {}: {e}", path.display())))?;
            Ok(Some(record))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(CoreError::Cache(e.to_string())),
    }
}

pub fn put(dir: &Path, record: &CacheRecord) -> CoreResult<()> {
    std::fs::create_dir_all(dir)?;
    let path = record_path(dir, &record.content_hash);
    let raw = serde_json::to_string_pretty(record).map_err(|e| CoreError::Cache(e.to_string()))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, raw)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jev::JevAnswer;
    use std::collections::HashMap;

    fn record(hash: &str) -> CacheRecord {
        CacheRecord {
            schema: CACHE_SCHEMA,
            criteria_version: 3,
            questions_version: 1,
            model: "jev-latest".into(),
            content_hash: hash.into(),
            file: CacheFileInfo {
                name: "张三.pdf".into(),
                size: 1024,
                format: "PDF".into(),
            },
            state_text: "[正文]\n测试".into(),
            result: JevResult {
                model: Some("jev-1.13.0".into()),
                answers: HashMap::from([(
                    "strength".into(),
                    JevAnswer {
                        kind: "score".into(),
                        choice: None,
                        probabilities: HashMap::new(),
                        score: Some(6.5),
                        noul: None,
                        confidence: Some(0.8),
                    },
                )]),
                usage: HashMap::from([("input_tokens".into(), 1300)]),
            },
            elapsed_ms: 830,
            created_at: 1_758_000_000,
        }
    }

    #[test]
    fn roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let rec = record("abc123");
        put(dir.path(), &rec).unwrap();
        let got = get(dir.path(), "abc123").unwrap().unwrap();
        assert_eq!(got, rec);
    }

    #[test]
    fn miss_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        assert!(get(dir.path(), "nope").unwrap().is_none());
    }

    #[test]
    fn versioned_dir_isolation() {
        let dir = tempfile::tempdir().unwrap();
        let v3 = cache_dir_for(dir.path(), 3);
        let v4 = cache_dir_for(dir.path(), 4);
        assert_ne!(v3, v4);
        put(&v3, &record("abc")).unwrap();
        assert!(get(&v3, "abc").unwrap().is_some());
        assert!(get(&v4, "abc").unwrap().is_none());
    }
}
