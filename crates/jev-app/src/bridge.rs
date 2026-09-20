//! 桥接：在 gpui 前台执行器上泵取流水线事件，更新 Store。
//! Pipeline 在独立线程创建/运行/销毁（tokio Runtime 不能在 async 上下文 drop）。

use std::path::PathBuf;

use gpui::{AsyncApp, Context, WeakEntity};
use jev_core::criteria::Criteria;
use jev_core::pipeline::{Pipeline, PipelineConfig, PipelineEvent};

use crate::views::workspace::WorkspaceView;

/// 从 UI 线程启动一个批次，并在前台执行器上开始泵事件。
pub fn start_batch(
    _workspace: WeakEntity<WorkspaceView>,
    files: Vec<PathBuf>,
    criteria: Criteria,
    config: PipelineConfig,
    cx: &mut Context<WorkspaceView>,
) {
    let (tx, rx) = async_channel::bounded::<PipelineEvent>(256);
    std::thread::spawn(move || {
        let Ok(pipeline) = Pipeline::new(config) else {
            let _ = tx.send_blocking(PipelineEvent::LogLine("流水线初始化失败".into()));
            return;
        };
        pipeline.run_batch(files, criteria, tx);
        // pipeline 在此线程 drop —— Runtime 安全释放
    });

    cx.spawn(async move |this: WeakEntity<WorkspaceView>, cx: &mut AsyncApp| {
        while let Ok(event) = rx.recv().await {
            let cont = this
                .update(cx, |this, cx| {
                    this.apply_pipeline_event(&event, cx);
                })
                .is_ok();
            if !cont {
                break; // 应用退出
            }
        }
    })
    .detach();
}
