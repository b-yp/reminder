//! 「新建提醒」抽屉：常用推荐 / 自定义新提醒两个页签，含草稿复位、
//! 循环周期与每天定点两套表单，以及图标主题选择。

use nsh_base::{collapsible, theme};
use nsh_core::element::input;
use nsh_core::prelude::*;

use crate::app::AppCtx;
use crate::constants::{
    DRAFT_HOUR, DRAFT_ICON, DRAFT_MINS, DRAFT_MINUTE, ICON_CHOICES_FOCUS, ICON_CHOICES_LIFE,
};
use crate::icons;
use crate::localtime;
use crate::model::{ReminderItem, Schedule, insert_item_at_top};
use crate::presets::{DAILY_PRESETS, INTERVAL_PRESETS};
use crate::theme_mode::ACCENT_DANGER_FG;
use crate::ui;
use crate::views::presets;

mod fields;
use fields::{render_daily_fields, render_interval_fields, schedule_mode_button};

/// 「新建提醒」抽屉的页签与自定义表单草稿。
///
/// 默认值只在这里写一份：首次打开与之后每次重新打开都回到同一份初始状态，
/// 复位逻辑（[`AddDraft::reset`]）不会再和 `signal(...)` 的初始值各写一遍。
#[derive(Clone, Copy)]
struct AddDraft {
    /// 当前页签：false = 常用推荐，true = 自定义新提醒
    tab_custom: Signal<bool>,
    name: Signal<String>,
    /// 循环周期（分钟）
    mins: Signal<String>,
    /// 提醒方式：false = 循环周期，true = 每天定点
    daily: Signal<bool>,
    hour: Signal<String>,
    minute: Signal<String>,
    icon: Signal<String>,
    error: Signal<Option<String>>,
}

impl Default for AddDraft {
    fn default() -> Self {
        Self {
            tab_custom: signal(false),
            name: signal(String::new()),
            mins: signal(DRAFT_MINS.to_string()),
            daily: signal(false),
            hour: signal(DRAFT_HOUR.to_string()),
            minute: signal(DRAFT_MINUTE.to_string()),
            icon: signal(DRAFT_ICON.to_string()),
            error: signal(None),
        }
    }
}

impl AddDraft {
    /// 回到「干净首屏」：推荐页签、空名称、默认周期/时刻/图标、无错误提示
    fn reset(&self) {
        self.tab_custom.set(false);
        self.name.set(String::new());
        self.mins.set(DRAFT_MINS.to_string());
        self.daily.set(false);
        self.hour.set(DRAFT_HOUR.to_string());
        self.minute.set(DRAFT_MINUTE.to_string());
        self.icon.set(DRAFT_ICON.to_string());
        self.error.set(None);
    }
}

/// 新建提醒抽屉（点击加号展开，集成常用推荐预设与自定义新提醒配置）。
///
/// 每次打开都从「常用推荐」和空草稿开始；收起时保持原样，
/// 所以收起动画里不会先闪换成推荐页。
pub fn render(app: AppCtx) -> impl IntoElement {
    let AppCtx {
        show_add: show,
        add_reset,
        items,
        ..
    } = app;
    let on_save = move || app.save();
    let draft = AddDraft::default();

    // 打开（或空列表入口强制）时复位。`add_reset` 只用来订阅：
    // 抽屉已打开时 `show` 不变，靠它把页签和草稿拉回干净首屏。
    effect(move || {
        let _ = add_reset.get();
        if show.get() {
            draft.reset();
        }
    });

    collapsible().open(show).reveal(true).content(
        view()
            .window_drag(false)
            .p(px(8.0))
            .mb(px(4.0))
            .rounded(px(8.0))
            .bg(move || theme().muted.opacity(0.40))
            .border_1()
            .border_color(move || theme().border)
            .flex_col()
            .gap(px(6.0))
            // 顶部分段选择：【常用推荐】与【自定义新提醒】
            .child(ui::choice_group(
                [
                    (false, "常用推荐", icons::ICON_STAR),
                    (true, "自定义新提醒", icons::ICON_EDIT),
                ]
                .into_iter()
                .map(|(is_custom, label, icon)| {
                    ui::segment_toggle(
                        label,
                        icon,
                        move || draft.tab_custom.get() == is_custom,
                        move |next: bool| {
                            if next {
                                draft.tab_custom.set(is_custom);
                                draft.error.set(None);
                            }
                        },
                    )
                }),
            ))
            // 模式切换区：常用推荐网格 vs 自定义配置表单
            .child(
                when(
                    move || !draft.tab_custom.get(),
                    move || {
                        view()
                            .w_full()
                            .p(px(6.0))
                            .rounded(px(6.0))
                            .bg(move || theme().card)
                            .border_1()
                            .border_color(move || theme().border)
                            .flex_col()
                            .gap(px(5.0))
                            // 循环打卡：每隔 X 分钟提醒一次
                            .child(presets::section_label("循环打卡", "按间隔反复提醒"))
                            .child(presets::render_grid(INTERVAL_PRESETS, app))
                            // 定点时刻：每天同一时刻提醒（午餐 / 下班 / 睡觉）
                            .child(presets::section_label("定点时刻", "每天到点提醒"))
                            .child(presets::render_grid(DAILY_PRESETS, app))
                    },
                )
                .otherwise(move || {
                    view()
                        .w_full()
                        .p(px(8.0))
                        .rounded(px(6.0))
                        .bg(move || theme().card)
                        .border_1()
                        .border_color(move || theme().border)
                        .flex_col()
                        .gap(px(6.0))
                        // Field 1: 提醒名称
                        .child(
                            view()
                                .flex_col()
                                .gap(px(3.0))
                                .child(
                                    view()
                                        .flex_row()
                                        .items_center()
                                        .justify_between()
                                        .child(
                                            text("提醒事项名称")
                                                .text_size(px(11.0))
                                                .text_color(move || theme().muted_foreground),
                                        )
                                        .child(
                                            text(move || draft.error.get().unwrap_or_default())
                                                .text_size(px(10.5))
                                                .text_color(move || ACCENT_DANGER_FG),
                                        ),
                                )
                                .child(
                                    input()
                                        .test_id("custom-name")
                                        .placeholder("如：站姿切换、滴眼药水、工位通风")
                                        .value(move || draft.name.get())
                                        .on_change(move |s: &str| {
                                            draft.name.set(s.to_string());
                                            draft.error.set(None);
                                        })
                                        .w_full()
                                        .h(px(25.0))
                                        .px(px(8.0))
                                        .rounded(px(4.0))
                                        .bg(move || theme().background)
                                        .border_1()
                                        .border_color(move || theme().input)
                                        .text_size(px(11.0))
                                        .text_color(move || theme().foreground),
                                ),
                        )
                        // Field 2: 提醒方式（循环周期 / 每天定点）+ 具体时刻
                        .child(
                            view()
                                .flex_col()
                                .gap(px(4.0))
                                .child(
                                    view()
                                        .flex_row()
                                        .items_center()
                                        .justify_between()
                                        .child(
                                            text("提醒方式")
                                                .text_size(px(11.0))
                                                .text_color(move || theme().muted_foreground),
                                        )
                                        .child(
                                            view()
                                                .flex_row()
                                                .items_center()
                                                .gap(px(3.0))
                                                .child(schedule_mode_button(
                                                    draft.daily,
                                                    false,
                                                    "循环周期",
                                                    "schedule-mode-interval",
                                                ))
                                                .child(schedule_mode_button(
                                                    draft.daily,
                                                    true,
                                                    "每天定点",
                                                    "schedule-mode-daily",
                                                )),
                                        ),
                                )
                                .child(
                                    when(
                                        move || draft.daily.get(),
                                        move || {
                                            render_daily_fields(
                                                draft.hour,
                                                draft.minute,
                                                draft.error,
                                            )
                                        },
                                    )
                                    .otherwise(move || {
                                        render_interval_fields(draft.mins, draft.error)
                                    }),
                                ),
                        )
                        // Field 3: 图标分类与高光色（16 种丰富主题，两排 8 列精致排布）
                        .child(
                            view()
                                .flex_col()
                                .gap(px(3.0))
                                .child(
                                    text("图标主题")
                                        .text_size(px(11.0))
                                        .text_color(move || theme().muted_foreground),
                                )
                                // 16 个图标 = 一个 toggle 组，两排铺开
                                .child(ui::choice_grid([
                                    view().flex_row().items_center().justify_between().children(
                                        ICON_CHOICES_LIFE.iter().copied().map(|(ic, label)| {
                                            ui::icon_choice(
                                                ic,
                                                label,
                                                move || draft.icon.get() == ic,
                                                move |next: bool| {
                                                    if next {
                                                        draft.icon.set(ic.to_string())
                                                    }
                                                },
                                            )
                                        }),
                                    ),
                                    view().flex_row().items_center().justify_between().children(
                                        ICON_CHOICES_FOCUS.iter().copied().map(|(ic, label)| {
                                            ui::icon_choice(
                                                ic,
                                                label,
                                                move || draft.icon.get() == ic,
                                                move |next: bool| {
                                                    if next {
                                                        draft.icon.set(ic.to_string())
                                                    }
                                                },
                                            )
                                        }),
                                    ),
                                ])),
                        )
                        // Field 4: 底部操作栏
                        .child(
                            view()
                                .flex_row()
                                .items_center()
                                .justify_between()
                                .pt(px(2.0))
                                .child(ui::text_button("收起", false, move || {
                                    show.set(false);
                                }))
                                .child(view().test_id("custom-submit").child(ui::text_button(
                                    "开启提醒",
                                    true,
                                    move || {
                                        let name = draft.name.get().trim().to_string();
                                        if name.is_empty() {
                                            draft.error.set(Some("请输入名称".to_string()));
                                            return;
                                        }
                                        // 按当前模式解析出计时方式
                                        let (schedule, interval_mins) = if draft.daily.get() {
                                            let hour = match draft.hour.get().trim().parse::<u32>()
                                            {
                                                Ok(hour) if hour <= 23 => hour,
                                                _ => {
                                                    draft.error.set(Some(
                                                        "小时需在 0-23 之间".to_string(),
                                                    ));
                                                    return;
                                                }
                                            };
                                            let minute =
                                                match draft.minute.get().trim().parse::<u32>() {
                                                    Ok(minute) if minute <= 59 => minute,
                                                    _ => {
                                                        draft.error.set(Some(
                                                            "分钟需在 0-59 之间".to_string(),
                                                        ));
                                                        return;
                                                    }
                                                };
                                            (Schedule::daily(hour, minute), 0)
                                        } else {
                                            let mins = match draft.mins.get().trim().parse::<u32>()
                                            {
                                                Ok(mins) if mins > 0 => mins,
                                                _ => {
                                                    draft.error.set(Some(
                                                        "请输入有效分钟数(≥1)".to_string(),
                                                    ));
                                                    return;
                                                }
                                            };
                                            if mins > 1440 {
                                                draft.error.set(Some(
                                                    "时长上限为 1440 分钟 (24小时)".to_string(),
                                                ));
                                                return;
                                            }
                                            (Schedule::Interval, mins)
                                        };
                                        if items.with(|list| list.iter().any(|x| x.name == name)) {
                                            draft.error.set(Some("该提醒已在列表中".to_string()));
                                            return;
                                        }
                                        let now = localtime::now();
                                        let icon = draft.icon.get();
                                        let item = match schedule {
                                            Schedule::Interval => {
                                                ReminderItem::new(&name, icon, interval_mins * 60)
                                            }
                                            Schedule::Daily { hour, minute } => {
                                                ReminderItem::new_daily(
                                                    &name,
                                                    icon,
                                                    u32::from(hour),
                                                    u32::from(minute),
                                                    &now,
                                                )
                                            }
                                        };
                                        items.update(|list| {
                                            insert_item_at_top(list, item);
                                        });
                                        on_save();
                                        show.set(false);
                                    },
                                ))),
                        )
                }),
            ),
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_input_element() {
        use nsh_core::element::input;
        let _ = input().placeholder("测试输入");
    }
}
