//! state 文本构建：模型所见 = 调试页所见。
//! 截断：上限 8000 字符，保留头部 6000 + 尾部 1800（教育/证书常在尾部）。

use crate::criteria::Criteria;
use crate::extract::Extracted;

const BODY_CAP: usize = 8000;
const HEAD_KEEP: usize = 6000;
const TAIL_KEEP: usize = 1800;

#[derive(Debug, Clone)]
pub struct StateInput<'a> {
    pub file_name: &'a str,
    pub extracted: &'a Extracted,
}

fn truncate_body(body: &str) -> String {
    let count = body.chars().count();
    if count <= BODY_CAP {
        return body.to_string();
    }
    let head: String = body.chars().take(HEAD_KEEP).collect();
    let tail: String = body.chars().skip(count - TAIL_KEEP).collect();
    let omitted = count - HEAD_KEEP - TAIL_KEEP;
    format!("{head}\n…(中间省略{omitted}字)…\n{tail}")
}

pub fn build_state(input: &StateInput<'_>, criteria: &Criteria) -> String {
    let e = input.extracted;
    let pages = e
        .page_count
        .map(|p| p.to_string())
        .unwrap_or_else(|| "未知".into());

    let mut sections = vec![
        format!("文件名: {}", input.file_name),
        format!("格式: {}, {} 页", e.format.as_str(), pages),
        format!(
            "提取字数: {} (提取完整度: {})",
            e.text.chars().count(),
            if e.text.trim().is_empty() { "空" } else { "完整" }
        ),
    ];
    for warning in &e.warnings {
        sections.push(format!("[注意] {warning}"));
    }
    if !criteria.state.extra_instructions.trim().is_empty() {
        sections.push(format!("[hint] {}", criteria.state.extra_instructions.trim()));
    }
    sections.push("[正文]".into());
    sections.push(truncate_body(&e.text));
    sections.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extracted(text: &str) -> Extracted {
        Extracted {
            format: crate::extract::Format::Pdf,
            text: text.into(),
            page_count: Some(2),
            warnings: vec![],
        }
    }

    #[test]
    fn renders_header_and_body() {
        let e = extracted("三年后端经验, 熟悉分布式系统。");
        let state = build_state(
            &StateInput {
                file_name: "张三_后端_5年.pdf",
                extracted: &e,
            },
            &Criteria::default_toml(),
        );
        assert!(state.contains("文件名: 张三_后端_5年.pdf"));
        assert!(state.contains("格式: PDF, 2 页"));
        assert!(state.contains("提取字数: 16"));
        assert!(state.contains("[正文]"));
        assert!(state.contains("三年后端经验"));
    }

    #[test]
    fn truncates_long_body_keeping_head_and_tail() {
        let mut body = "A".repeat(7000);
        body.push_str(&"B".repeat(2900));
        body.push_str("TAIL_MARKER");
        let e = extracted(&body);
        let state = build_state(
            &StateInput {
                file_name: "f.pdf",
                extracted: &e,
            },
            &Criteria::default_toml(),
        );
        assert!(state.contains("…(中间省略"));
        assert!(state.contains("TAIL_MARKER"));
        assert!(state.contains('A'));
        assert!(state.chars().count() < 8000);
    }

    #[test]
    fn hint_injected_when_configured() {
        let mut c = Criteria::default_toml();
        c.state.extra_instructions = "侧重 Rust 经历".into();
        let e = extracted("正文");
        let state = build_state(
            &StateInput {
                file_name: "f.pdf",
                extracted: &e,
            },
            &c,
        );
        assert!(state.contains("[hint] 侧重 Rust 经历"));
    }

    #[test]
    fn no_hint_when_empty() {
        let e = extracted("正文");
        let state = build_state(
            &StateInput {
                file_name: "f.pdf",
                extracted: &e,
            },
            &Criteria::default_toml(),
        );
        assert!(!state.contains("[hint]"));
    }
}
