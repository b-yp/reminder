//! 行内「更多」菜单：低频动作与破坏性动作都收进来，底座是 `nsh/base` 的
//! `menu`（定位、Escape / 点外关闭、键盘与菜单项角色都由它给）。

use nsh_base::{menu, menu_item, theme};
use nsh_core::prelude::*;

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
                .rounded(px(6.0))
                .border_1()
                .border_color(move || theme().border)
                .bg(move || theme().popover)
                .shadow_lg()
        })
        .trigger(
            view()
                .label("更多操作")
                .w(px(22.0))
                .h(px(24.0))
                .rounded(px(5.0))
                .flex_row()
                .items_center()
                .justify_center()
                .bg(move || theme().secondary)
                .border_1()
                .border_color(move || theme().border)
                .hover(move |s| s.bg(move || theme().muted))
                .active(|s| s.scale(0.96).opacity(0.9))
                .focus_visible(|s| s.outline(px(2.0)).outline_color(move || theme().ring))
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
        .rounded(px(4.0))
        .flex_row()
        .items_center()
        .gap(px(4.0))
        .hover(move |s| s.bg(move || theme().muted))
        .active(move |s| s.bg(move || theme().muted))
        .child(text(label).text_size(px(11.0)).text_color(move || {
            if destructive {
                theme().destructive
            } else {
                theme().popover_foreground
            }
        }))
}
