//! 键盘焦点环：全应用只有这一份。
//!
//! 主题自带的 `ring` 在浅色下对白底只有 2.59:1，键盘用户看不出焦点在哪；
//! 这里统一换成高对比度的前景色令牌（见 `theme_mode::FOCUS_RING`），
//! 并且画在控件外框之外，不影响已选中的金边或半透明底色。

use nsh_core::element::StyleRefinement;
use nsh_core::prelude::*;

use super::tokens::size;
use crate::theme_mode::FOCUS_RING;

/// 2pt 焦点环，供所有控件的 `focus_visible` 使用。
pub fn focus_ring(s: StyleRefinement) -> StyleRefinement {
    s.outline(px(size::FOCUS_RING)).outline_color(FOCUS_RING)
}
