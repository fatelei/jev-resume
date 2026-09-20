//! 提取集成测试：DOCX 现场构建 zip、PDF 用手写最小文档、TXT 直接写入。
//! PDF 测试在本地无 pdfium 时自动跳过（CI 供给后硬性执行）。

use std::io::Write as _;

use jev_core::extract::{detect_format, extract_bytes, extract_file, Format};

fn build_docx(document_xml: &str) -> Vec<u8> {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buf);
        let options: zip::write::SimpleFileOptions = Default::default();
        zip.start_file("word/document.xml", options).unwrap();
        zip.write_all(document_xml.as_bytes()).unwrap();
        zip.finish().unwrap();
    }
    buf.into_inner()
}

fn docx_xml(body: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>{body}</w:body>
</w:document>"#
    )
}

/// 手写最小合法 PDF（1 页，含可提取文本层）。
fn build_minimal_pdf(text: &str) -> Vec<u8> {
    let content = format!("BT /F1 12 Tf 50 700 Td ({text}) Tj ET");
    let objects = [
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
        "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n",
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n",
    ];
    let mut out = Vec::new();
    out.extend_from_slice(b"%PDF-1.4\n");
    let mut offsets = vec![0usize; objects.len() + 2];
    for (i, obj) in objects.iter().enumerate() {
        offsets[i] = out.len();
        out.extend_from_slice(obj.as_bytes());
    }
    // content stream (object 4)
    offsets[3] = out.len();
    let stream_obj = format!("4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n", content.len());
    out.extend_from_slice(stream_obj.as_bytes());
    // font (object 5)
    offsets[4] = out.len();
    out.extend_from_slice(b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n");

    let start_xref = out.len();
    out.extend_from_slice(format!("xref\n0 6\n0000000000 65535 f \n").as_bytes());
    for &off in &offsets[0..5] {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{start_xref}\n%%EOF\n").as_bytes(),
    );
    out
}

#[test]
fn docx_roundtrip_extracts_text() {
    let bytes = build_docx(&docx_xml(
        r#"<w:p><w:r><w:t>姓名: 张三</w:t></w:r></w:p><w:p><w:r><w:t>五年后端经验</w:t></w:r></w:p>"#,
    ));
    let extracted = extract_bytes(Format::Docx, &bytes).unwrap();
    assert!(extracted.text.contains("姓名: 张三"));
    assert!(extracted.text.contains("五年后端经验"));
}

#[test]
fn malformed_docx_raises_docx_malformed() {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buf);
        let options: zip::write::SimpleFileOptions = Default::default();
        zip.start_file("not-document.txt", options).unwrap();
        zip.write_all(b"hello").unwrap();
        zip.finish().unwrap();
    }
    let result = extract_bytes(Format::Docx, &buf.into_inner());
    match result {
        Err(jev_core::CoreError::DocxMalformed(_)) => {}
        other => panic!("expected DocxMalformed, got {other:?}"),
    }
}

#[test]
fn txt_extracts_and_detects() {
    let extracted = extract_bytes(Format::Txt, "纯文本简历内容".as_bytes()).unwrap();
    assert_eq!(extracted.text, "纯文本简历内容");
}

#[test]
fn detect_by_magic_bytes_when_ext_unknown() {
    let dir = tempfile::tempdir().unwrap();
    let pdf_bytes = build_minimal_pdf("Hello Resume");
    let path = dir.path().join("mystery.bin");
    std::fs::write(&path, &pdf_bytes).unwrap();
    assert_eq!(detect_format(&path, &pdf_bytes).unwrap(), Format::Pdf);
}

#[test]
fn doc_extension_rejected_with_hint() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old.doc");
    std::fs::write(&path, b"legacy").unwrap();
    let err = extract_file(&path).unwrap_err();
    assert!(err.user_message().contains("另存为"));
}

#[test]
fn pdf_extracts_text_when_pdfium_available() {
    if !jev_core::extract::pdf::pdfium_available() {
        eprintln!("跳过: 本地无 pdfium 动态库 (CI 会硬性执行)");
        return;
    }
    let pdf_bytes = build_minimal_pdf("Backend engineer with 5 years of experience");
    let extracted = extract_bytes(Format::Pdf, &pdf_bytes).unwrap();
    assert!(extracted.text.contains("Backend engineer"));
    assert_eq!(extracted.page_count, Some(1));
}

#[test]
fn scanned_pdf_raises_no_text_layer() {
    if !jev_core::extract::pdf::pdfium_available() {
        eprintln!("跳过: 本地无 pdfium 动态库");
        return;
    }
    // 合法 PDF 但内容流为空 → 无文本层
    let pdf = build_minimal_pdf("");
    // 空文本需要绕过 50 字阈值检查的构造：直接生成无文字的 PDF
    let mut pdf = pdf;
    // 替换文本指令为空绘制（保持合法）
    let content = b"BT ET";
    let raw = String::from_utf8_lossy(&pdf).into_owned();
    if let Some(pos) = raw.find("BT /F1") {
        let end = raw[pos..].find("endstream").unwrap() + pos;
        let mut rebuilt = String::new();
        rebuilt.push_str(&raw[..pos]);
        rebuilt.push_str(&String::from_utf8_lossy(content));
        rebuilt.push_str(&raw[end..]);
        pdf = rebuilt.into_bytes();
    }
    let result = extract_bytes(Format::Pdf, &pdf);
    assert!(matches!(result, Err(jev_core::CoreError::NoTextLayer)));
}
