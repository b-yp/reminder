//! 单行文本输入：表单里所有能打字的地方都从这里出。
//!
//! 高度、内距、圆角、边界与焦点环只在这里决定，页面只给宽度和布局。
//! 主题自带的 `input` 边界只有 1.26:1，看不出「这里能输入」，所以换成
//! 3.1:1 的 `FIELD_BORDER`；键盘焦点由统一的 `focus_ring` 负责。

use nsh_base::theme;
use nsh_core::element::input;
use nsh_core::prelude::*;

use super::focus::focus_ring;
use super::tokens::{font, radius, size};
use crate::theme_mode::FIELD_BORDER;

/// 单行输入框。`label` 是读屏用的名字（可见标签由页面自己排在旁边），
/// `test_id` 落在输入框自身，测试可以直接读它的值。
pub fn text_field(
    label: &'static str,
    test_id: &'static str,
    placeholder: &'static str,
    value: impl Fn() -> String + 'static,
    on_change: impl Fn(&str) + 'static,
) -> impl IntoElement {
    input()
        .test_id(test_id)
        .label(label)
        .placeholder(placeholder)
        .value(value)
        .on_change(move |text: &str| on_change(text))
        .h(px(size::CONTROL_MD))
        .w_full()
        .px(px(8.0))
        .rounded(px(radius::LG))
        .bg(move || theme().background)
        .border_1()
        .border_color(move || FIELD_BORDER)
        .text_size(px(font::BODY))
        .text_color(move || theme().foreground)
        .focus_visible(focus_ring)
}
