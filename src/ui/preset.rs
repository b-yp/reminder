//! 常用推荐卡片：一个已经填好时长的预设，按一下加入列表。

use nsh_base::{button, theme};
use nsh_core::prelude::*;

use super::focus::focus_ring;
use super::tokens::{font, motion, radius};
use crate::icons;
use crate::theme_mode::{ACCENT_DONE_BG, ACCENT_DONE_BORDER, ACCENT_DONE_FG};

/// 推荐预设卡片。`added` 为真时表示已经在列表里：语义上禁用（读屏会读出
/// 「不可用」），外观用「已加入」的绿色，而不是另一种可点样式。
pub fn preset_card(
    name: &'static str,
    badge: String,
    icon_key: &'static str,
    added: impl Fn() -> bool + Copy + 'static,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    button()
        .accessibility_label(name)
        .disabled(added)
        .flex_1()
        .px(px(7.0))
        .py(px(5.0))
        .rounded(px(radius::LG))
        .flex_row()
        .items_center()
        .justify_between()
        .bg(move || {
            if added() {
                ACCENT_DONE_BG
            } else {
                theme().secondary
            }
        })
        .border_1()
        .border_color(move || {
            if added() {
                ACCENT_DONE_BORDER
            } else {
                theme().border
            }
        })
        .hover(move |s| {
            s.bg(move || {
                if added() {
                    ACCENT_DONE_BG
                } else {
                    theme().muted
                }
            })
        })
        .active(|s| s.scale(0.98).opacity(0.9))
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
                .child(icons::icon_svg(
                    icons::get_icon_markup(icon_key),
                    12.0,
                    icons::get_item_adaptive(icon_key),
                ))
                .child(text(name).text_size(px(font::LABEL)).text_color(move || {
                    if added() {
                        ACCENT_DONE_FG
                    } else {
                        theme().foreground
                    }
                })),
        )
        .child(
            view()
                .px(px(4.0))
                .py(px(1.5))
                .rounded(px(radius::SM))
                .bg(move || {
                    if added() {
                        ACCENT_DONE_BORDER.opacity(0.35)
                    } else {
                        theme().muted
                    }
                })
                .child(text(badge).text_size(px(font::CAPTION)).text_color(move || {
                    if added() {
                        ACCENT_DONE_FG
                    } else {
                        theme().muted_foreground
                    }
                })),
        )
}
