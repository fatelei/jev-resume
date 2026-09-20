//! 内容哈希：缓存键。归一化 parts + SHA-256，截 32 hex。
//! 哈希输入包含 questions schema 版本与 model —— 换判据或模型即缓存失效。

use sha2::{Digest, Sha256};

const SEPARATOR: char = '\u{241F}'; // ␟

pub fn sha256_hex(input: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    let digest = hasher.finalize();
    hex::encode(digest)
}

/// 归一化（连续空白折叠为单空格 + trim）后拼接哈希，截 32 hex。
pub fn normalized_parts_hash(parts: &[&str]) -> String {
    let normalized: Vec<String> = parts
        .iter()
        .map(|p| p.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect();
    let joined = normalized.join(&SEPARATOR.to_string());
    let full = sha256_hex(joined.as_bytes());
    full[..32].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_is_stable() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn normalized_hash_is_stable() {
        let a = normalized_parts_hash(&["问题", "作者", "正文"]);
        let b = normalized_parts_hash(&["问题", "作者", "正文"]);
        assert_eq!(a, b);
        assert_eq!(a.len(), 32);
    }

    #[test]
    fn whitespace_is_normalized() {
        let a = normalized_parts_hash(&["问题", "作者", "行一\n\n行二  词"]);
        let b = normalized_parts_hash(&[" 问题 ", "作者", "行一 行二 词"]);
        assert_eq!(a, b);
    }

    #[test]
    fn different_parts_differ() {
        let a = normalized_parts_hash(&["问题", "作者", "正文A"]);
        let b = normalized_parts_hash(&["问题", "作者", "正文B"]);
        assert_ne!(a, b);
    }

    #[test]
    fn version_busts_hash() {
        let v1 = normalized_parts_hash(&["1", "问题", "作者", "正文"]);
        let v2 = normalized_parts_hash(&["2", "问题", "作者", "正文"]);
        assert_ne!(v1, v2);
    }
}
