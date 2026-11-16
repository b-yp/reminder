//! 应用内共享的 styled 控件层：把 NSH 的无样式行为组件（`nsh_base`）包成
//! 挂件统一的外观，业务区块只组合它们，不各自重写颜色、圆角与动效。

use nsh_base::{switch, theme};
use nsh_core::prelude::*;

use crate::theme_mode::{ACCENT_GOLD, ACCENT_GOLD_BG};

/// 设置抽屉里的小状态徽章用哪种语气
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BadgeTone {
    /// 已生效（品牌金黄）
    Accent,
    /// 需要用户处理（警示色）
    Danger,
    /// 中性 / 未生效
    Muted,
}

/// 设置抽屉里的开关：34×20 的轨道 + 14 的滑块，开启时金黄高亮，
/// 状态变化动效 160ms、随系统「减弱动态效果」归零。
///
/// `on_change` 只收到新状态；写入失败时由调用方把信号改回系统里的真实状态。
pub fn setting_switch(
    checked: Signal<bool>,
    disabled: bool,
    label: &'static str,
    on_change: impl Fn(bool) + 'static,
) -> impl IntoElement {
    switch()
        .checked(checked)
        .disabled(disabled)
        .accessibility_label(label)
        .focus_visible(|s| s.outline(px(2.0)).outline_color(move || theme().ring))
        .on_change(move |next: &bool| on_change(*next))
        .track_style(|s| {
            s.w(px(34.0))
                .h(px(20.0))
                .p(px(3.0))
                .rounded_full()
                .bg(move || theme().muted)
                .border_1()
                .border_color(move || theme().border)
                .transition_colors()
                .duration(move || {
                    if reduce_motion() { 0.0_f32 } else { 160.0_f32 }
                })
                .checked(|s| s.bg(ACCENT_GOLD).border_color(ACCENT_GOLD))
        })
        .thumb_style(|s| {
            s.size(px(14.0))
                .rounded_full()
                .bg(move || theme().background)
                .border_1()
                .border_color(move || theme().border)
                .transition_transform()
                .duration(move || {
                    if reduce_motion() { 0.0_f32 } else { 160.0_f32 }
                })
                .checked(|s| s.translate_x(px(12.0)))
        })
}

/// 设置项的状态徽章：6×1.5 的内边距、4 的圆角、9.5 的文字，
/// 状态切换时颜色 120ms 过渡（减弱动态效果时立即切换）。
pub fn status_badge(
    tone_of: impl Fn() -> BadgeTone + 'static,
    label: impl Fn() -> &'static str + 'static,
) -> impl IntoElement {
    // 结算成响应式值，两处样式（底色、文字色）读同一份状态
    let tone = computed(tone_of);
    view()
        .px(px(6.0))
        .py(px(1.5))
        .rounded(px(4.0))
        .bg(move || match tone.get() {
            BadgeTone::Accent => ACCENT_GOLD_BG,
            BadgeTone::Danger => theme().destructive.opacity(0.12),
            BadgeTone::Muted => theme().muted,
        })
        .transition_colors()
        .duration(move || {
            if reduce_motion() { 0.0_f32 } else { 120.0_f32 }
        })
        .child(
            text(label)
                .text_size(px(9.5))
                .text_color(move || match tone.get() {
                    BadgeTone::Accent => ACCENT_GOLD,
                    BadgeTone::Danger => theme().destructive,
                    BadgeTone::Muted => theme().muted_foreground,
                }),
        )
}
