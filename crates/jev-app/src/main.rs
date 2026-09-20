//! jev-resume 桌面应用入口 (Tauri 2)。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use std::sync::{atomic::AtomicBool, Arc, Mutex};

use tokio_util::sync::CancellationToken;

/// 批次运行状态：取消句柄 + 重入保护（两字段都要 clone 进流水线线程）。
#[derive(Default)]
pub struct PipelineState {
    pub token: Arc<Mutex<Option<CancellationToken>>>,
    pub running: Arc<AtomicBool>,
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(PipelineState::default())
        .invoke_handler(tauri::generate_handler![
            commands::get_meta,
            commands::collect_files,
            commands::start_batch,
            commands::cancel_batch,
            commands::export_results,
        ])
        .run(tauri::generate_context!())
        .expect("tauri 应用运行失败");
}
