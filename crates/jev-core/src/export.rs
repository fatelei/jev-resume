//! 导出：CSV（带 BOM，Excel 中文兼容）与 JSON。

use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRow {
    pub file_name: String,
    pub status: String,
    pub role_category: String,
    pub seniority: String,
    pub strength: Option<f64>,
    pub inflation: Option<f64>,
    pub elapsed_ms: Option<u64>,
    pub cached: bool,
    pub note: String,
}

const CSV_HEADERS: [&str; 9] = [
    "文件名",
    "状态",
    "岗位分类",
    "资历级别",
    "技术强度(0-10)",
    "注水嫌疑(0-1)",
    "耗时(ms)",
    "缓存",
    "备注",
];

pub fn rows_to_csv<W: Write>(rows: &[ExportRow], writer: &mut W) -> CoreResult<()> {
    // UTF-8 BOM: Excel 打开中文不乱码
    writer
        .write_all(&[0xEF, 0xBB, 0xBF])
        .map_err(|e| CoreError::Export(e.to_string()))?;
    let mut csv_writer = csv::Writer::from_writer(writer);
    csv_writer
        .write_record(CSV_HEADERS)
        .map_err(|e| CoreError::Export(e.to_string()))?;
    for row in rows {
        let strength = row.strength.map(|v| format!("{v:.1}")).unwrap_or_default();
        let inflation = row.inflation.map(|v| format!("{v:.2}")).unwrap_or_default();
        let elapsed = row.elapsed_ms.map(|v| v.to_string()).unwrap_or_default();
        let cached = if row.cached {
            "是".to_string()
        } else {
            String::new()
        };
        csv_writer
            .write_record([
                row.file_name.as_str(),
                row.status.as_str(),
                row.role_category.as_str(),
                row.seniority.as_str(),
                strength.as_str(),
                inflation.as_str(),
                elapsed.as_str(),
                cached.as_str(),
                row.note.as_str(),
            ])
            .map_err(|e| CoreError::Export(e.to_string()))?;
    }
    csv_writer
        .flush()
        .map_err(|e| CoreError::Export(e.to_string()))?;
    Ok(())
}

pub fn export_csv(rows: &[ExportRow], path: &Path) -> CoreResult<()> {
    let file = std::fs::File::create(path).map_err(|e| CoreError::Export(e.to_string()))?;
    let mut writer = std::io::BufWriter::new(file);
    rows_to_csv(rows, &mut writer)
}

pub fn export_json(rows: &[ExportRow], path: &Path) -> CoreResult<()> {
    let raw = serde_json::to_string_pretty(rows).map_err(|e| CoreError::Export(e.to_string()))?;
    std::fs::write(path, raw).map_err(|e| CoreError::Export(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_rows() -> Vec<ExportRow> {
        vec![
            ExportRow {
                file_name: "张三_后端_5年.pdf".into(),
                status: "完成".into(),
                role_category: "backend".into(),
                seniority: "mid_3_5".into(),
                strength: Some(6.5),
                inflation: Some(0.10),
                elapsed_ms: Some(830),
                cached: false,
                note: "".into(),
            },
            ExportRow {
                file_name: "李四.docx".into(),
                status: "失败".into(),
                role_category: "".into(),
                seniority: "".into(),
                strength: None,
                inflation: None,
                elapsed_ms: None,
                cached: false,
                note: "疑似扫描件, 无文本层".into(),
            },
        ]
    }

    #[test]
    fn csv_starts_with_bom_and_contains_rows() {
        let mut buf: Vec<u8> = Vec::new();
        rows_to_csv(&sample_rows(), &mut buf).unwrap();
        assert_eq!(&buf[0..3], &[0xEF, 0xBB, 0xBF]);
        let text = String::from_utf8(buf).unwrap();
        assert!(text.contains("张三_后端_5年.pdf"));
        assert!(text.contains("backend"));
        assert!(text.contains("疑似扫描件"));
        assert!(text.contains("6.5"));
        // 失败行数值列留空
        assert!(text.contains("李四.docx,失败"));
    }

    #[test]
    fn csv_escapes_commas() {
        let rows = vec![ExportRow {
            file_name: "a,b.csv".into(),
            note: "错误, 带逗号".into(),
            status: "失败".into(),
            role_category: String::new(),
            seniority: String::new(),
            strength: None,
            inflation: None,
            elapsed_ms: None,
            cached: false,
        }];
        let mut buf: Vec<u8> = Vec::new();
        rows_to_csv(&rows, &mut buf).unwrap();
        let text = String::from_utf8(buf).unwrap();
        assert!(text.contains("\"a,b.csv\""));
    }

    #[test]
    fn json_export_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.json");
        export_json(&sample_rows(), &path).unwrap();
        let raw = std::fs::read_to_string(&path).unwrap();
        let parsed: Vec<ExportRow> = serde_json::from_str(&raw).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].role_category, "backend");
    }
}
