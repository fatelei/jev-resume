//! HTTP 客户端：POST {base}/v1/systemone，429/5xx 指数退避（2s/4s，共 3 次尝试）。

use std::time::Duration;

use serde_json::{json, Value};

use crate::error::{CoreError, CoreResult};
use crate::jev::{parse_response, types::JevResult};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);
const DEFAULT_MAX_RETRIES: u32 = 3;
const BACKOFF_BASE: Duration = Duration::from_secs(2);

#[derive(Debug, Clone)]
pub struct JevClientConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub timeout: Duration,
    pub max_retries: u32,
    /// 重试退避基数；测试置 0 避免真实等待。
    pub retry_backoff: Duration,
}

impl Default for JevClientConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: "https://api.typesafe.ai".into(),
            model: "jev-latest".into(),
            timeout: DEFAULT_TIMEOUT,
            max_retries: DEFAULT_MAX_RETRIES,
            retry_backoff: BACKOFF_BASE,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AskParams<'a> {
    pub state: &'a str,
    pub questions: &'a Value,
}

pub struct JevClient {
    http: reqwest::Client,
    pub config: JevClientConfig,
}

impl JevClient {
    pub fn new(config: JevClientConfig) -> CoreResult<Self> {
        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| CoreError::JevNetwork(e.to_string()))?;
        Ok(Self { http, config })
    }

    fn url(&self) -> String {
        format!(
            "{}/v1/systemone",
            self.config.base_url.trim_end_matches('/')
        )
    }

    fn build_body(&self, params: &AskParams<'_>) -> Value {
        json!({
            "state": params.state,
            "model": self.config.model,
            "questions": params.questions,
        })
    }

    /// 单次提问；网络与 429/5xx 自动退避重试，其余错误立即返回。
    pub async fn ask(&self, params: &AskParams<'_>) -> CoreResult<JevResult> {
        if self.config.api_key.trim().is_empty() {
            return Err(CoreError::NotConfigured(
                "请先在设置中填写 TypeSafe API Key".into(),
            ));
        }
        let body = self.build_body(params);
        let url = self.url();
        let max = self.config.max_retries.max(1);

        for attempt in 0..max {
            let result = self.send_once(&url, &body).await;
            match result {
                Ok(text) => {
                    let payload: Value = serde_json::from_str(&text)
                        .map_err(|_| CoreError::JevBadResponse(crate::jev::excerpt(&text)))?;
                    return parse_response(&payload, &text);
                }
                Err(err) => {
                    let is_last = attempt + 1 >= max;
                    let retryable =
                        matches!(&err, CoreError::JevNetwork(_) | CoreError::JevHttp { .. })
                            && err.retryable();
                    if !retryable || is_last {
                        if retryable {
                            return Err(CoreError::JevRetriesExhausted);
                        }
                        return Err(err);
                    }
                    tokio::time::sleep(self.config.retry_backoff * 2u32.pow(attempt)).await;
                }
            }
        }
        Err(CoreError::JevRetriesExhausted)
    }

    async fn send_once(&self, url: &str, body: &Value) -> CoreResult<String> {
        let resp = self
            .http
            .post(url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await
            .map_err(|e| CoreError::JevNetwork(e.to_string()))?;

        let status = resp.status().as_u16();
        let text = resp
            .text()
            .await
            .map_err(|e| CoreError::JevNetwork(e.to_string()))?;

        if status >= 400 {
            return Err(CoreError::JevHttp {
                status,
                excerpt: crate::jev::excerpt(&text),
            });
        }
        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn client(base: String) -> JevClient {
        JevClient::new(JevClientConfig {
            api_key: "test-key".into(),
            base_url: base,
            retry_backoff: Duration::ZERO,
            ..Default::default()
        })
        .unwrap()
    }

    fn ok_body() -> Value {
        json!({
            "model": "jev-1.13.0",
            "answers": {
                "role_category": {"type": "choice", "choice": "backend",
                    "probabilities": {"backend": 0.9}, "confidence": 0.8},
                "strength": {"type": "score", "score": 6.5, "confidence": 0.8},
                "inflation": {"type": "noul", "noul": 0.1}
            },
            "usage": {"input_tokens": 1200, "output_tokens": 100}
        })
    }

    #[tokio::test]
    async fn posts_verified_contract_shape() {
        let server = MockServer::start().await;
        let expected = ok_body();
        Mock::given(method("POST"))
            .and(path("/v1/systemone"))
            .and(header("Authorization", "Bearer test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(expected))
            .mount(&server)
            .await;

        let c = client(server.uri());
        let questions = json!({"role_category": {"type": "noul", "instructions": "x"}});
        let out = c
            .ask(&AskParams {
                state: "测试正文",
                questions: &questions,
            })
            .await
            .unwrap();

        assert_eq!(out.model.as_deref(), Some("jev-1.13.0"));
        assert_eq!(
            out.answers.get("role_category").unwrap().choice.as_deref(),
            Some("backend")
        );
        assert_eq!(out.usage.get("input_tokens"), Some(&1200));
    }

    #[tokio::test]
    async fn retries_429_then_succeeds() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(429))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ok_body()))
            .mount(&server)
            .await;

        let c = client(server.uri());
        let questions = json!({});
        let out = c
            .ask(&AskParams {
                state: "s",
                questions: &questions,
            })
            .await
            .unwrap();
        assert_eq!(out.answers.len(), 3);
    }

    #[tokio::test]
    async fn exhausts_429_retries() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(429))
            .expect(3)
            .mount(&server)
            .await;

        let c = client(server.uri());
        let questions = json!({});
        let err = c
            .ask(&AskParams {
                state: "s",
                questions: &questions,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, CoreError::JevRetriesExhausted));
    }

    #[tokio::test]
    async fn does_not_retry_on_401() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(401).set_body_string("unauthorized"))
            .expect(1)
            .mount(&server)
            .await;

        let c = client(server.uri());
        let questions = json!({});
        let err = c
            .ask(&AskParams {
                state: "s",
                questions: &questions,
            })
            .await
            .unwrap_err();
        match err {
            CoreError::JevHttp { status, .. } => assert_eq!(status, 401),
            other => panic!("expected JevHttp, got {other}"),
        }
    }

    #[tokio::test]
    async fn empty_key_raises_not_configured() {
        let c = JevClient::new(JevClientConfig {
            api_key: "  ".into(),
            ..Default::default()
        })
        .unwrap();
        let questions = json!({});
        let err = c
            .ask(&AskParams {
                state: "s",
                questions: &questions,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, CoreError::NotConfigured(_)));
    }
}
