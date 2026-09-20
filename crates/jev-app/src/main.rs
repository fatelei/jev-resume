//! jev-resume 桌面应用入口。

mod bridge;
mod store;
mod views;

use gpui::{
    App, AppContext as _, TitlebarOptions, Window, WindowBounds, WindowOptions, px, size,
};
use gpui_component::{Root, Theme, ThemeMode};
use gpui_platform::application;

use views::workspace::WorkspaceView;

const APP_FONT: &[u8] = include_bytes!("../assets/fonts/NotoSansSC-Regular.otf");

fn main() {
    application().run(|cx: &mut App| {
        gpui_component::init(cx);
        // 明确浅色主题: 深色模式下组件白字与白色窗口底叠加会全部隐形
        Theme::change(ThemeMode::Light, None, cx);
        // gpui-pre 默认字体族在新系统上可能解析失败, 显式注册并指定内置字体
        cx.text_system()
            .add_fonts(vec![std::borrow::Cow::Borrowed(APP_FONT)])
            .expect("注册内置字体失败");
        Theme::global_mut(cx).font_family = "Noto Sans SC".into();

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
