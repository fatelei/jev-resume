//! 错误分类：每个变体携带是否可重试与面向用户的中文消息。

use thiserror::Error;

pub type CoreResult<T> = Result<T, CoreError>;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("pdfium library not found")]
    PdfBind(String),

    #[error("pdf load failed: {0}")]
    PdfLoad(String),

    #[error("no text layer (scanned pdf?)")]
    NoTextLayer,

    #[error("docx malformed: {0}")]
    DocxMalformed(String),

    #[error("jev network error: {0}")]
    JevNetwork(String),

    #[error("jev http {status}: {excerpt}")]
    JevHttp { status: u16, excerpt: String },

    #[error("jev retries exhausted")]
    JevRetriesExhausted,

    #[error("jev bad response: {0}")]
    JevBadResponse(String),

    #[error("not configured: {0}")]
    NotConfigured(String),

    #[error("cache error: {0}")]
    Cache(String),

    #[error("export error: {0}")]
    Export(String),

    #[error("cancelled")]
    Cancelled,
}

impl CoreError {
    /// 网络错误与 429/5xx 可退避重试；其余（含 4xx 业务错误）重试无意义。
    pub fn retryable(&self) -> bool {
        match self {
            CoreError::JevNetwork(_) => true,
            CoreError::JevHttp { status, .. } => *status == 429 || (500..600).contains(status),
            _ => false,
        }
    }

    /// 面向用户的中文消息（表格备注列 / toast）。
    pub fn user_message(&self) -> String {
        match self {
            CoreError::UnsupportedFormat(f) => format!("不支持的格式: {f}"),
            CoreError::Io(e) => format!("文件读取失败: {e}"),
            CoreError::PdfBind(_) => "未找到 pdfium 动态库, 见 README 安装说明".into(),
            CoreError::PdfLoad(e) => format!("PDF 打开失败(可能加密): {e}"),
            CoreError::NoTextLayer => "疑似扫描件, 无文本层".into(),
            CoreError::DocxMalformed(e) => format!("DOCX 解析失败: {e}"),
            CoreError::JevNetwork(e) => format!("网络错误: {e}"),
            CoreError::JevHttp { status, excerpt } => {
                format!("Jev HTTP {status}: {excerpt}")
            }
            CoreError::JevRetriesExhausted => "重试次数耗尽".into(),
            CoreError::JevBadResponse(e) => format!("响应解析失败: {e}"),
            CoreError::NotConfigured(m) => m.clone(),
            CoreError::Cache(e) => format!("缓存失败: {e}"),
            CoreError::Export(e) => format!("导出失败: {e}"),
            CoreError::Cancelled => "已取消".into(),
        }
    }
}
