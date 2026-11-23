//! 底部状态行：今日统计 + 全局热键提示 + 半透明时的不透明度微标。

use nsh_base::theme;
use nsh_core::prelude::*;

use crate::app::AppCtx;
use crate::hotkey;
use crate::icons;
use crate::localtime;
use crate::model::ReminderItem;
use crate::theme_mode::ACCENT_GOLD_STRONG;
use crate::ui::tokens::{font, space};

/// 底部统计文案：只汇总各提醒「今天」的完成次数，历史累计不进入底栏
fn footer_summary(items: &[ReminderItem], now: &localtime::LocalNow) -> String {
    let completed_today = items.iter().fold(0_u32, |sum, item| {
        sum.saturating_add(item.today_completed(now))
    });
    format!(
        "进行中 {} 项 · 今日打卡 {} 次",
        items.len(),
        completed_today
    )
}

/// 底部状态行：细线分隔 + 灰白小字今日统计 + 半透明时指示不透明度，
/// 常态下右侧用键盘图标与键位提示全局热键（一键隐藏 / 呼出悬浮窗）
pub fn render(app: AppCtx) -> impl IntoElement {
    let AppCtx {
        items,
        opacity,
        hotkey,
        hotkey_enabled,
        ..
    } = app;
    let summary_text = computed(move || {
        let now = localtime::now();
        items.with(|list| footer_summary(list, &now))
    });

    view()
        .window_drag(false)
        .flex_row()
        .items_center()
        .justify_between()
        .pt(px(space::SNUG))
        .border_t_1()
        .border_color(move || theme().border)
        .child(
            view().child(
                text(move || summary_text.get())
                    .text_size(px(font::BODY))
                    .text_color(move || theme().muted_foreground),
            ),
        )
        // 右侧：全局热键提示 + 半透明时的不透明度微标（都放得下，热键提示不被挤掉）
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(space::SNUG))
                .child(when(
                    move || hotkey_enabled.get(),
                    move || {
                        view()
                            .test_id("hotkey-hint")
                            .flex_row()
                            .items_center()
                            .gap(px(space::TIGHT))
                            .child(icons::icon_svg(icons::ICON_KEYBOARD, 11.0, move || {
                                theme().muted_foreground
                            }))
                            .child(
                                text(move || hotkey::label(&hotkey.get()))
                                    .text_size(px(font::CAPTION))
                                    .text_color(move || theme().muted_foreground),
                            )
                    },
                ))
                .child(when(
                    move || opacity.get() < 0.99,
                    move || {
                        view()
                            .flex_row()
                            .items_center()
                            .gap(px(3.0))
                            .child(icons::icon_svg(
                                icons::ICON_OPACITY,
                                10.5,
                                ACCENT_GOLD_STRONG,
                            ))
                            .child(
                                text(move || format!("{:.0}%", opacity.get() * 100.0))
                                    .text_size(px(font::CAPTION))
                                    .text_color(ACCENT_GOLD_STRONG),
                            )
                    },
                )),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造一个确定性的本地时刻（偏移 0，便于断言）
    fn local(day: i64) -> localtime::LocalNow {
        localtime::LocalNow {
            unix: day as u64 * 86_400,
            day,
            secs_of_day: 0,
            hour: 0,
            minute: 0,
            second: 0,
            weekday: 0,
            offset: 0,
        }
    }

    #[test]
    fn footer_counts_only_todays_check_ins() {
        let now = local(20_000);
        let mut yesterday = ReminderItem::new("喝水", "droplet", 1800);
        // 30 次历史累计都发生在昨天：底栏只统计今天
        yesterday.completed_count = 30;
        yesterday.completed_by_day.insert(now.day - 1, 30);

        let mut today = ReminderItem::new("起身活动", "walk", 2700);
        today.complete(&now);
        today.complete(&now);

        let items = [yesterday, today];
        assert_eq!(footer_summary(&items, &now), "进行中 2 项 · 今日打卡 2 次");
        assert_eq!(
            footer_summary(&items, &local(20_001)),
            "进行中 2 项 · 今日打卡 0 次"
        );
        // 历史累计仍留在数据层
        assert_eq!(items[0].completed_count, 30);
    }
}
