//! PDF 提取：pdfium-render 绑定。
//! 库发现链：PDFIUM_DYNAMIC_LIB_PATH env → 可执行文件旁 → 系统库路径。
//! 本地无 pdfium 时 pdf 测试自动跳过；CI 供给后硬性通过。

use pdfium_render::prelude::*;

use crate::error::{CoreError, CoreResult};
use crate::extract::Extracted;

/// 逐 OS 尝试的库文件名。
fn lib_file_name() -> &'static str {
    #[cfg(target_os = "macos")]
    return "libpdfium.dylib";
    #[cfg(target_os = "windows")]
    return "pdfium.dll";
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    return "libpdfium.so";
}

/// 发现 pdfium 动态库路径；找不到返回 None（调用方决定报错或跳过）。
/// 顺序：环境变量 → 二进制旁边 → 安装包 resources（tauri bundle）→ 系统。
pub fn discover_pdfium_path() -> Option<std::path::PathBuf> {
    if let Ok(env_path) = std::env::var("PDFIUM_DYNAMIC_LIB_PATH") {
        let p = std::path::PathBuf::from(env_path);
        if p.exists() {
            return Some(p);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let name = lib_file_name();
            let p = dir.join(name);
            if p.exists() {
                return Some(p);
            }
            // bundle.resources 的 glob 保留相对路径 pdfium/lib/：
            // Windows/Linux 资源在 exe 旁的 pdfium/lib/，macOS .app 在 ../Resources/pdfium/lib/
            let mut candidates = vec![dir.join("pdfium").join("lib").join(name)];
            #[cfg(target_os = "macos")]
            candidates.push(dir.join("../Resources/pdfium/lib").join(name));
            for p in candidates {
                if p.exists() {
                    return Some(p);
                }
            }
        }
    }
    None
}

/// 绑定 pdfium；失败返回 PdfBind（含排查指引）。
pub fn bind_pdfium() -> Result<Pdfium, CoreError> {
    let bindings = if let Some(path) = discover_pdfium_path() {
        Pdfium::bind_to_library(&path)
            .map_err(|e| CoreError::PdfBind(format!("{}: {e}", path.display())))?
    } else {
        Pdfium::bind_to_system_library().map_err(|e| {
            CoreError::PdfBind(format!(
                "{e}; 请运行 scripts/fetch-pdfium.sh 或设置 PDFIUM_DYNAMIC_LIB_PATH"
            ))
        })?
    };
    Ok(Pdfium::new(bindings))
}

pub fn extract(bytes: &[u8]) -> CoreResult<Extracted> {
    let pdfium = bind_pdfium()?;
    let document = pdfium
        .load_pdf_from_byte_slice(bytes, None)
        .map_err(|e| CoreError::PdfLoad(e.to_string()))?;

    let page_count = document.pages().len() as u32;
    let mut text = String::new();
    for page in document.pages().iter() {
        let page_text = page
            .text()
            .map_err(|e| CoreError::PdfLoad(e.to_string()))?
            .all();
        text.push_str(&page_text);
        text.push('\n');
    }

    if text.trim().chars().count() < 50 {
        return Err(CoreError::NoTextLayer);
    }

    Ok(Extracted {
        format: crate::extract::Format::Pdf,
        text,
        page_count: Some(page_count),
        warnings: vec![],
    })
}

/// 本地是否具备 pdf 测试条件（CI 供给后为 true）。
pub fn pdfium_available() -> bool {
    discover_pdfium_path().is_some() || bind_pdfium().is_ok()
}
