//! 结果表格：TableDelegate 实现，数据来自 ResumeStore。

use gpui::{div, App, AppContext as _, Context, IntoElement, ParentElement as _, Window};
use gpui_component::table::{Column, TableDelegate, TableState};

use crate::store::ResumeStore;

pub struct ResultsTableDelegate {
    store: gpui::Entity<ResumeStore>,
}

impl ResultsTableDelegate {
    pub fn new(store: gpui::Entity<ResumeStore>) -> Self {
        Self { store }
    }
}

const COLUMN_KEYS: [(&str, &str, f32); 9] = [
    ("file", "文件名", 240.),
    ("status", "状态", 72.),
    ("category", "岗位分类", 110.),
    ("seniority", "资历级别", 110.),
    ("strength", "强度", 64.),
    ("inflation", "注水", 64.),
    ("elapsed", "耗时", 76.),
    ("cached", "缓存", 56.),
    ("note", "备注", 240.),
];

impl TableDelegate for ResultsTableDelegate {
    fn columns_count(&self, _cx: &App) -> usize {
        COLUMN_KEYS.len()
    }

    fn rows_count(&self, cx: &App) -> usize {
        self.store.read(cx).visible_len()
    }

    fn column(&self, col_ix: usize, _cx: &App) -> Column {
        let (key, name, width) = COLUMN_KEYS[col_ix];
        Column::new(key, name).width(gpui::px(width))
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let text = self.cell_text(row_ix, col_ix, cx);
        div().child(text)
    }

    fn cell_text(&self, row_ix: usize, col_ix: usize, cx: &App) -> String {
        let Some(row) = self.store.read(cx).visible_row(row_ix) else {
            return String::new();
        };
        let Some((key, _, _)) = COLUMN_KEYS.get(col_ix) else {
            return String::new();
        };
        let strength = row
            .judgment
            .as_ref()
            .and_then(|j| j.strength)
            .map(|v| format!("{v:.1}"))
            .unwrap_or_else(|| "-".into());
        let inflation = row
            .judgment
            .as_ref()
            .and_then(|j| j.inflation)
            .map(|v| format!("{v:.2}"))
            .unwrap_or_else(|| "-".into());
        let elapsed = row
            .elapsed_ms
            .map(|v| format!("{v}ms"))
            .unwrap_or_else(|| "-".into());
        match *key {
            "file" => row.file_name.clone(),
            "status" => row.status_text().into(),
            "category" => row
                .judgment
                .as_ref()
                .and_then(|j| j.role_category.clone())
                .unwrap_or_else(|| "-".into()),
            "seniority" => row
                .judgment
                .as_ref()
                .and_then(|j| j.seniority.clone())
                .map(|s| jev_core::questions::seniority_label(&s))
                .unwrap_or_else(|| "-".into()),
            "strength" => strength,
            "inflation" => inflation,
            "elapsed" => elapsed,
            "cached" => {
                if row.cached {
                    "是".into()
                } else {
                    "-".into()
                }
            }
            "note" => row.error.clone().unwrap_or_default(),
            _ => String::new(),
        }
    }
}

/// 构建 TableState 实体。
pub fn create_table_state(
    store: gpui::Entity<ResumeStore>,
    window: &mut Window,
    cx: &mut Context<crate::views::workspace::WorkspaceView>,
) -> gpui::Entity<TableState<ResultsTableDelegate>> {
    let delegate = ResultsTableDelegate::new(store);
    cx.new(|cx| TableState::new(delegate, window, cx))
}
