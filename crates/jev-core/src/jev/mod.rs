//! TypeSafe Jev /v1/systemone 客户端。
//!
//! 移植自生产验证的 zhihu-jev/lib/jev/client.ts：容错解析（拒绝 bool）、
//! 429/5xx 指数退避重试（2s/4s，最多 3 次）、错误摘录 ≤300 字符。

pub mod client;
pub mod types;

pub use client::{AskParams, JevClient};
pub use types::{JevAnswer, JevResult, QuestionSet};

use crate::error::{CoreError, CoreResult};
use crate::jev::types::JevAnswerRaw;
use serde_json::Value;

pub(crate) const EXCERPT_LEN: usize = 300;

pub(crate) fn excerpt(text: &str) -> String {
    text.chars().take(EXCERPT_LEN).collect()
}

pub(crate) fn as_float(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        _ => None, // bool / string / null 一律拒绝（与 TS 端一致）
    }
}

pub(crate) fn as_float_map(value: &Value) -> std::collections::HashMap<String, f64> {
    let mut out = std::collections::HashMap::new();
    if let Value::Object(map) = value {
        for (k, v) in map {
            if let Some(f) = as_float(v) {
                out.insert(k.clone(), f);
            }
        }
    }
    out
}

fn parse_answer(name: &str, raw: &Value, body: &str) -> CoreResult<JevAnswer> {
    let answer: JevAnswerRaw = serde_json::from_value(raw.clone())
        .map_err(|e| CoreError::JevBadResponse(format!("答案 {name}: {e} — {}", excerpt(body))))?;

    let confidence = answer.confidence.as_ref().and_then(as_float);
    match answer.kind.as_str() {
        "choice" => {
            let choice = match &answer.choice {
                Some(Value::String(s)) => s.clone(),
                _ => {
                    return Err(CoreError::JevBadResponse(format!(
                        "答案 {name} 缺少 choice: {}",
                        excerpt(body)
                    )))
                }
            };
            Ok(JevAnswer {
                kind: "choice".into(),
                choice: Some(choice),
                probabilities: answer
                    .probabilities
                    .as_ref()
                    .map(as_float_map)
                    .unwrap_or_default(),
                score: None,
                noul: None,
                confidence,
            })
        }
        "score" => {
            let score = answer.score.as_ref().and_then(as_float).ok_or_else(|| {
                CoreError::JevBadResponse(format!("答案 {name} 缺少 score: {}", excerpt(body)))
            })?;
            Ok(JevAnswer {
                kind: "score".into(),
                choice: None,
                probabilities: Default::default(),
                score: Some(score),
                noul: None,
                confidence,
            })
        }
        "noul" => {
            let noul = answer.noul.as_ref().and_then(as_float).ok_or_else(|| {
                CoreError::JevBadResponse(format!("答案 {name} 缺少 noul: {}", excerpt(body)))
            })?;
            Ok(JevAnswer {
                kind: "noul".into(),
                choice: None,
                probabilities: Default::default(),
                score: None,
                noul: Some(noul),
                confidence,
            })
        }
        other => Err(CoreError::JevBadResponse(format!(
            "答案 {name} 类型未知 {other}: {}",
            excerpt(body)
        ))),
    }
}

pub(crate) fn parse_response(payload: &Value, body: &str) -> CoreResult<JevResult> {
    let answers_raw = payload
        .get("answers")
        .and_then(|a| a.as_object())
        .ok_or_else(|| CoreError::JevBadResponse(format!("Jev 响应缺少 answers: {}", excerpt(body))))?;
    if answers_raw.is_empty() {
        return Err(CoreError::JevBadResponse(format!(
            "Jev 响应缺少 answers: {}",
            excerpt(body)
        )));
    }

    let mut answers = std::collections::HashMap::new();
    for (name, raw) in answers_raw {
        answers.insert(name.clone(), parse_answer(name, raw, body)?);
    }

    let mut usage = std::collections::HashMap::new();
    if let Some(Value::Object(map)) = payload.get("usage") {
        for (k, v) in map {
            if let Some(n) = v.as_i64() {
                usage.insert(k.clone(), n);
            }
        }
    }

    Ok(JevResult {
        model: payload
            .get("model")
            .and_then(|m| m.as_str())
            .map(|s| s.to_string()),
        answers,
        usage,
    })
}

pub fn is_retryable_status(status: u16) -> bool {
    status == 429 || (500..600).contains(&status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_choice_answer() {
        let body = r#"{"model":"jev-1","answers":{"decision":{"type":"choice","choice":"backend","probabilities":{"backend":0.8},"confidence":0.7}},"usage":{"input_tokens":10}}"#;
        let payload: Value = serde_json::from_str(body).unwrap();
        let out = parse_response(&payload, body).unwrap();
        let a = out.answers.get("decision").unwrap();
        assert_eq!(a.choice.as_deref(), Some("backend"));
        assert_eq!(a.probabilities.get("backend"), Some(&0.8));
        assert_eq!(out.usage.get("input_tokens"), Some(&10));
    }

    #[test]
    fn rejects_bool_score() {
        let body = r#"{"answers":{"s":{"type":"score","score":true}}}"#;
        let payload: Value = serde_json::from_str(body).unwrap();
        assert!(parse_response(&payload, body).is_err());
    }

    #[test]
    fn missing_choice_raises_with_excerpt() {
        let body = r#"{"answers":{"d":{"type":"choice"}}}"#;
        let payload: Value = serde_json::from_str(body).unwrap();
        let err = parse_response(&payload, body).unwrap_err();
        assert!(err.to_string().contains("缺少 choice"));
    }

    #[test]
    fn missing_answers_raises() {
        let body = r#"{"model":"m"}"#;
        let payload: Value = serde_json::from_str(body).unwrap();
        let err = parse_response(&payload, body).unwrap_err();
        assert!(err.to_string().contains("answers"));
    }

    #[test]
    fn empty_answers_raises() {
        let body = r#"{"answers":{}}"#;
        let payload: Value = serde_json::from_str(body).unwrap();
        assert!(parse_response(&payload, body).is_err());
    }

    #[test]
    fn noul_parses() {
        let body = r#"{"answers":{"n":{"type":"noul","noul":0.05}}}"#;
        let payload: Value = serde_json::from_str(body).unwrap();
        let out = parse_response(&payload, body).unwrap();
        assert_eq!(out.answers.get("n").unwrap().noul, Some(0.05));
    }
}
