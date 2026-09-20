//! ResumeStore：表格数据源实体，消费流水线事件并维护行状态。

use std::path::PathBuf;

use gpui::App;
use jev_core::pipeline::{FileJudgment, FileState, PipelineEvent};

#[derive(Debug, Clone)]
pub struct ResumeRow {
    pub path: PathBuf,
    pub file_name: String,
    pub state: FileState,
    pub judgment: Option<FileJudgment>,
    pub error: Option<String>,
    pub elapsed_ms: Option<u64>,
    pub cached: bool,
}

impl ResumeRow {
    pub fn status_text(&self) -> &'static str {
        match self.state {
            FileState::Pending => "待判定",
            FileState::Extracting => "提取中",
            FileState::Queued => "排队中",
            FileState::Judging => "判定中",
            FileState::Done => "完成",
            FileState::Failed => "失败",
        }
    }
}

#[derive(Debug, Default)]
pub struct ResumeStore {
    rows: Vec<ResumeRow>,
    /// 可见行索引（筛选后），供表格 rows_count / cell_text 对齐。
    visible: Vec<usize>,
    total: usize,
    done: usize,
    failed: usize,
    cached: usize,
    running: bool,
}

impl ResumeStore {
    pub fn add_files(&mut self, files: Vec<PathBuf>) {
        for path in files {
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("未知文件")
                .to_string();
            self.rows.push(ResumeRow {
                path: path.clone(),
                file_name,
                state: FileState::Pending,
                judgment: None,
                error: None,
                elapsed_ms: None,
                cached: false,
            });
        }
        self.refilter();
    }

    pub fn apply_event(&mut self, event: &PipelineEvent) {
        match event {
            PipelineEvent::BatchStarted { total } => {
                self.total = *total;
                self.done = 0;
                self.failed = 0;
                self.cached = 0;
                self.running = true;
            }
            PipelineEvent::FileUpdated(update) => {
                if let Some(row) = self
                    .rows
                    .iter_mut()
                    .find(|r| r.path == update.path)
                {
                    row.state = update.state.clone();
                    row.judgment = update.judgment.clone();
                    row.error = update.error.clone();
                    row.elapsed_ms = update.elapsed_ms;
                    row.cached = update.cached;
                }
            }
            PipelineEvent::BatchFinished {
                done,
                failed,
                cached,
                cancelled,
            } => {
                self.done = *done;
                self.failed = *failed;
                self.cached = *cached;
                self.running = false;
                let _ = cancelled;
            }
            PipelineEvent::LogLine(_) => {}
        }
        self.refilter();
    }

    fn refilter(&mut self) {
        self.visible = (0..self.rows.len()).collect();
    }

    pub fn visible_row(&self, ix: usize) -> Option<&ResumeRow> {
        self.visible.get(ix).and_then(|&i| self.rows.get(i))
    }

    pub fn visible_len(&self) -> usize {
        self.visible.len()
    }

    pub fn status_summary(&self) -> String {
        if self.running {
            format!(
                "{} 个文件判定中 · 已完成 {} · 失败 {} · 缓存 {}",
                self.total, self.done, self.failed, self.cached
            )
        } else if self.rows.is_empty() {
            "拖入简历文件或文件夹开始".into()
        } else {
            format!(
                "共 {} 个文件 · 完成 {} · 失败 {} · 缓存 {}",
                self.rows.len(),
                self.rows
                    .iter()
                    .filter(|r| r.state == FileState::Done)
                    .count(),
                self.rows.iter().filter(|r| r.state == FileState::Failed).count(),
                self.rows.iter().filter(|r| r.cached).count()
            )
        }
    }

    /// 同步状态辅助（测试用）。
    #[cfg(test)]
    pub fn row_states(&self) -> Vec<FileState> {
        self.rows.iter().map(|r| r.state.clone()).collect()
    }

    #[allow(dead_code)]
    fn touch(_cx: &App) {}
}
