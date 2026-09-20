//! jev-resume 桌面应用入口。

mod bridge;
mod store;
mod views;

use gpui::{
    App, AppContext as _, TitlebarOptions, Window, WindowBounds, WindowOptions, px, size,
};
use gpui_component::Root;
use gpui_platform::application;

use views::workspace::WorkspaceView;

fn main() {
    application().run(|cx: &mut App| {
        gpui_component::init(cx);

        let bounds = gpui::Bounds::centered(None, size(px(1280.), px(820.)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("Jev 简历分类".into()),
                ..Default::default()
            }),
            ..Default::default()
        };

        cx.open_window(options, |window: &mut Window, cx: &mut App| {
            let view: gpui::Entity<WorkspaceView> = cx.new(|cx| WorkspaceView::new(window, cx));
            let any_view: gpui::AnyView = view.into();
            cx.new(|cx| Root::new(any_view, window, cx))

        })
        .expect("打开主窗口失败");

        cx.activate(true);
    });
}
