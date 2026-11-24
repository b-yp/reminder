//! 抽屉里的折叠区标题：整行都是触发区（底座 `nsh/base` 的 `button`），
//! 右侧「展开 / 收起」文字与箭头跟着状态走。

use nsh_base::{button, theme};
use nsh_core::prelude::*;

use super::focus::focus_ring;
use super::tokens::{font, motion, radius};
use crate::icons;
use crate::theme_mode::{ACCENT_GOLD, ACCENT_GOLD_STRONG};

/// 折叠区标题行。`detail` 是标题后的补充（如「(6项预设)」），可为空串。
pub fn disclosure_row(
    title: &'static str,
    icon_markup: &'static str,
    detail: String,
    expanded: impl Fn() -> bool + Copy + 'static,
    collapsed_label: &'static str,
    expanded_label: &'static str,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    button()
        .accessibility_label(title)
        .w_full()
        .px(px(8.0))
        .py(px(4.0))
        .rounded(px(radius::LG))
        .flex_row()
        .items_center()
        .justify_between()
        .bg(move || theme().secondary)
        .border_1()
        .border_color(move || {
            if expanded() {
                ACCENT_GOLD
            } else {
                theme().border
            }
        })
        .hover(move |s| s.bg(move || theme().muted))
        .active(|s| s.opacity(0.9))
        .focus_visible(focus_ring)
        .transition_colors()
        .transition_transform()
        .duration(move || if reduce_motion() { 0.0_f32 } else { motion::FAST })
        .on_press(move || on_press())
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(5.0))
                .child(icons::icon_svg(icon_markup, 11.5, ACCENT_GOLD_STRONG))
                .child(text(title).text_size(px(font::BODY)).text_color(ACCENT_GOLD_STRONG))
                .child(
                    text(detail)
                        .text_size(px(font::CAPTION))
                        .text_color(move || theme().muted_foreground),
                ),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(3.0))
                .child(
                    text(if expanded() {
                        expanded_label
                    } else {
                        collapsed_label
                    })
                    .text_size(px(font::CAPTION))
                    .text_color(move || theme().muted_foreground),
                )
                .child(icons::dynamic_icon_svg(
                    move || {
                        if expanded() {
                            icons::ICON_CHEVRON_UP
                        } else {
                            icons::ICON_CHEVRON_DOWN
                        }
                    },
                    11.0,
                    move || theme().muted_foreground,
                )),
        )
}
