//! 设置区的状态徽章：把「已生效 / 需要处理 / 中性」三种语气映射成颜色。

use nsh_base::theme;
use nsh_core::prelude::*;

use crate::theme_mode::{ACCENT_GOLD, ACCENT_GOLD_BG};

/// 状态徽章的语气
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BadgeTone {
    /// 已生效（品牌金黄）
    Accent,
    /// 需要用户处理（警示色）
    Danger,
    /// 中性 / 未生效
    Muted,
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
        .duration(move || if reduce_motion() { 0.0_f32 } else { 120.0_f32 })
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
