//! 按钮：应用里所有可按的东西都从这里出。
//!
//! 底座是 `nsh/base` 的 `button`：它自带按钮角色、Tab 可达、Enter / Space
//! 触发、禁用语义与 `focus-visible` 的键盘焦点；这里只补外观与状态色。
use crate::theme_mode::{ACCENT_GOLD, ACCENT_GOLD_BG, ACCENT_GOLD_FG};
use super::focus::focus_ring;
use super::tokens::{font, motion, radius, size};
use nsh_base::{button, theme};
use nsh_core::prelude::*;
/// 按钮语气：页面按语义挑，不写颜色。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionTone {
    /// 常规动作：`secondary` 底 + `border` 描边。
    Quiet,
    /// 当前该做的事：金黄浅底 + 金描边（打卡）。
    Accent,
}
/// 图标按钮的语气（只影响外观，不影响语义）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconTone {
    /// 静默的工具栏图标：没有底色和描边，悬停才浮出一层浅底（顶栏）。
    Ghost,
    /// 需要一块可见实底的图标按钮（迷你胶囊里的动作）。
    Quiet,
}

/// 顶栏与迷你胶囊里的方形图标按钮：24×24，圆角 6（扩充热区与防误触）。
///
/// `selected` 只表达「这个按钮对应的面板正开着」，颜色与语义都由这里给；
/// `tone` 决定静默还是实底。`disabled` 表达该动作当前是否不可用，
/// 禁用时呈现透明度淡化并阻止触发。
pub fn icon_button(
    label: &'static str,
    tone: IconTone,
    selected: impl Fn() -> bool + Copy + 'static,
    disabled: impl Fn() -> bool + Copy + 'static,
    on_press: impl Fn() + 'static,
    content: impl IntoElement,
) -> impl IntoElement {
    let solid = tone == IconTone::Quiet;
    button()
        .role(AccessibilityRole::Button)
        .accessibility_label(label)
        .selected(selected)
        .disabled(disabled)
        .w(px(size::CONTROL_MD))
        .h(px(size::CONTROL_MD))
        .rounded(px(radius::LG))
        .flex_row()
        .items_center()
        .justify_center()
        .bg(move || match (selected(), solid) {
            (true, _) => ACCENT_GOLD_BG,
            (false, true) => theme().secondary,
            (false, false) => AdaptiveColor::TRANSPARENT,
        })
        .border_1()
        .border_color(move || match (selected(), solid) {
            (true, _) => ACCENT_GOLD,
            (false, true) => theme().border,
            (false, false) => AdaptiveColor::TRANSPARENT,
        })
        .hover(move |s| {
            s.bg(move || {
                if selected() {
                    ACCENT_GOLD_BG
                } else {
                    theme().muted
                }
            })
        })
        .active(|s| s.scale(0.94).opacity(0.88))
        .focus_visible(focus_ring)
        .disabled_style(|s| s.opacity(0.35))
        .transition_transform()
        .transition_colors()
        .duration(move || if reduce_motion() { 0.0_f32 } else { motion::FAST })
        .on_press(move || on_press())
        .child(content)
}

/// 提醒行里的动作按钮：28×24（打卡、延期）或 22×24（删除）。
/// `tone` 是响应式的：同一颗按钮会随行状态（逾期 / 常态）换语气。
pub fn action_button(
    label: &'static str,
    width: f32,
    tone: impl Fn() -> ActionTone + Copy + 'static,
    disabled: impl Fn() -> bool + Copy + 'static,
    on_press: impl Fn() + 'static,
    content: impl IntoElement,
) -> impl IntoElement {
    button()
        .role(AccessibilityRole::Button)
        .accessibility_label(label)
        .disabled(disabled)
        .w(px(width))
        .h(px(size::CONTROL_MD))
        .rounded(px(radius::LG))
        .flex_row()
        .items_center()
        .justify_center()
        .bg(move || match tone() {
            ActionTone::Quiet => theme().secondary,
            ActionTone::Accent => ACCENT_GOLD_BG,
        })
        .border_1()
        .border_color(move || match tone() {
            ActionTone::Quiet => theme().border,
            ActionTone::Accent => ACCENT_GOLD,
        })
        .hover(move |s| match tone() {
            ActionTone::Quiet => s.bg(move || theme().muted),
            ActionTone::Accent => s.opacity(0.92),
        })
        .active(|s| s.scale(0.92).opacity(0.85))
        .focus_visible(focus_ring)
        .disabled_style(|s| s.opacity(0.35))
        .transition_transform()
        .transition_colors()
        .duration(move || if reduce_motion() { 0.0_f32 } else { motion::FAST })
        .on_press(move || on_press())
        .child(content)
}

/// 通用气泡提示：用于挂在控件的 `.tooltip(...)` 上
pub fn tooltip_bubble(label: &'static str) -> AnyElement {
    view()
        .px(px(8.0))
        .py(px(4.0))
        .rounded(px(radius::LG))
        .bg(move || theme().popover)
        .border_1()
        .border_color(move || theme().border)
        .shadow_sm()
        .child(
            text(label)
                .text_size(px(font::BODY))
                .text_color(move || theme().popover_foreground),
        )
        .into_element()
}

/// 响应式动态气泡提示
pub fn dynamic_tooltip_bubble(label: impl Fn() -> &'static str + 'static) -> AnyElement {
    view()
        .px(px(8.0))
        .py(px(4.0))
        .rounded(px(radius::LG))
        .bg(move || theme().popover)
        .border_1()
        .border_color(move || theme().border)
        .shadow_sm()
        .child(
            text(label)
                .text_size(px(font::BODY))
                .text_color(move || theme().popover_foreground),
        )
        .into_element()
}
/// 行内小胶囊按钮：提醒行里的周期徽标（点一下轮换间隔）。
pub fn micro_button(
    label: impl Fn() -> String + Copy + 'static,
    accessibility_label: &'static str,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    button()
        .role(AccessibilityRole::Button)
        .accessibility_label(accessibility_label)
        .px(px(4.0))
        .py(px(0.5))
        .rounded(px(radius::SM))
        .flex_row()
        .items_center()
        .justify_center()
        .bg(move || theme().muted)
        .border_1()
        .border_color(move || theme().border)
        .hover(move |s| s.bg(move || theme().secondary))
        .active(|s| s.scale(0.96).opacity(0.9))
        .focus_visible(focus_ring)
        .transition_colors()
        .transition_transform()
        .duration(move || if reduce_motion() { 0.0_f32 } else { motion::FAST })
        .on_press(move || on_press())
        .child(
            text(label)
                .text_size(px(font::CAPTION))
                .text_color(move || theme().muted_foreground),
        )
}
/// 数字微调按钮：24×24 的「−」「+」，和它两侧的输入框同高。
pub fn step_button(
    label: &'static str,
    sign: &'static str,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    button()
        .role(AccessibilityRole::Button)
        .accessibility_label(label)
        .w(px(size::CONTROL_MD))
        .h(px(size::CONTROL_MD))
        .rounded(px(radius::LG))
        .flex_row()
        .items_center()
        .justify_center()
        .bg(move || theme().secondary)
        .border_1()
        .border_color(move || theme().border)
        .hover(move |s| s.bg(move || theme().muted))
        .active(|s| s.scale(0.94).opacity(0.9))
        .focus_visible(focus_ring)
        .transition_colors()
        .transition_transform()
        .duration(move || if reduce_motion() { 0.0_f32 } else { motion::FAST })
        .on_press(move || on_press())
        .child(
            text(sign)
                .text_size(px(font::BODY))
                .text_color(move || theme().secondary_foreground),
        )
}
/// 文字按钮：抽屉底部与二级动作用。「主」只用于这一屏唯一的提交动作。
pub fn text_button(
    label: &'static str,
    primary: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    button()
        .role(AccessibilityRole::Button)
        .accessibility_label(label)
        .h(px(size::CONTROL_SM))
        .rounded(px(radius::MD))
        .flex_row()
        .items_center()
        .justify_center()
        .px(if primary { px(12.0) } else { px(8.0) })
        .bg(move || {
            if primary {
                ACCENT_GOLD
            } else {
                theme().secondary
            }
        })
        .border_1()
        .border_color(move || if primary { ACCENT_GOLD } else { theme().border })
        .hover(move |s| {
            s.opacity(if primary { 0.92 } else { 1.0 })
                .bg(move || if primary { ACCENT_GOLD } else { theme().muted })
        })
        .active(|s| s.scale(0.97).opacity(0.9))
        .focus_visible(focus_ring)
        .transition_colors()
        .transition_transform()
        .duration(move || if reduce_motion() { 0.0_f32 } else { motion::FAST })
        .on_press(move || on_press())
        .child(
            text(label)
                .text_size(px(font::BODY))
                .text_color(move || {
                    if primary {
                        ACCENT_GOLD_FG
                    } else {
                        theme().muted_foreground
                    }
                }),
        )
}

/// 撤销条里的「撤销」：20 高的小号金黄实心按钮。
pub fn undo_button(label: &'static str, on_press: impl Fn() + 'static) -> impl IntoElement {
    button()
        .role(AccessibilityRole::Button)
        .accessibility_label(label)
        .h(px(size::CONTROL_SM))
        .px(px(8.0))
        .rounded(px(radius::MD))
        .flex_row()
        .items_center()
        .justify_center()
        .bg(move || ACCENT_GOLD)
        .border_1()
        .border_color(move || ACCENT_GOLD)
        .hover(move |s| s.opacity(0.92))
        .active(|s| s.scale(0.95).opacity(0.9))
        .focus_visible(focus_ring)
        .transition_colors()
        .transition_transform()
        .duration(move || if reduce_motion() { 0.0_f32 } else { motion::FAST })
        .on_press(move || on_press())
        .child(
            text(label)
                .text_size(px(font::LABEL))
                .text_color(move || ACCENT_GOLD_FG),
        )
}
