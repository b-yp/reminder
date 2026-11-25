//! 提醒列表：空状态引导、基于 key 的响应式列表，以及单行提醒
//! （环形进度图标 + 名称倒计时 + 当天统计 + 打卡 / 延后 / 删除）。

use nsh_base::theme;
use nsh_core::element::AdaptiveColor;
use nsh_core::prelude::*;

use crate::app::AppCtx;
use crate::icons;
use crate::localtime;
use crate::model::ReminderItem;
use crate::theme_mode::{ACCENT_GOLD_BG, ACCENT_GOLD_STRONG, OVERDUE_FG, OVERDUE_WASH};
use crate::ui;
use crate::ui::tokens::{font, radius, space};

/// 核心列表：当列表为空时展示精致的空状态引导；有任务时基于 each 原语基于 key 响应式高效追踪每个 item 的变化
pub fn render(app: AppCtx) -> impl IntoElement {
    let AppCtx { items, .. } = app;
    when(
        move || items.with(|list| list.is_empty()),
        move || render_empty_state(move || app.open_add()),
    )
    .otherwise(move || {
        view()
            .window_drag(true)
            .w_full()
            .flex_col()
            .gap(px(space::SNUG))
            .child(each(
                items,
                |item| item.id,
                move |item_sig, _| render_item_row(item_sig, app),
            ))
    })
}

/// 列表为空时的空状态展示：轻量金黄微标、友好文案与直达预设的快捷按钮（防信息断崖）
fn render_empty_state(on_add: impl Fn() + Copy + 'static) -> impl IntoElement {
    view()
        .window_drag(true)
        .w_full()
        .py(px(26.0))
        .px(px(16.0))
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(space::GAP))
        .child(
            view()
                .w(px(38.0))
                .h(px(38.0))
                .rounded_full()
                .bg(ACCENT_GOLD_BG)
                .items_center()
                .justify_center()
                .child(icons::icon_svg(icons::ICON_BELL, 18.0, ACCENT_GOLD_STRONG)),
        )
        .child(
            view()
                .flex_col()
                .items_center()
                .gap(px(2.0))
                .child(
                    text("暂无进行中的提醒")
                        .text_size(px(font::TITLE))
                        .text_color(move || theme().foreground),
                )
                .child(
                    text("添加喝水、久坐活动等习惯开始提醒")
                        .text_size(px(font::LABEL))
                        .text_color(move || theme().muted_foreground),
                ),
        )
        .child(view().pt(px(4.0)).window_drag(false).child(ui::text_button(
            "添加常用提醒",
            true,
            on_add,
        )))
}

/// 单个提醒行：左侧发光进度图标 + 中间名称倒计时 + 完成次数 + 右侧 [✓] [+5] [🗑]
fn render_item_row(item_sig: ReadSignal<ReminderItem>, app: AppCtx) -> impl IntoElement {
    let AppCtx { items, .. } = app;
    let on_save = move || app.save();
    let on_delete = move |id| app.delete(id);
    let item_id = item_sig.get_untracked().id;
    let icon_key = item_sig.get_untracked().icon.clone();
    // 打卡后的反馈：让按钮弹一下（在 UI 线程播放，减弱动态效果时直接落到终态）
    let check_pop = motion();

    view()
        .window_drag(true)
        .w_full()
        .flex_row()
        .items_center()
        .justify_between()
        .gap(px(space::GAP))
        .py(px(space::TIGHT))
        .px(px(space::TIGHT))
        .rounded(px(radius::LG))
        // 逾期时用固定的警示底色令牌（浅色淡红、深色暗红），不靠 opacity 叠加
        .bg(move || {
            if item_sig.get().is_expired() {
                OVERDUE_WASH
            } else {
                AdaptiveColor::TRANSPARENT
            }
        })
        // 左半部分：圆环进度图标 + 标题/时间（弹性收缩且 min_w(0)，防长文本挤压右侧操作区）
        .child(
            view()
                .flex_1()
                .min_w(px(0.0))
                .flex_row()
                .items_center()
                .gap(px(space::GAP))
                // 左侧发光圆环进度图标（到期时红光警示，SVG 底轨原生自适应明暗）
                .child(
                    view()
                        .flex_shrink_0()
                        .w(px(36.0))
                        .h(px(36.0))
                        .items_center()
                        .justify_center()
                        .text_color(move || theme().foreground)
                        .child(
                            svg(move || {
                                let it = item_sig.get();
                                let is_exp = it.is_expired();
                                let color = icons::get_item_color(&icon_key);
                                let ratio = if is_exp { 1.0 } else { it.progress_ratio() };
                                let markup = icons::ring_progress_svg(&icon_key, ratio, color);
                                nsh_core::element::ImageSrc::svg_markup(&markup)
                            })
                            .w(px(36.0))
                            .h(px(36.0)),
                        ),
                )
                // 标题（第一行独享完整宽度）与周期/今日统计（第二行，点击周期可快速切换常见时长）
                .child(
                    view()
                        .flex_1()
                        .min_w(px(0.0))
                        .flex_col()
                        .justify_center()
                        .gap(px(3.0))
                        .child(
                            text(move || item_sig.get().name.clone())
                                .w_full()
                                .min_w(px(0.0))
                                .text_size(px(font::TITLE))
                                .text_color(move || theme().foreground)
                                .truncate(),
                        )
                        .child(
                            view()
                                .w_full()
                                .min_w(px(0.0))
                                .flex_row()
                                .items_center()
                                .overflow_hidden()
                                .gap(px(5.0))
                                .child(view().flex_shrink_0().window_drag(false).child(
                                    ui::micro_button(
                                        move || item_sig.get().format_badge(),
                                        "切换提醒周期",
                                        move || {
                                            let mut changed = false;
                                            items.update(|list| {
                                                if let Some(it) =
                                                    list.iter_mut().find(|x| x.id == item_id)
                                                {
                                                    // 定点提醒按墙上时间走，不参与「点击轮换间隔」
                                                    if !it.is_daily() {
                                                        let cur_m = it.interval_secs / 60;
                                                        let next_m = match cur_m {
                                                            0..=19 => 20,
                                                            20..=24 => 25,
                                                            25..=29 => 30,
                                                            30..=44 => 45,
                                                            45..=59 => 60,
                                                            60..=89 => 90,
                                                            90..=119 => 120,
                                                            _ => 15,
                                                        };
                                                        it.update_interval_mins(next_m);
                                                        changed = true;
                                                    }
                                                }
                                            });
                                            if changed {
                                                on_save();
                                            }
                                        },
                                    ),
                                ))
                                .child(when(
                                    move || item_sig.get().today_label(&localtime::now()).is_some(),
                                    move || {
                                        text(move || {
                                            item_sig
                                                .get()
                                                .today_label(&localtime::now())
                                                .map(|label| format!("· {label}"))
                                                .unwrap_or_default()
                                        })
                                        .flex_1()
                                        .min_w(px(0.0))
                                        .text_size(px(font::LABEL))
                                        .tabular_nums()
                                        .text_color(move || theme().muted_foreground)
                                        .truncate()
                                    },
                                )),
                        ),
                ),
        )
        // 右半部分：倒计时/超时 + 主行动 [✓] + 低频动作菜单 [⋯]（固定不被压缩）
        .child(
            view()
                .window_drag(false)
                .flex_shrink_0()
                .flex_row()
                .items_center()
                .gap(px(space::SNUG))
                // 倒计时/超时状态：紧邻打卡按钮，使用等宽数字避免每秒跳动引起右侧按钮位移
                .child(
                    view()
                        .test_id("row-time")
                        .flex_row()
                        .items_center()
                        .justify_end()
                        .gap(px(3.0))
                        .child(when(
                            move || item_sig.get().is_expired(),
                            move || icons::icon_svg(icons::ICON_ALERT, 11.5, OVERDUE_FG),
                        ))
                        .child(
                            text(move || item_sig.get().format_remaining_compact())
                                .text_size(px(font::BODY))
                                .tabular_nums()
                                .text_color(move || {
                                    if item_sig.get().is_expired() {
                                        OVERDUE_FG
                                    } else {
                                        theme().muted_foreground
                                    }
                                }),
                        ),
                )
                // 打卡：行内唯一的强调动作。只有「已到期」这一行给金黄，
                // 其余行用中性底 —— 一张卡片里只留一个焦点。
                .child(
                    view()
                        .test_id("row-check")
                        .bind_motion(&check_pop)
                        .child(ui::action_button(
                            "打卡完成",
                            ui::tokens::size::CONTROL_MD,
                            move || {
                                if item_sig.get().is_expired() {
                                    ui::ActionTone::Accent
                                } else {
                                    ui::ActionTone::Quiet
                                }
                            },
                            || false,
                            move || {
                                let now = localtime::now();
                                items.update(|list| {
                                    if let Some(it) = list.iter_mut().find(|x| x.id == item_id) {
                                        it.complete(&now);
                                    }
                                });
                                on_save();
                                check_pop.play(MotionPreset::Pulse);
                            },
                            icons::icon_svg(icons::ICON_CHECK, 12.0, move || {
                                if item_sig.get().is_expired() {
                                    ACCENT_GOLD_STRONG
                                } else {
                                    theme().muted_foreground
                                }
                            }),
                        ))
                        .tooltip(ui::tooltip_bubble("完成打卡")),
                )
                // 延后与删除收进「⋯」菜单：删除触发底部撤销 Toast
                .child(view().test_id("row-more").child(ui::row_menu(
                    move || item_sig.get().is_done_today(&localtime::now()),
                    move || {
                        let now = localtime::now();
                        let mut changed = false;
                        items.update(|list| {
                            if let Some(it) = list.iter_mut().find(|x| x.id == item_id) {
                                changed = it.snooze(300, &now);
                            }
                        });
                        if changed {
                            on_save();
                        }
                    },
                    move || on_delete(item_id),
                ))),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::AppState;
    use nsh_testing::LayoutNode;

    #[component]
    fn ShortRowHost() -> impl IntoElement {
        let mut item = ReminderItem::new("喝水", "droplet", 1800);
        item.complete(&localtime::now());
        let item_sig = signal(item);
        view().w(px(320.0)).child(render_item_row(
            item_sig.read_only(),
            AppCtx::new(AppState {
                items: vec![item_sig.get()],
                ..Default::default()
            }),
        ))
    }

    /// 40 个中文字符：用来验证长名字不会挤坏右侧车道
    const LONG_NAME: &str =
        "这是测试超长任务名称用来验证行布局不会被挤坏的四十个中文字符串测试用例一二三四五";

    #[component]
    fn LongRowHost() -> impl IntoElement {
        assert_eq!(LONG_NAME.chars().count(), 40);
        let mut item = ReminderItem::new(LONG_NAME, "droplet", 1800);
        item.complete(&localtime::now());
        let item_sig = signal(item);
        view().w(px(320.0)).child(render_item_row(
            item_sig.read_only(),
            AppCtx::new(AppState {
                items: vec![item_sig.get()],
                ..Default::default()
            }),
        ))
    }

    fn find<'a>(nodes: &'a [LayoutNode], text: &str) -> &'a LayoutNode {
        nodes
            .iter()
            .find(|n| n.text.as_ref().is_some_and(|t| t.text.text == text))
            .unwrap_or_else(|| panic!("布局里找不到文本 {text:?}"))
    }

    fn by_test_id<'a>(nodes: &'a [LayoutNode], id: &str) -> &'a LayoutNode {
        nodes
            .iter()
            .find(|n| n.test_id.as_deref() == Some(id))
            .unwrap_or_else(|| panic!("布局里找不到 test_id {id:?}"))
    }

    #[test]
    fn test_long_name_row_regression_320w() {
        use nsh_engine::{EngineConfig, Points, Root, Size};
        use nsh_testing::Headless;

        let config_short = EngineConfig {
            viewport: Size::new(Points(320.0), Points(200.0)),
            ..EngineConfig::new(Root::of::<ShortRowHost>())
        };
        let mut app_short = Headless::start(config_short).unwrap();
        app_short.wait_for("喝水").unwrap();

        let config_long = EngineConfig {
            viewport: Size::new(Points(320.0), Points(200.0)),
            ..EngineConfig::new(Root::of::<LongRowHost>())
        };
        let mut app_long = Headless::start(config_long).unwrap();
        app_long.wait_for("30m").unwrap();

        // 1. 验证徽标与统计文本在 40 个中文字符超长名称下依然完整显示
        assert!(app_long.screen().shows("30m"), "周期徽标在长文本下必须可见");
        assert!(
            app_long.screen().shows("· 今日 1 次"),
            "统计文案在长文本下必须可见"
        );
        assert!(
            app_long.screen().shows("30:00"),
            "倒计时在长文本下必须可见"
        );

        let nodes_short = app_short.screen().layout_nodes();
        let nodes_long = app_long.screen().layout_nodes();

        // 2. 名称被 flex_1 + truncate 限制在分配宽度内，不撑开行
        assert_eq!(
            find(&nodes_long, LONG_NAME).bounds.size.width,
            find(&nodes_short, "喝水").bounds.size.width,
            "长文本由于 flex_1 和 truncate 约束在可用宽度内"
        );

        // 3. 周期徽标不位移、不变形
        assert_eq!(
            find(&nodes_long, "30m").bounds,
            find(&nodes_short, "30m").bounds,
            "周期徽标 bounds 在长文本下不发生位移和变形"
        );

        // 4. 右侧车道：倒计时不位移，打卡与菜单都是 24 的方格
        let time_long = by_test_id(&nodes_long, "row-time");
        assert_eq!(
            time_long.bounds,
            by_test_id(&nodes_short, "row-time").bounds,
            "倒计时车道在长文本下不位移"
        );

        let check_long = by_test_id(&nodes_long, "row-check");
        let check_short = by_test_id(&nodes_short, "row-check");
        let more_long = by_test_id(&nodes_long, "row-more");
        assert_eq!(
            check_long.bounds, check_short.bounds,
            "打卡按钮在长文本下不位移"
        );
        for (label, node) in [("打卡", check_long), ("更多", more_long)] {
            assert!(
                (node.bounds.size.width.0 - 24.0).abs() < 0.5
                    && (node.bounds.size.height.0 - 24.0).abs() < 0.5,
                "{label}按钮应为 24×24 的方格，实测 {}×{}",
                node.bounds.size.width.0,
                node.bounds.size.height.0
            );
        }
        assert_eq!(
            more_long.bounds.origin.x.0,
            check_long.bounds.origin.x.0 + 24.0 + 6.0,
            "打卡与更多按钮同组等宽、间距一致"
        );
    }
}
