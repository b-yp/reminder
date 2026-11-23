//! 设置区的开关：`nsh/base` 的 `switch` 加挂件外观。

use nsh_base::{switch, theme};
use nsh_core::prelude::*;

use super::focus::focus_ring;
use super::tokens::motion;
use crate::theme_mode::{ACCENT_GOLD, SWITCH_KNOB};

/// 轨道宽高与内距。滑块 = 高度 − 2×(边框 + 内距)，行程 = 宽度 − 高度，
/// 所以 14 的滑块在 20 高的轨道里正好留出 2pt 的上下边距，不会顶出轨道。
const TRACK_W: f32 = 34.0;
const TRACK_H: f32 = 20.0;
const TRACK_PAD: f32 = 2.0;
const THUMB: f32 = 14.0;
const TRAVEL: f32 = TRACK_W - TRACK_H;

/// 设置抽屉里的开关：34×20 的轨道 + 14 的滑块，开启时金黄高亮，
/// 状态变化动效来自 `motion::FAST`、随系统「减弱动态效果」归零。
/// 不可用时轨道退回中性色、滑块变淡：开关看起来就不能动。
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
        .focus_visible(focus_ring)
        .on_change(move |next: &bool| on_change(*next))
        .track_style(move |s| {
            let track = s
                .w(px(TRACK_W))
                .h(px(TRACK_H))
                .p(px(TRACK_PAD))
                .rounded_full()
                .bg(move || theme().muted)
                .border_1()
                .border_color(move || theme().border)
                .transition_colors()
                .duration(move || if reduce_motion() { 0.0_f32 } else { motion::FAST })
                .checked(|s| s.bg(ACCENT_GOLD).border_color(ACCENT_GOLD));
            // 不可用：整颗开关变淡，读起来就是「这条现在改不了」
            if disabled { track.opacity(0.45) } else { track }
        })
        .thumb_style(move |s| {
            let thumb = s
                .size(px(THUMB))
                .rounded_full()
                .bg(move || SWITCH_KNOB)
                .border_1()
                .border_color(move || theme().border)
                .transition_transform()
                .duration(move || if reduce_motion() { 0.0_f32 } else { motion::FAST })
                .checked(|s| s.translate_x(px(TRAVEL)));
            if disabled { thumb.opacity(0.55) } else { thumb }
        })
}
