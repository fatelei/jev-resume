//! Tauri 命令层：UI ↔ jev-core 的薄粘合。
//!
//! 批次在独立 std::thread 运行（Pipeline 独占 tokio Runtime，
//! 不能在 async 上下文 drop）；事件经 async_channel 泵给 webview。

use std::path::PathBuf;
use std::sync::atomic::Ordering;

use jev_core::criteria::Criteria;
use jev_core::export::ExportRow;
use jev_core::pipeline::{Pipeline, PipelineConfig, PipelineEvent};
use serde::Serialize;
use tauri::{AppHandle, Emitter as _, State};

use crate::PipelineState;

#[derive(Serialize)]
pub struct Meta {
    /// [key, label] × 5，资历 key → 中文标签
    pub seniority_labels: Vec<[String; 2]>,
    pub config_path: Option<String>,
    pub has_api_key: bool,
}

#[tauri::command(async)]
pub fn get_meta() -> Meta {
    Meta {
        seniority_labels: jev_core::questions::SENIORITY_LABELS
            .iter()
            .map(|(k, l)| [(*k).to_string(), (*l).to_string()])
            .collect(),
        config_path: jev_core::config::config_path().map(|p| p.to_string_lossy().into_owned()),
        has_api_key: jev_core::config::load()
            .map(|c| !c.api_key.trim().is_empty())
            .unwrap_or(false),
    }
}

/// 读取应用配置（api_key 等），供设置页回填。
#[tauri::command(async)]
pub fn get_config() -> Result<jev_core::config::AppConfig, String> {
    jev_core::config::load().map_err(|e| e.user_message())
}

/// 保存应用配置（unix 0600 原子写）。保存后下一批次即生效。
#[tauri::command(async)]
pub fn save_config(config: jev_core::config::AppConfig) -> Result<(), String> {
    jev_core::config::save(&config).map_err(|e| e.user_message())
}

/// 拖拽/文件夹路径 → 展开为简历文件列表（walkdir，pdf/docx/txt）。
#[tauri::command(async)]
pub fn collect_files(paths: Vec<String>) -> Result<Vec<String>, String> {
    let p: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
    jev_core::extract::collect_resume_files(&p)
        .map(|v| v.iter().map(|f| f.to_string_lossy().into_owned()).collect())
        .map_err(|e| e.user_message())
}

/// 启动一个判定批次：读当前 config/criteria，起线程跑流水线，
/// 事件经 "pipeline-event" 发给前端。running 时拒绝重入。
#[tauri::command(async)]
pub fn start_batch(
    app: AppHandle,
    state: State<'_, PipelineState>,
    files: Vec<String>,
) -> Result<(), String> {
    if state.running.load(Ordering::SeqCst) {
        return Err("已有批次进行中".into());
    }

    // 每批次现读 config/criteria —— 改配置文件无需重启
    let cfg = jev_core::config::load().map_err(|e| e.user_message())?;
    if cfg.api_key.trim().is_empty() {
        let _ = app.emit(
            "pipeline-event",
            &PipelineEvent::LogLine(format!(
                "未配置 API Key, 请编辑 {:?}",
                jev_core::config::config_path()
            )),
        );
    }
    let criteria_path =
        jev_core::config::ensure_criteria_file(jev_core::criteria::DEFAULT_CRITERIA_TOML)
            .map_err(|e| e.user_message())?;
    let criteria = criteria_path
        .and_then(|p| Criteria::load(&p).ok())
        .unwrap_or_else(Criteria::default_toml);

    let config = PipelineConfig {
        concurrency: cfg.concurrency.max(1) as usize,
        api_key: cfg.api_key,
        base_url: cfg.base_url,
        model: cfg.model,
    };

    let paths: Vec<PathBuf> = files.iter().map(PathBuf::from).collect();
    let (tx, rx) = async_channel::bounded::<PipelineEvent>(256);
    let token_slot = state.token.clone();
    let running = state.running.clone();
    running.store(true, Ordering::SeqCst);

    // 流水线线程：Pipeline 的 Runtime 随它在本线程 drop
    std::thread::spawn(move || {
        let pipeline = Pipeline::new(config);
        match pipeline {
            Ok(pipeline) => {
                *token_slot.lock().expect("token lock") = Some(pipeline.token_handle());
                pipeline.run_batch(paths, criteria, tx);
                *token_slot.lock().expect("token lock") = None;
            }
            Err(e) => {
                let _ = tx.send_blocking(PipelineEvent::LogLine(e.user_message()));
                let _ = tx.send_blocking(PipelineEvent::BatchFinished {
                    done: 0,
                    failed: paths.len(),
                    cached: 0,
                    cancelled: false,
                });
            }
        }
        running.store(false, Ordering::SeqCst);
    });

    // 事件泵：async runtime 上转发给 webview
    tauri::async_runtime::spawn(async move {
        while let Ok(event) = rx.recv().await {
            if app.emit("pipeline-event", &event).is_err() {
                break; // 应用已退出
            }
        }
    });
    Ok(())
}

#[tauri::command(async)]
pub fn cancel_batch(state: State<'_, PipelineState>) {
    if let Some(token) = state.token.lock().expect("token lock").as_ref() {
        token.cancel();
    }
}

#[tauri::command(async)]
pub fn export_results(
    rows: Vec<ExportRow>,
    path: String,
    format: String,
) -> Result<String, String> {
    let path = PathBuf::from(&path);
    let result = match format.as_str() {
        "csv" => jev_core::export::export_csv(&rows, &path),
        "json" => jev_core::export::export_json(&rows, &path),
        other => return Err(format!("未知导出格式: {other}")),
    };
    result.map_err(|e| e.user_message())?;
    Ok(path.to_string_lossy().into_owned())
}
