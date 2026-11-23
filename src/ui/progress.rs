//! 更新下载的细进度条：3pt 轨道 + 品牌金黄进度，宽度随进度平滑过渡。
//!
//! 只表达一件事：这次下载走了多远。旁边的文字给出百分比与字节数，
//! 所以进度条本身不带无障碍名称，也不参与焦点顺序。

use nsh_base::theme;
use nsh_core::prelude::*;

use super::tokens::radius;
use crate::theme_mode::ACCENT_GOLD;

/// 进度变化后的过渡时长（比颜色微动效长一点，读得出「在往前走」）
const SLIDE: f32 = 200.0;

/// 进度条：`fraction` 返回 0.0–1.0。知道总大小时按比例推进；
/// 不知道时给 0，界面只显示已下载的字节数。
///
/// 宽度用 `relative`，是轨道的百分比而不是固定点数：窗口宽度、文字大小或缩放变了，
/// 进度条跟着走，不会停在半格。
pub fn progress_bar(fraction: impl Fn() -> f32 + 'static) -> impl IntoElement {
    let fraction = computed(move || fraction().clamp(0.0, 1.0));
    view()
        .w_full()
        .h(px(3.0))
        .rounded(px(radius::SM))
        .bg(move || theme().muted)
        .overflow_hidden()
        .child(
            view()
                .h_full()
                .rounded(px(radius::SM))
                .bg(ACCENT_GOLD)
                .w(move || relative(fraction.get()))
                .transition_all()
                .duration(move || if reduce_motion() { 0.0_f32 } else { SLIDE }),
        )
}
