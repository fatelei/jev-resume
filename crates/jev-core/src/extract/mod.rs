//! 简历文本提取：格式探测（扩展名 + 魔数）→ 分发到对应提取器。

pub mod docx;
pub mod pdf;
pub mod txt;

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Pdf,
    Docx,
    Txt,
}

impl Format {
    pub fn as_str(&self) -> &'static str {
        match self {
            Format::Pdf => "PDF",
            Format::Docx => "DOCX",
            Format::Txt => "TXT",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extracted {
    pub format: Format,
    pub text: String,
    pub page_count: Option<u32>,
    pub warnings: Vec<String>,
}

/// 从文件路径提取文本。
pub fn extract_file(path: &Path) -> CoreResult<Extracted> {
    let bytes = std::fs::read(path)?;
    let format = detect_format(path, &bytes)?;
    extract_bytes(format, &bytes)
}

pub fn extract_bytes(format: Format, bytes: &[u8]) -> CoreResult<Extracted> {
    match format {
        Format::Txt => {
            let text = txt::extract(bytes);
            Ok(Extracted {
                format,
                text,
                page_count: None,
                warnings: vec![],
            })
        }
        Format::Docx => docx::extract(bytes),
        Format::Pdf => pdf::extract(bytes),
    }
}

/// 按扩展名优先、魔数兜底探测格式。
pub fn detect_format(path: &Path, bytes: &[u8]) -> CoreResult<Format> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "pdf" => return Ok(Format::Pdf),
        "docx" => return Ok(Format::Docx),
        "txt" | "md" | "markdown" => return Ok(Format::Txt),
        "doc" => {
            return Err(CoreError::UnsupportedFormat(
                ".doc (旧格式), 请另存为 .docx".into(),
            ))
        }
        _ => {}
    }
    // 扩展名未知时用魔数兜底
    if bytes.starts_with(b"%PDF-") {
        return Ok(Format::Pdf);
    }
    if bytes.len() >= 2 && bytes.starts_with(b"PK") {
        return Ok(Format::Docx);
    }
    // 尝试按 UTF-8 文本处理
    if std::str::from_utf8(bytes).is_ok() {
        return Ok(Format::Txt);
    }
    Err(CoreError::UnsupportedFormat(format!(
        "无法识别的文件: {}",
        path.display()
    )))
}

/// 遍历目录收集支持的简历文件（上限 500 个）。
pub fn collect_resume_files(paths: &[std::path::PathBuf]) -> CoreResult<Vec<std::path::PathBuf>> {
    const MAX_FILES: usize = 500;
    let mut out = Vec::new();
    for path in paths {
        if path.is_file() {
            if is_supported_ext(path) {
                out.push(path.clone());
            }
            continue;
        }
        for entry in walkdir::WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }
            if is_supported_ext(entry.path()) {
                out.push(entry.path().to_path_buf());
                if out.len() >= MAX_FILES {
                    return Err(CoreError::UnsupportedFormat(format!(
                        "一次最多导入 {MAX_FILES} 个文件"
                    )));
                }
            }
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

fn is_supported_ext(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .as_deref(),
        Some("pdf") | Some("docx") | Some("txt") | Some("md") | Some("markdown")
    )
}
