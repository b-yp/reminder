//! 设置区的开关：`nsh/base` 的 `switch` 加挂件外观。

use nsh_base::{switch, theme};
use nsh_core::prelude::*;

use crate::theme_mode::ACCENT_GOLD;

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
                .duration(move || if reduce_motion() { 0.0_f32 } else { 160.0_f32 })
                .checked(|s| s.bg(ACCENT_GOLD).border_color(ACCENT_GOLD))
        })
        .thumb_style(|s| {
            s.size(px(14.0))
                .rounded_full()
                .bg(move || theme().background)
                .border_1()
                .border_color(move || theme().border)
                .transition_transform()
                .duration(move || if reduce_motion() { 0.0_f32 } else { 160.0_f32 })
                .checked(|s| s.translate_x(px(12.0)))
        })
}
