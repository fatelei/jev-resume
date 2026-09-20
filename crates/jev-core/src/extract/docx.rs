//! DOCX 提取：zip → word/document.xml → quick-xml 提取 w:t 文本段。
//! w:p / w:br 产生换行，w:tab 产生空格；naive 遍历天然覆盖表格与文本框
//! （w:txbxContent 内仍是 w:t）。页眉页脚与域代码不在 document.xml，自然跳过。

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::error::{CoreError, CoreResult};
use crate::extract::Extracted;
use std::io::Read as _;

pub fn extract(bytes: &[u8]) -> CoreResult<Extracted> {
    let text = extract_text(bytes)?;
    Ok(Extracted {
        format: crate::extract::Format::Docx,
        text,
        page_count: None,
        warnings: vec![],
    })
}

pub fn extract_text(bytes: &[u8]) -> CoreResult<String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| CoreError::DocxMalformed(e.to_string()))?;
    let mut document = archive
        .by_name("word/document.xml")
        .map_err(|_| CoreError::DocxMalformed("缺少 word/document.xml".into()))?;

    let mut xml = String::new();
    document
        .read_to_string(&mut xml)
        .map_err(|e| CoreError::DocxMalformed(e.to_string()))?;
    parse_document_xml(&xml)
}

fn local_name(name: &[u8]) -> &[u8] {
    match name.iter().rposition(|&b| b == b':') {
        Some(pos) => &name[pos + 1..],
        None => name,
    }
}

fn parse_document_xml(xml: &str) -> CoreResult<String> {
    let mut reader = Reader::from_str(xml);
    let mut out = String::new();
    let mut buf = Vec::new();
    let mut in_w_t = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => match local_name(e.name().as_ref()) {
                b"p" | b"br" => out.push('\n'),
                b"tab" => out.push(' '),
                b"t" => in_w_t = true,
                _ => {}
            },
            Ok(Event::Empty(ref e)) => match local_name(e.name().as_ref()) {
                b"br" => out.push('\n'),
                b"tab" => out.push(' '),
                _ => {}
            },
            Ok(Event::End(ref e)) => {
                if local_name(e.name().as_ref()) == b"t" {
                    in_w_t = false;
                }
            }
            Ok(Event::Text(ref t)) => {
                if in_w_t {
                    let decoded = t
                        .unescape()
                        .map_err(|e| CoreError::DocxMalformed(e.to_string()))?;
                    out.push_str(&decoded);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(CoreError::DocxMalformed(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    let cleaned: String = out
        .lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n");
    Ok(cleaned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_runs_paragraphs_and_tabs() {
        let xml = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>姓名: 张三</w:t></w:r></w:p>
    <w:p><w:r><w:t>技能</w:t><w:tab/><w:t>Rust Go</w:t></w:r></w:p>
    <w:p><w:r><w:t>第一行</w:t><w:br/><w:t>第二行</w:t></w:r></w:p>
  </w:body>
</w:document>"#;
        let text = parse_document_xml(xml).unwrap();
        assert!(text.contains("姓名: 张三"));
        assert!(text.contains("技能\tRust Go") || text.contains("技能 Rust Go"));
        assert!(text.contains("第一行\n第二行"));
    }

    #[test]
    fn unescapes_entities() {
        let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:p><w:r><w:t>A &amp; B &lt;C&gt;</w:t></w:r></w:p></w:document>"#;
        let text = parse_document_xml(xml).unwrap();
        assert!(text.contains("A & B <C>"));
    }

    #[test]
    fn covers_textboxes_and_tables() {
        // w:txbxContent / w:tbl 内仍是 w:t —— naive 遍历天然覆盖
        let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>
  <w:tbl><w:tr><w:tc><w:p><w:r><w:t>表格单元格</w:t></w:r></w:p></w:tc></w:tr></w:tbl>
  <w:p><w:r><w:txbxContent><w:t>文本框内容</w:t></w:txbxContent></w:r></w:p>
</w:body></w:document>"#;
        let text = parse_document_xml(xml).unwrap();
        assert!(text.contains("表格单元格"));
        assert!(text.contains("文本框内容"));
    }

    #[test]
    fn malformed_xml_raises() {
        assert!(parse_document_xml("<w:document><a></b></w:document>").is_err());
    }
}
