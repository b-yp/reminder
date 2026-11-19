//! 行内「更多」菜单：低频动作与破坏性动作都收进来，底座是 `nsh/base` 的
//! `menu`（定位、Escape / 点外关闭、键盘与菜单项角色都由它给）。

use nsh_base::{button, menu, menu_item, theme};
use nsh_core::prelude::*;

use super::tokens::{font, radius, size};
use crate::icons;

/// 提醒行右侧的「⋯」菜单：延后 5 分钟、删除。
///
/// 删除只负责把意图交回页面（`on_delete`），触发底部带撤销倒计时的 Toast 浮层。
pub fn row_menu(
    snooze_disabled: impl Fn() -> bool + Copy + 'static,
    on_snooze: impl Fn() + 'static,
    on_delete: impl Fn() + 'static,
) -> impl IntoElement {
    menu()
        .popup_style(|s| {
            s.min_w(px(132.0))
                .p(px(3.0))
                .rounded(px(radius::LG))
                .border_1()
                .border_color(move || theme().border)
                .bg(move || theme().popover)
                .shadow_lg()
        })
        .trigger(
            button()
                .accessibility_label("更多操作")
                .w(px(22.0))
                .h(px(size::CONTROL_MD))
                .rounded(px(radius::LG))
                .flex_row()
                .items_center()
                .justify_center()
                .bg(move || theme().secondary)
                .border_1()
                .border_color(move || theme().border)
                .hover(move |s| s.bg(move || theme().muted))
                .active(|s| s.scale(0.96).opacity(0.9))
                .focus_visible(|s| s.outline(px(size::FOCUS_RING)).outline_color(move || theme().ring))
                .transition_colors()
                .child(icons::icon_svg(icons::ICON_MORE, 12.0, move || {
                    theme().muted_foreground
                })),
        )
        .child(item("延后 5 分钟", false, snooze_disabled, move || {
            on_snooze()
        }))
        .child(item("删除提醒", true, || false, move || on_delete()))
}

/// 一个菜单项：`destructive` 决定文字色，`disabled` 跟随状态。
fn item(
    label: &'static str,
    destructive: bool,
    disabled: impl Fn() -> bool + Copy + 'static,
    on_select: impl Fn() + 'static,
) -> impl IntoElement {
    menu_item()
        .disabled(disabled)
        .on_select(move || on_select())
        .px(px(8.0))
        .py(px(4.0))
        .rounded(px(radius::MD))
        .flex_row()
        .items_center()
        .gap(px(4.0))
        .hover(move |s| s.bg(move || theme().muted))
        .active(move |s| s.bg(move || theme().muted))
        .child(text(label).text_size(px(font::BODY)).text_color(move || {
            if destructive {
                theme().destructive
            } else {
                theme().popover_foreground
            }
        }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nsh_core::platform::{Idiom, Os};
    use nsh_engine::{EngineConfig, Points, Root, Size};
    use nsh_testing::Headless;

    #[component]
    fn MenuHost() -> impl IntoElement {
        view()
            .w(px(320.0))
            .h(px(200.0))
            .child(row_menu(|| false, || {}, || {}))
    }

    #[test]
    fn test_menu_keyboard_tab_open_escape_focus_restore() {
        let mut config = EngineConfig {
            viewport: Size::new(Points(320.0), Points(200.0)),
            ..EngineConfig::new(Root::of::<MenuHost>())
        };
        config.environment.os = Os::MacOs;
        config.environment.idiom = Idiom::Desktop;
        let mut app = Headless::start(config).expect("Headless started");
        app.wait_for("更多操作").unwrap();

        // 1. Tab 聚焦到 ⋯ 触发器按钮
        app.press_key("tab").unwrap();

        // 2. Enter 键展开菜单
        app.press_key("enter").unwrap();
        app.wait_for("延后 5 分钟").unwrap();
        assert!(app.screen().shows("延后 5 分钟"));
        assert!(app.screen().shows("删除提醒"));

        // 3. Escape 键关闭菜单
        app.press_key("escape").unwrap();
        app.wait_for_gone("延后 5 分钟", std::time::Duration::from_secs(2))
            .unwrap();
        assert!(!app.screen().shows("延后 5 分钟"));

        // 4. 验证焦点已回到 ⋯ 触发器：再次按 Enter 能重新展开菜单
        app.press_key("enter").unwrap();
        app.wait_for("延后 5 分钟").unwrap();
        assert!(app.screen().shows("延后 5 分钟"));

        // 5. 再次 Escape 恢复关闭状态
        app.press_key("escape").unwrap();
        app.wait_for_gone("延后 5 分钟", std::time::Duration::from_secs(2))
            .unwrap();
    }
}
