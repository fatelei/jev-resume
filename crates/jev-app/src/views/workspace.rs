//! 主工作区：工具栏 + 结果表 + 状态栏。

use std::path::PathBuf;

use gpui::{
    div, prelude::FluentBuilder as _, px, AppContext as _, Context, Entity,
    InteractiveElement as _, IntoElement, ParentElement as _, Styled as _, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants},
    ActiveTheme as _, Sizable as _,
    table::{TableState},
};

use crate::bridge;
use crate::store::ResumeStore;

use super::results_table::{create_table_state, ResultsTableDelegate};

pub struct WorkspaceView {
    store: Entity<ResumeStore>,
    table: Entity<TableState<ResultsTableDelegate>>,
    criteria: jev_core::criteria::Criteria,
    app_config: jev_core::config::AppConfig,
}

impl WorkspaceView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let app_config = jev_core::config::load().unwrap_or_default();
        let criteria_path =
            jev_core::config::ensure_criteria_file(jev_core::criteria::DEFAULT_CRITERIA_TOML)
                .ok()
                .flatten();
        let criteria = criteria_path
            .as_deref()
            .and_then(|p| jev_core::criteria::Criteria::load(p).ok())
            .unwrap_or_else(jev_core::criteria::Criteria::default_toml);

        let store = cx.new(|_| ResumeStore::default());
        let table = create_table_state(store.clone(), window, cx);

        cx.observe(&store, |_, _, cx| cx.notify()).detach();
        Self {
            store,
            table,
            criteria,
            app_config,
        }
    }

    /// 导入文件夹并启动判定批次。
    fn import_folder(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let picker = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: true,
            multiple: true,
            prompt: Some("选择简历文件或文件夹".into()),
        });

        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(selected))) = picker.await else {
                return;
            };
            let files = jev_core::extract::collect_resume_files(&selected).unwrap_or_default();
            if files.is_empty() {
                return;
            }
            let _ = this.update(cx, |this, cx| {
                this.store.update(cx, |store, cx| {
                    store.add_files(files.clone());
                    cx.notify();
                });
                this.launch_batch(files, cx);
            });
        })
        .detach();
    }

    pub fn apply_pipeline_event(&mut self, event: &jev_core::pipeline::PipelineEvent, cx: &mut Context<Self>) {
        self.store.update(cx, |store, cx| {
            store.apply_event(event);
            cx.notify();
        });
        cx.notify();
    }

    fn launch_batch(&mut self, files: Vec<PathBuf>, cx: &mut Context<Self>) {
        if self.app_config.api_key.trim().is_empty() {
            eprintln!(
                "[jev] 未配置 API Key, 请编辑 {:?}",
                jev_core::config::config_path()
            );
        }
        let config = jev_core::pipeline::PipelineConfig {
            concurrency: self.app_config.concurrency.max(1) as usize,
            api_key: self.app_config.api_key.clone(),
            base_url: self.app_config.base_url.clone(),
            model: self.app_config.model.clone(),
        };
        bridge::start_batch(
            cx.entity().downgrade(),
            files,
            self.criteria.clone(),
            config,
            cx,
        );
    }

    fn render_toolbar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(gpui::hsla(0., 0., 0.85, 1.))
            .child(
                Button::new("import-folder")
                    .label("导入文件夹")
                    .small()
                    .primary()
                    .on_click(cx.listener(|this, _event, window, cx| {
                        this.import_folder(window, cx);
                    })),
            )
            .child(
                div()
                    .flex_1()
                    .text_size(px(13.))
                    .text_color(cx.theme().muted_foreground)
                    .child("支持 PDF / DOCX / TXT (单文件或整文件夹), 结果本地缓存"),
            )
    }

    fn render_status_bar(&self, cx: &Context<Self>) -> impl IntoElement {
        div()
            .px_3()
            .py_1()
            .text_size(px(12.))
            .text_color(cx.theme().muted_foreground)
            .child(self.store.read(cx).status_summary())
    }
}

impl gpui::Render for WorkspaceView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let import = self.render_toolbar(cx);
        let status = self.render_status_bar(cx);
        let empty = self.store.read(cx).visible_len() == 0;

        div()
            .id("workspace")
            .size_full()
            .flex()
            .flex_col()
            .child(import)
            .when(empty, |this| {
                this.child(
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_color(cx.theme().muted_foreground)
                        .child("点击「导入」选择简历文件或文件夹 (支持 PDF / DOCX / TXT, 可多选)"),
                )
            })
            .when(!empty, |this| {
                this.child(
                    div()
                        .flex_1()
                        .overflow_hidden()
                        .child(gpui_component::table::DataTable::new(&self.table)),
                )
            })
            .child(status)
    }
}
