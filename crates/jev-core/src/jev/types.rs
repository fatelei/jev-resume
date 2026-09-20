//! Jev 请求/响应类型。

use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::{json, Value};

/// 单个问题定义（choice/score/noul），序列化即线上 payload。
#[derive(Debug, Clone)]
pub enum QuestionSpec {
    Choice {
        instructions: String,
        /// label → description（对象）
        criteria: BTreeMap<String, String>,
    },
    Score {
        instructions: String,
        /// 分档图例（数组）
        criteria: Vec<String>,
    },
    Noul {
        instructions: String,
    },
}

impl QuestionSpec {
    pub fn to_json(&self) -> Value {
        match self {
            QuestionSpec::Choice {
                instructions,
                criteria,
            } => json!({
                "type": "choice",
                "instructions": instructions,
                "criteria": criteria,
            }),
            QuestionSpec::Score {
                instructions,
                criteria,
            } => json!({
                "type": "score",
                "instructions": instructions,
                "criteria": criteria,
            }),
            QuestionSpec::Noul { instructions } => json!({
                "type": "noul",
                "instructions": instructions,
            }),
        }
    }
}

pub type QuestionSet = BTreeMap<String, QuestionSpec>;

#[derive(Debug, Clone, Deserialize)]
pub struct JevAnswerRaw {
    #[serde(rename = "type")]
    pub kind: String,
    pub choice: Option<Value>,
    pub probabilities: Option<Value>,
    pub score: Option<Value>,
    pub noul: Option<Value>,
    pub confidence: Option<Value>,
    pub legend: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct JevAnswer {
    pub kind: String,
    pub choice: Option<String>,
    pub probabilities: std::collections::HashMap<String, f64>,
    pub score: Option<f64>,
    pub noul: Option<f64>,
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct JevResult {
    pub model: Option<String>,
    pub answers: std::collections::HashMap<String, JevAnswer>,
    pub usage: std::collections::HashMap<String, i64>,
}
