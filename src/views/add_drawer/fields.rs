//! 自定义表单的字段控件：循环周期 / 每天定点的模式切换、数字与时间输入、
//! 快捷填入胶囊。只负责外观与输入约束，草稿状态由抽屉传入。

use chrono::Timelike;
use nsh_base::{TimeOfDay, theme, time_field};
use nsh_core::element::input;
use nsh_core::prelude::*;

use crate::constants::{MINUTE_CHIPS, TIME_CHIPS};
use crate::theme_mode::{ACCENT_GOLD, ACCENT_GOLD_STRONG};
use crate::ui;

/// 自定义提醒的「循环周期 / 每天定点」切换按钮
pub(super) fn schedule_mode_button(
    mode: Signal<bool>,
    daily: bool,
    label: &'static str,
    test_id: &'static str,
) -> impl IntoElement {
    // 二选一的选择项，底座是 nsh/base 的 toggle：方向键在组内移动、Space 切换
    view().test_id(test_id).child(ui::choice_chip(
        label,
        move || mode.get() == daily,
        move |next: bool| {
            if next {
                mode.set(daily);
            }
        },
    ))
}

/// 循环周期配置：一个 NumberInput（底座 nsh/base 的 number_input，自带
/// 上下键步进、按住连发、min/max 钳制、spinbutton 角色）+ 快捷填入胶囊。
pub(super) fn render_interval_fields(
    custom_mins_str: Signal<String>,
    error_tip: Signal<Option<String>>,
) -> impl IntoElement {
    view()
        .flex_col()
        .gap(px(3.0))
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .child(
                    text("循环周期")
                        .text_size(px(11.0))
                        .text_color(move || theme().muted_foreground),
                )
                .child(
                    view().flex_row().items_center().gap(px(3.0)).child(
                        view()
                            .test_id("custom-mins")
                            .flex_row()
                            .items_center()
                            .gap(px(3.0))
                            .child(ui::step_button("减少 1 分钟", "-", move || {
                                let cur = custom_mins_str.get().parse::<u32>().unwrap_or(30);
                                custom_mins_str.set(cur.saturating_sub(1).max(1).to_string());
                                error_tip.set(None);
                            }))
                            .child(
                                input()
                                    .placeholder("30")
                                    .value(move || custom_mins_str.get())
                                    .label("循环周期分钟数")
                                    .on_change(move |s: &str| {
                                        let digits: String = s
                                            .chars()
                                            .filter(|c| c.is_ascii_digit())
                                            .take(4)
                                            .collect();
                                        custom_mins_str.set(digits);
                                        error_tip.set(None);
                                    })
                                    .w(px(38.0))
                                    .h(px(20.0))
                                    .px(px(4.0))
                                    .rounded(px(3.0))
                                    .bg(move || theme().background)
                                    .border_1()
                                    .border_color(ACCENT_GOLD)
                                    .text_size(px(11.0))
                                    .text_color(ACCENT_GOLD_STRONG),
                            )
                            .child(ui::step_button("增加 1 分钟", "+", move || {
                                let cur = custom_mins_str.get().parse::<u32>().unwrap_or(30);
                                custom_mins_str.set((cur + 1).min(1440).to_string());
                                error_tip.set(None);
                            })),
                    ),
                ),
        )
        // 快捷填入：一组互斥的 toggle（方向键可在组内移动）
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .gap(px(3.0))
                .child(
                    text("快捷填入:")
                        .text_size(px(10.5))
                        .text_color(move || theme().muted_foreground),
                )
                .child(ui::choice_group(MINUTE_CHIPS.iter().map(
                    |&(label, mins)| {
                        ui::choice_chip(
                            label,
                            move || custom_mins_str.get() == mins.to_string(),
                            move |next: bool| {
                                if next {
                                    custom_mins_str.set(mins.to_string());
                                    error_tip.set(None);
                                }
                            },
                        )
                    },
                ))),
        )
}

/// 定点时刻配置：一个 TimeField（底座 nsh/base 的 time_field，分段编辑、
/// 上下键步进、读屏按时间段朗读）+ 常用时刻胶囊。
pub(super) fn render_daily_fields(
    custom_hour_str: Signal<String>,
    custom_minute_str: Signal<String>,
    error_tip: Signal<Option<String>>,
) -> impl IntoElement {
    view()
        .flex_col()
        .gap(px(3.0))
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .child(
                    text("每天提醒时间")
                        .text_size(px(11.0))
                        .text_color(move || theme().muted_foreground),
                )
                .child(
                    view().test_id("custom-time").child(
                        time_field()
                            .value(time_of_day(
                                &custom_hour_str.get(),
                                &custom_minute_str.get(),
                            ))
                            .accessibility_label("每天提醒时间")
                            .on_change(move |value: &TimeOfDay| {
                                custom_hour_str.set(format!("{:02}", value.0.hour()));
                                custom_minute_str.set(format!("{:02}", value.0.minute()));
                                error_tip.set(None);
                            })
                            .w(px(72.0))
                            .h(px(24.0))
                            .rounded(px(3.0))
                            .bg(move || theme().background)
                            .border_1()
                            .border_color(ACCENT_GOLD)
                            .segment_style(|s| s.text_size(px(11.0)).text_color(ACCENT_GOLD_STRONG))
                            .separator_style(|s| {
                                s.text_size(px(11.0)).text_color(ACCENT_GOLD_STRONG)
                            }),
                    ),
                ),
        )
        // 常用定点时刻：同样是一组互斥 toggle
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .gap(px(3.0))
                .child(
                    text("常用时刻:")
                        .text_size(px(10.5))
                        .text_color(move || theme().muted_foreground),
                )
                .child(ui::choice_group(TIME_CHIPS.iter().map(
                    |&(label, hour, minute)| {
                        ui::choice_chip(
                            label,
                            move || {
                                custom_hour_str.get() == format!("{hour:02}")
                                    && custom_minute_str.get() == format!("{minute:02}")
                            },
                            move |next: bool| {
                                if next {
                                    custom_hour_str.set(format!("{hour:02}"));
                                    custom_minute_str.set(format!("{minute:02}"));
                                    error_tip.set(None);
                                }
                            },
                        )
                    },
                ))),
        )
}

/// 把「时 / 分」字符串解析成 TimeField 接受的时间；非法值交给它显示的空值。
fn time_of_day(hour: &str, minute: &str) -> Option<TimeOfDay> {
    let hour: u32 = hour.trim().parse().ok()?;
    let minute: u32 = minute.trim().parse().ok()?;
    if hour > 23 || minute > 59 {
        return None;
    }
    chrono::NaiveTime::from_hms_opt(hour, minute, 0).map(TimeOfDay)
}
