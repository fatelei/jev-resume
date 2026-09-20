//! 判据：岗位分类体系 + 可选提示，TOML 持久化。
//!
//! 版本纪律：任何保存（或外部手改检测）都会递增 `meta.version`，
//! 缓存目录以 version 命名 —— 改判据即全量缓存失效。

use std::fmt;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};
use crate::hash::sha256_hex;

pub const DEFAULT_CRITERIA_TOML: &str = include_str!("criteria-default.toml");
pub const CRITERIA_FILE_NAME: &str = "criteria.toml";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CriteriaMeta {
    pub version: u64,
    pub schema: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Category {
    pub key: String,
    pub label: String,
    pub description: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct StateSection {
    #[serde(default)]
    pub extra_instructions: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Criteria {
    pub meta: CriteriaMeta,
    #[serde(rename = "category", default)]
    pub categories: Vec<Category>,
    #[serde(default)]
    pub state: StateSection,
}

impl Criteria {
    pub fn default_toml() -> Self {
        Self::parse_toml(DEFAULT_CRITERIA_TOML).expect("embedded default criteria must parse")
    }

    pub fn parse_toml(toml_str: &str) -> CoreResult<Self> {
        let criteria: Criteria =
            toml::from_str(toml_str).map_err(|e| CoreError::DocxMalformed(format!("判据 TOML: {e}")))?;
        criteria.validate()?;
        Ok(criteria)
    }

    pub fn to_toml(&self) -> CoreResult<String> {
        toml::to_string_pretty(self).map_err(|e| CoreError::DocxMalformed(format!("判据序列化: {e}")))
    }

    pub fn validate(&self) -> CoreResult<()> {
        if self.categories.is_empty() {
            return Err(CoreError::DocxMalformed("至少需要一个分类".into()));
        }
        let mut seen = std::collections::HashSet::new();
        for cat in &self.categories {
            if cat.key.trim().is_empty() {
                return Err(CoreError::DocxMalformed("分类 key 不能为空".into()));
            }
            if !seen.insert(cat.key.clone()) {
                return Err(CoreError::DocxMalformed(format!("重复的分类 key: {}", cat.key)));
            }
        }
        Ok(())
    }

    pub fn category_keys(&self) -> Vec<String> {
        self.categories.iter().map(|c| c.key.clone()).collect()
    }

    pub fn version(&self) -> u64 {
        self.meta.version
    }

    /// 从文件加载；若文件被外部手改（记录哈希不匹配）则自动递增版本并回写。
    pub fn load(path: &Path) -> CoreResult<Self> {
        let raw = std::fs::read_to_string(path)?;
        let mut criteria = Self::parse_toml(&raw)?;
        let stored_hash_path = path.with_extension("hash");
        let current_hash = sha256_hex(raw.as_bytes());
        let needs_bump = match std::fs::read_to_string(&stored_hash_path) {
            Ok(stored) => stored.trim() != current_hash,
            Err(_) => false, // 全新文件（首次安装）不 bump
        };
        if needs_bump {
            criteria.meta.version += 1;
            criteria.save(path)?;
        }
        Ok(criteria)
    }

    /// 原子保存：递增版本 + 写文件 + 写内容哈希记录。
    pub fn save(&self, path: &Path) -> CoreResult<()> {
        let mut bumped = self.clone();
        bumped.meta.version += 1;
        bumped.validate()?;
        let toml_str = bumped.to_toml()?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, &toml_str)?;
        std::fs::rename(&tmp, path)?;

        let raw_for_hash = std::fs::read_to_string(path)?;
        let hash_path = path.with_extension("hash");
        std::fs::write(hash_path, sha256_hex(raw_for_hash.as_bytes()))?;
        Ok(())
    }
}

impl fmt::Display for Criteria {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Criteria v{} ({} 类)", self.meta.version, self.categories.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_toml_parses_and_has_nine_categories() {
        let c = Criteria::default_toml();
        assert_eq!(c.meta.version, 1);
        assert_eq!(c.categories.len(), 9);
        assert!(c.category_keys().contains(&"backend".to_string()));
        assert!(c.category_keys().contains(&"other".to_string()));
    }

    #[test]
    fn roundtrip_preserves_data() {
        let c = Criteria::default_toml();
        let toml_str = c.to_toml().unwrap();
        let c2 = Criteria::parse_toml(&toml_str).unwrap();
        assert_eq!(c, c2);
    }

    #[test]
    fn empty_categories_rejected() {
        let toml_str = "[meta]\nversion = 1\nschema = 1\n";
        assert!(Criteria::parse_toml(toml_str).is_err());
    }

    #[test]
    fn duplicate_keys_rejected() {
        let toml_str = r#"
[meta]
version = 1
schema = 1

[[category]]
key = "backend"
label = "后端"
description = "d"

[[category]]
key = "backend"
label = "后端2"
description = "d"
"#;
        assert!(Criteria::parse_toml(toml_str).is_err());
    }

    #[test]
    fn save_bumps_version_and_reload_sees_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(CRITERIA_FILE_NAME);
        let mut c = Criteria::default_toml();
        c.meta.version = 5;
        c.save(&path).unwrap();
        // save bumps
        let reloaded = Criteria::load(&path).unwrap();
        assert_eq!(reloaded.meta.version, 6);
        // reload again without edits: no bump
        let again = Criteria::load(&path).unwrap();
        assert_eq!(again.meta.version, 6);
    }

    #[test]
    fn external_edit_bumps_version_on_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(CRITERIA_FILE_NAME);
        let c = Criteria::default_toml();
        c.save(&path).unwrap();
        let v_after_save = Criteria::load(&path).unwrap().meta.version;

        // 外部手改：绕过 save 直接写文件
        let mut raw = std::fs::read_to_string(&path).unwrap();
        raw = raw.replace("extra_instructions = \"\"", "extra_instructions = \"侧重 rust\"");
        std::fs::write(&path, &raw).unwrap();

        let reloaded = Criteria::load(&path).unwrap();
        assert_eq!(reloaded.meta.version, v_after_save + 1);
        assert_eq!(reloaded.state.extra_instructions, "侧重 rust");
    }
}
