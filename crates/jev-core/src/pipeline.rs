//! 批量流水线：导入文件 → 提取 → 缓存查询 → Jev 判定 → 缓存写入。
//!
//! 内部独占一个 tokio Runtime；跨边界全是纯数据事件（async-channel）。
//! 并发由 Semaphore 控制（默认 4）；批次级 CancellationToken 支持中途取消。

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

use crate::cache::{self, CacheFileInfo, CacheRecord};
use crate::criteria::Criteria;
use crate::error::{CoreError, CoreResult};
use crate::extract::{self, Extracted};
use crate::hash::normalized_parts_hash;
use crate::jev::client::{AskParams, JevClient, JevClientConfig};
use crate::jev::types::JevResult;
use crate::questions::{build_questions_payload, QUESTIONS_SCHEMA_VERSION};
use crate::state::build_state;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileState {
    Pending,
    Extracting,
    Queued,
    Judging,
    Done,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileJudgment {
    pub role_category: Option<String>,
    pub role_confidence: Option<f64>,
    pub seniority: Option<String>,
    pub strength: Option<f64>,
    pub inflation: Option<f64>,
    pub model: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
}

#[derive(Debug, Clone, Serialize)]
pub enum PipelineEvent {
    BatchStarted {
        total: usize,
    },
    FileUpdated(Box<FileUpdate>),
    BatchFinished {
        done: usize,
        failed: usize,
        cached: usize,
        cancelled: bool,
    },
    LogLine(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct FileUpdate {
    pub path: PathBuf,
    pub state: FileState,
    pub judgment: Option<FileJudgment>,
    pub error: Option<String>,
    pub elapsed_ms: Option<u64>,
    pub cached: bool,
}

impl FileUpdate {
    fn transition(path: &Path, state: FileState) -> Self {
        Self {
            path: path.to_path_buf(),
            state,
            judgment: None,
            error: None,
            elapsed_ms: None,
            cached: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub concurrency: usize,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            concurrency: 4,
            api_key: String::new(),
            base_url: "https://api.typesafe.ai".into(),
            model: "jev-latest".into(),
        }
    }
}

pub struct Pipeline {
    runtime: tokio::runtime::Runtime,
    token: std::sync::Mutex<CancellationToken>,
    #[allow(dead_code)]
    config: PipelineConfig,
}

impl Pipeline {
    pub fn new(config: PipelineConfig) -> CoreResult<Self> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .map_err(|e| CoreError::JevNetwork(format!("runtime: {e}")))?;
        Ok(Self {
            runtime,
            token: std::sync::Mutex::new(CancellationToken::new()),
            config,
        })
    }

    pub fn cancel(&self) {
        self.token.lock().expect("token").cancel();
    }

    /// 当前批次的取消句柄（跨线程取消用）。
    pub fn token_handle(&self) -> CancellationToken {
        self.token.lock().expect("token").clone()
    }

    /// 上一批次被取消后，开新批次前调用（否则新批次立即视为已取消）。
    pub fn reset_token(&self) {
        *self.token.lock().expect("token") = CancellationToken::new();
    }

    /// 阻塞调用线程直到批次完成（UI 在独立线程/泵任务中调用）。
    /// 事件经 `events` 发出。
    pub fn run_batch(
        &self,
        files: Vec<PathBuf>,
        criteria: Criteria,
        events: async_channel::Sender<PipelineEvent>,
    ) {
        let total = files.len();
        let _ = events.send_blocking(PipelineEvent::BatchStarted { total });

        let client = match JevClient::new(JevClientConfig {
            api_key: self.config.api_key.clone(),
            base_url: self.config.base_url.clone(),
            model: self.config.model.clone(),
            ..Default::default()
        }) {
            Ok(c) => Arc::new(c),
            Err(e) => {
                let _ = events.send_blocking(PipelineEvent::LogLine(e.user_message()));
                let _ = events.send_blocking(PipelineEvent::BatchFinished {
                    done: 0,
                    failed: total,
                    cached: 0,
                    cancelled: false,
                });
                return;
            }
        };

        self.runtime.block_on(async {
            let semaphore = Arc::new(Semaphore::new(self.config.concurrency.max(1)));
            let criteria = Arc::new(criteria);
            let events = Arc::new(events);
            let questions_payload = Arc::new(build_questions_payload(&criteria));

            let mut handles = Vec::with_capacity(total);
            for path in files {
                let permit_sem = semaphore.clone();
                let client = client.clone();
                let criteria = criteria.clone();
                let events = events.clone();
                let questions = questions_payload.clone();
                let token = self.token.lock().expect("token").clone();
                handles.push(tokio::spawn(async move {
                    let _permit = permit_sem.acquire_owned().await;
                    process_file(path, client, criteria, questions, events, token).await
                }));
            }
            let mut done = 0usize;
            let mut failed = 0usize;
            let mut cached = 0usize;
            for handle in handles {
                if let Ok(outcome) = handle.await {
                    match outcome {
                        Outcome::Fresh => done += 1,
                        Outcome::Cached => {
                            done += 1;
                            cached += 1;
                        }
                        Outcome::Failed => failed += 1,
                    }
                } else {
                    failed += 1;
                }
            }
            let cancelled = self.token.lock().expect("token").is_cancelled();
            let _ = events.send_blocking(PipelineEvent::BatchFinished {
                done,
                failed,
                cached,
                cancelled,
            });
        });
    }
}

enum Outcome {
    Fresh,
    Cached,
    Failed,
}

#[allow(clippy::too_many_arguments)]
async fn process_file(
    path: PathBuf,
    client: Arc<JevClient>,
    criteria: Arc<Criteria>,
    questions_payload: Arc<serde_json::Map<String, serde_json::Value>>,
    events: Arc<async_channel::Sender<PipelineEvent>>,
    token: CancellationToken,
) -> Outcome {
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("未知文件")
        .to_string();
    let started = Instant::now();

    let send = |update: FileUpdate| {
        let _ = events.send_blocking(PipelineEvent::FileUpdated(Box::new(update)));
    };

    // ── Extracting（CPU + IO，走 spawn_blocking）──
    send(FileUpdate::transition(&path, FileState::Extracting));
    let extract_path = path.clone();
    let extract_result = tokio::task::spawn_blocking(move || extract::extract_file(&extract_path))
        .await
        .unwrap_or_else(|e| Err(CoreError::Io(std::io::Error::other(e.to_string()))));

    let extracted: Extracted = match extract_result {
        Ok(e) => e,
        Err(err) => {
            send(FileUpdate {
                error: Some(err.user_message()),
                ..FileUpdate::transition(&path, FileState::Failed)
            });
            return Outcome::Failed;
        }
    };

    // ── 哈希 + 缓存查询 ──
    let file_bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) => {
            send(FileUpdate {
                error: Some(CoreError::Io(e).user_message()),
                ..FileUpdate::transition(&path, FileState::Failed)
            });
            return Outcome::Failed;
        }
    };
    let content_hash = normalized_parts_hash(&[
        &QUESTIONS_SCHEMA_VERSION.to_string(),
        &client.config.model,
        &crate::hash::sha256_hex(&file_bytes),
    ]);

    let cache_dir: Option<PathBuf> =
        cache::cache_root().map(|root| cache::cache_dir_for(&root, criteria.version()));
    if let Some(dir) = &cache_dir {
        match cache::get(dir, &content_hash) {
            Ok(Some(record)) => {
                send(FileUpdate {
                    judgment: Some(judgment_from_result(&record.result)),
                    elapsed_ms: Some(record.elapsed_ms),
                    cached: true,
                    ..FileUpdate::transition(&path, FileState::Done)
                });
                return Outcome::Cached;
            }
            Ok(None) => {}
            Err(e) => {
                let _ = events.send_blocking(PipelineEvent::LogLine(e.user_message()));
            }
        }
    }

    if token.is_cancelled() {
        send(FileUpdate {
            error: Some(CoreError::Cancelled.user_message()),
            ..FileUpdate::transition(&path, FileState::Failed)
        });
        return Outcome::Failed;
    }

    // ── Queued → Judging ──
    send(FileUpdate::transition(&path, FileState::Queued));
    let state_text = build_state(
        &crate::state::StateInput {
            file_name: &file_name,
            extracted: &extracted,
        },
        &criteria,
    );

    send(FileUpdate::transition(&path, FileState::Judging));
    let ask = client
        .ask(&AskParams {
            state: &state_text,
            questions: &serde_json::Value::Object(questions_payload.as_ref().clone()),
        })
        .await;

    let result: JevResult = match ask {
        Ok(r) => r,
        Err(err) => {
            send(FileUpdate {
                error: Some(err.user_message()),
                ..FileUpdate::transition(&path, FileState::Failed)
            });
            return Outcome::Failed;
        }
    };

    let elapsed_ms = started.elapsed().as_millis() as u64;

    // ── 写缓存 ──
    if let Some(dir) = &cache_dir {
        let record = CacheRecord {
            schema: cache::CACHE_SCHEMA,
            criteria_version: criteria.version(),
            questions_version: QUESTIONS_SCHEMA_VERSION,
            model: client.config.model.clone(),
            content_hash: content_hash.clone(),
            file: CacheFileInfo {
                name: file_name.clone(),
                size: file_bytes.len() as u64,
                format: extracted.format.as_str().into(),
            },
            state_text: state_text.clone(),
            result: result.clone(),
            elapsed_ms,
            created_at: now_millis(),
        };
        if let Err(e) = cache::put(dir, &record) {
            let _ = events.send_blocking(PipelineEvent::LogLine(e.user_message()));
        }
    }

    send(FileUpdate {
        judgment: Some(judgment_from_result(&result)),
        elapsed_ms: Some(elapsed_ms),
        cached: false,
        ..FileUpdate::transition(&path, FileState::Done)
    });
    Outcome::Fresh
}

fn judgment_from_result(result: &JevResult) -> FileJudgment {
    let role = result.answers.get("role_category");
    let seniority = result.answers.get("seniority");
    let strength = result.answers.get("strength");
    let inflation = result.answers.get("inflation");
    FileJudgment {
        role_category: role.as_ref().and_then(|a| a.choice.clone()),
        role_confidence: role.as_ref().and_then(|a| a.confidence),
        seniority: seniority.as_ref().and_then(|a| a.choice.clone()),
        strength: strength.as_ref().and_then(|a| a.score),
        inflation: inflation.as_ref().and_then(|a| a.noul),
        model: result.model.clone(),
        input_tokens: result.usage.get("input_tokens").copied().unwrap_or(0),
        output_tokens: result.usage.get("output_tokens").copied().unwrap_or(0),
    }
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
