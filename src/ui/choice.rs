//! 选择类控件：成组互斥/多选的胶囊与图标块。
//!
//! 底座是 `nsh/base` 的 `toggle_group` + `toggle`：整组只占一个 Tab 停靠点，
//! 方向键在组内移动，Space / Enter 切换，读屏通过 `aria-pressed` 读出按下态。

use nsh_base::{theme, toggle, toggle_group};
use nsh_core::prelude::*;

use super::tokens::{font, radius, size};
use crate::icons;
use crate::theme_mode::{ACCENT_GOLD, ACCENT_GOLD_BG, ACCENT_GOLD_STRONG};

/// 一组互斥或独立的选择项（工具栏语义）。整组只占一个 Tab 停靠点。
pub fn choice_group<I>(items: I) -> impl IntoElement
where
    I: IntoIterator,
    I::Item: IntoElement,
{
    let mut group = toggle_group().flex_row().items_center().gap(px(3.0));
    for item in items {
        group = group.child(item);
    }
    group
}

/// 多行铺开的选择组（图标主题的两排）：整组仍是一个键盘工具栏。
pub fn choice_grid<I>(items: I) -> impl IntoElement
where
    I: IntoIterator,
    I::Item: IntoElement,
{
    let mut group = toggle_group().w_full().flex_col().gap(px(3.0));
    for item in items {
        group = group.child(item);
    }
    group
}

/// 小号文字胶囊：快捷填入、常用时刻、提醒方式。
pub fn choice_chip(
    label: &'static str,
    pressed: impl Fn() -> bool + Copy + 'static,
    on_change: impl Fn(bool) + 'static,
) -> impl IntoElement {
    toggle()
        .pressed(pressed)
        .accessibility_label(label)
        .on_change(move |next: &bool| on_change(*next))
        .flex_row()
        .items_center()
        .justify_center()
        .px(px(6.0))
        .py(px(2.0))
        .rounded(px(radius::SM))
        .bg(move || {
            if pressed() {
                ACCENT_GOLD_BG
            } else {
                theme().secondary
            }
        })
        .border_1()
        .border_color(move || {
            if pressed() {
                ACCENT_GOLD
            } else {
                theme().border
            }
        })
        .hover(move |s| {
            s.bg(move || {
                if pressed() {
                    ACCENT_GOLD_BG
                } else {
                    theme().muted
                }
            })
        })
        .active(|s| s.scale(0.96).opacity(0.9))
        .focus_visible(|s| s.outline(px(size::FOCUS_RING)).outline_color(move || theme().ring))
        .transition_colors()
        .child(text(label).text_size(px(font::CAPTION)).text_color(move || {
            if pressed() {
                ACCENT_GOLD_STRONG
            } else {
                theme().muted_foreground
            }
        }))
}

/// 外观主题的三选一：等分、24 高、图标 + 文字。
pub fn segment_toggle(
    label: &'static str,
    icon_markup: &'static str,
    pressed: impl Fn() -> bool + Copy + 'static,
    on_change: impl Fn(bool) + 'static,
) -> impl IntoElement {
    toggle()
        .pressed(pressed)
        .accessibility_label(label)
        .on_change(move |next: &bool| on_change(*next))
        .flex_1()
        .h(px(size::CONTROL_MD))
        .rounded(px(radius::LG))
        .flex_row()
        .items_center()
        .justify_center()
        .gap(px(4.0))
        .bg(move || {
            if pressed() {
                ACCENT_GOLD_BG
            } else {
                theme().secondary
            }
        })
        .border_1()
        .border_color(move || {
            if pressed() {
                ACCENT_GOLD
            } else {
                theme().border
            }
        })
        .hover(move |s| {
            s.bg(move || {
                if pressed() {
                    ACCENT_GOLD_BG
                } else {
                    theme().muted
                }
            })
        })
        .active(|s| s.scale(0.97).opacity(0.9))
        .focus_visible(|s| s.outline(px(size::FOCUS_RING)).outline_color(move || theme().ring))
        .transition_colors()
        .child(icons::icon_svg(icon_markup, 11.0, move || {
            if pressed() {
                ACCENT_GOLD_STRONG
            } else {
                theme().muted_foreground
            }
        }))
        .child(text(label).text_size(px(font::CAPTION)).text_color(move || {
            if pressed() {
                ACCENT_GOLD_STRONG
            } else {
                theme().muted_foreground
            }
        }))
}

/// 图标主题选择：24×24 的图标块，选中时用该图标自己的高光色。
pub fn icon_choice(
    icon_key: &'static str,
    label: &'static str,
    pressed: impl Fn() -> bool + Copy + 'static,
    on_change: impl Fn(bool) + 'static,
) -> impl IntoElement {
    toggle()
        .pressed(pressed)
        .accessibility_label(label)
        .on_change(move |next: &bool| on_change(*next))
        .w(px(size::CONTROL_MD))
        .h(px(size::CONTROL_MD))
        .rounded(px(radius::MD))
        .items_center()
        .justify_center()
        .bg(move || {
            if pressed() {
                icons::get_item_adaptive(icon_key).opacity(0.20)
            } else {
                theme().secondary
            }
        })
        .border_1()
        .border_color(move || {
            if pressed() {
                icons::get_item_adaptive(icon_key)
            } else {
                theme().border
            }
        })
        .hover(move |s| s.border_color(move || icons::get_item_adaptive(icon_key)))
        .active(|s| s.scale(0.94).opacity(0.9))
        .focus_visible(|s| s.outline(px(size::FOCUS_RING)).outline_color(move || theme().ring))
        .transition_colors()
        .child(icons::icon_svg(
            icons::get_icon_markup(icon_key),
            12.0,
            move || {
                if pressed() {
                    icons::get_item_adaptive(icon_key)
                } else {
                    theme().muted_foreground
                }
            },
        ))
}
