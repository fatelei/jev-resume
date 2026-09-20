//! 流水线集成测试：批次/缓存命中零HTTP/并发上限/取消。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use jev_core::cache;
use jev_core::criteria::Criteria;
use jev_core::pipeline::{Pipeline, PipelineConfig, PipelineEvent};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Respond, ResponseTemplate};

fn write_resume(dir: &std::path::Path, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, content).unwrap();
    path
}

fn ok_body() -> serde_json::Value {
    json!({
        "model": "jev-1.13.0",
        "answers": {
            "role_category": {"type": "choice", "choice": "backend", "probabilities": {"backend": 0.9}, "confidence": 0.8},
            "seniority": {"type": "choice", "choice": "mid_3_5", "probabilities": {}, "confidence": 0.7},
            "strength": {"type": "score", "score": 6.0, "confidence": 0.8},
            "inflation": {"type": "noul", "noul": 0.1}
        },
        "usage": {"input_tokens": 1000, "output_tokens": 90}
    })
}

/// 记录峰值并发与总请求数的 responder。
#[derive(Clone)]
struct Counting {
    total: Arc<AtomicUsize>,
    active: Arc<AtomicUsize>,
    max_active: Arc<AtomicUsize>,
    delay_ms: u64,
}

impl Respond for Counting {
    fn respond(&self, _req: &wiremock::Request) -> ResponseTemplate {
        self.total.fetch_add(1, Ordering::SeqCst);
        let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.max_active.fetch_max(active, Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(self.delay_ms));
        self.active.fetch_sub(1, Ordering::SeqCst);
        ResponseTemplate::new(200).set_body_json(ok_body())
    }
}

fn collect_events(rx: async_channel::Receiver<PipelineEvent>) -> Vec<PipelineEvent> {
    let mut out = Vec::new();
    loop {
        match rx.recv_blocking() {
            Ok(event) => {
                let finished = matches!(event, PipelineEvent::BatchFinished { .. });
                out.push(event);
                if finished {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    out
}

/// Pipeline 在普通线程内创建/运行/销毁 —— tokio Runtime 不能在 async 上下文 drop。
fn spawn_batch(
    config: PipelineConfig,
    files: Vec<std::path::PathBuf>,
    criteria: Criteria,
) -> (async_channel::Receiver<PipelineEvent>, tokio_util::sync::CancellationToken) {
    let (tx, rx) = async_channel::bounded(256);
    let (token_tx, token_rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let pipeline = Pipeline::new(config).unwrap();
        let _ = token_tx.send(pipeline.token_handle());
        pipeline.run_batch(files, criteria, tx);
    });
    (rx, token_rx.recv().unwrap())
}

#[tokio::test(flavor = "multi_thread")]
async fn batch_cache_concurrency_and_cancel_scenarios() {
    // 缓存根目录重定向到临时目录 —— 进程级 env, 故三个场景合并为一个测试顺序执行
    let data_dir = tempfile::tempdir().unwrap();
    std::env::set_var("JEV_RESUME_DATA_DIR", data_dir.path());

    let server = MockServer::start().await;
    let counting = Counting {
        total: Arc::new(AtomicUsize::new(0)),
        active: Arc::new(AtomicUsize::new(0)),
        max_active: Arc::new(AtomicUsize::new(0)),
        delay_ms: 0,
    };
    Mock::given(method("POST"))
        .and(path("/v1/systemone"))
        .respond_with(counting.clone())
        .mount(&server)
        .await;

    let criteria = Criteria::default_toml();
    let dir = tempfile::tempdir().unwrap();

    // ── 场景1: 5 文件首批全 Fresh ──
    let files: Vec<_> = (0..5)
        .map(|i| write_resume(dir.path(), &format!("简历{i}.txt"), &format!("候选人{i} 五年后端经验 Rust")))
        .collect();
    let (rx, _token) = spawn_batch(
        PipelineConfig {
            concurrency: 4,
            api_key: "test-key".into(),
            base_url: server.uri(),
            model: "jev-latest".into(),
        },
        files.clone(),
        criteria.clone(),
    );
    let finished = finished_of(collect_events(rx));
    assert_eq!(finished, (5, 0, 0, false));
    assert_eq!(counting.total.load(Ordering::SeqCst), 5);

    // ── 场景2: 重跑全缓存, 零 HTTP ──
    let total_before = counting.total.load(Ordering::SeqCst);
    let (rx, _token) = spawn_batch(
        PipelineConfig {
            concurrency: 4,
            api_key: "test-key".into(),
            base_url: server.uri(),
            model: "jev-latest".into(),
        },
        files,
        criteria.clone(),
    );
    let finished = finished_of(collect_events(rx));
    assert_eq!(finished, (5, 0, 5, false));
    assert_eq!(counting.total.load(Ordering::SeqCst), total_before);
    let cache_dir = cache::cache_dir_for(&cache::cache_root().unwrap(), criteria.version());
    assert_eq!(std::fs::read_dir(&cache_dir).unwrap().count(), 5);

    // ── 场景3: 8 文件并发 ≤ 3 ──
    server.reset().await;
    let counting_slow = Counting {
        total: Arc::new(AtomicUsize::new(0)),
        active: Arc::new(AtomicUsize::new(0)),
        max_active: Arc::new(AtomicUsize::new(0)),
        delay_ms: 120,
    };
    Mock::given(method("POST"))
        .and(path("/v1/systemone"))
        .respond_with(counting_slow.clone())
        .mount(&server)
        .await;
    let files: Vec<_> = (0..8)
        .map(|i| write_resume(dir.path(), &format!("并发{i}.txt"), &format!("并发内容{i} 前端 Vue")))
        .collect();
    let (rx, _token) = spawn_batch(
        PipelineConfig {
            concurrency: 3,
            api_key: "test-key".into(),
            base_url: server.uri(),
            model: "jev-latest".into(),
        },
        files,
        criteria.clone(),
    );
    let _ = collect_events(rx);
    assert_eq!(counting_slow.total.load(Ordering::SeqCst), 8);
    let max = counting_slow.max_active.load(Ordering::SeqCst);
    assert!(max <= 3, "峰值并发 {max} 超过上限 3");

    // ── 场景4: 中途取消 ──
    server.reset().await;
    let counting_delayed = Counting {
        total: Arc::new(AtomicUsize::new(0)),
        active: Arc::new(AtomicUsize::new(0)),
        max_active: Arc::new(AtomicUsize::new(0)),
        delay_ms: 300,
    };
    Mock::given(method("POST"))
        .and(path("/v1/systemone"))
        .respond_with(counting_delayed.clone())
        .mount(&server)
        .await;
    let files: Vec<_> = (0..10)
        .map(|i| write_resume(dir.path(), &format!("取消{i}.txt"), &format!("取消内容{i} 算法 深度学习")))
        .collect();
    let (rx, token) = spawn_batch(
        PipelineConfig {
            concurrency: 2,
            api_key: "test-key".into(),
            base_url: server.uri(),
            model: "jev-latest".into(),
        },
        files,
        criteria,
    );
    loop {
        match rx.try_recv() {
            Ok(PipelineEvent::FileUpdated(_)) => {
                token.cancel();
                break;
            }
            Ok(_) => continue,
            Err(_) => std::thread::sleep(Duration::from_millis(10)),
        }
    }
    let mut cancelled = false;
    while let Ok(event) = rx.recv_blocking() {
        if let PipelineEvent::BatchFinished { cancelled: c, .. } = event {
            cancelled = c;
            break;
        }
    }
    assert!(cancelled, "批次应以 cancelled=true 结束");
}

fn finished_of(events: Vec<PipelineEvent>) -> (usize, usize, usize, bool) {
    events
        .iter()
        .find_map(|e| match e {
            PipelineEvent::BatchFinished {
                done,
                failed,
                cached,
                cancelled,
            } => Some((*done, *failed, *cached, *cancelled)),
            _ => None,
        })
        .expect("缺少 BatchFinished 事件")
}
