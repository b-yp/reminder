//! 底部撤销 ToastStack：基于 NSH `toast_stack()` 与 `ToastManager` 原生原语，
//! 采用 Apple / Sonner 物理弹簧多卡片层叠堆放，悬停/触碰时自动展开为列表。

use std::time::Duration;

use nsh_base::{Anchor, SwipeDirection, ToastTransitionStatus, theme, toast, toast_stack};
use nsh_core::prelude::*;
use nsh_core::time::now;

use crate::app::{AppCtx, toast_motion};
use crate::constants::UNDO_SECONDS;
use crate::icons;
use crate::theme_mode::ACCENT_GOLD;
use crate::ui;
use crate::ui::tokens::{font, motion, radius};

/// 底部多卡片层叠 ToastStack：
/// - 默认层叠堆放（Collapsed）：最新的一张在前景，较旧的依次在后方缩放微露出（Sonner 风格）；
/// - 鼠标悬停 / 触碰时展开（Expanded）：卡片平滑弹簧展开为垂直列表，同时自动暂停 5s 倒计时；
/// - 鼠标移开时收折：自动收折回层叠堆，倒计时恢复推进；
/// - 单卡独立操作：每张卡片均带有「撤销」按钮、5s 动态倒计时与右滑（Swipe Right）独立关闭手势。
pub fn render(app: AppCtx) -> impl IntoElement {
    let AppCtx {
        toast_entries,
        toast_stack_state,
        toast_manager,
        ..
    } = app;

    // 启动 50ms 心跳推进 ToastManager 生命周期（卡片展开时自动暂停倒计时）
    set_interval(Duration::from_millis(50), move || {
        let is_expanded = toast_stack_state.is_expanded();
        let changed = toast_manager
            .with(|m| m.advance(now(), is_expanded).changed);
        if changed {
            app.sync_toasts();
        }
        if !is_expanded {
            for (_, pending, status) in app.toast_entries.get() {
                if status != ToastTransitionStatus::Ending {
                    let ms = pending.remaining_ms.get().saturating_sub(50);
                    pending.remaining_ms.set(ms);
                }
            }
        }
        app.toast_tick.update(|t| *t = t.wrapping_add(1));
    });

    view()
        .absolute()
        .left(px(14.0))
        .right(px(14.0))
        .bottom(px(36.0))
        .pointer_events(PointerEvents::BoxNone)
        .window_drag(false)
        .flex_row()
        .justify_center()
        .child(
            toast_stack()
                .placement(Anchor::BottomCenter)
                .motion(toast_motion())
                .state(toast_stack_state)
                .w_full()
                .on_cancel(move || app.undo_latest())
                .child(each(
                    toast_entries,
                    |(id, _, _)| *id,
                    move |entry, _| {
                        let id = entry.get().0;
                        let name = entry.get().1.item.name.clone();

                        // 卡片的数据全部从 entry 信号里读：`each` 会复用同一行，
                        // 只有信号能跟上 status 的 Starting → Present → Ending 变化；
                        // 若在创建行时把 status 捕获成常量，卡片会永远停在 Starting，
                        // `ending_class` 不生效，退场动画也就不会播。
                        let remaining_secs = {
                            let entry = entry.clone();
                            computed(move || {
                                let _ = app.toast_tick.get();
                                let ms = entry.get().1.remaining_ms.get();
                                (ms + 999) / 1000
                            })
                        };
                        let progress = {
                            let entry = entry.clone();
                            computed(move || {
                                let _ = app.toast_tick.get();
                                let ms = entry.get().1.remaining_ms.get();
                                (ms as f32 / (UNDO_SECONDS * 1000) as f32).clamp(0.0, 1.0)
                            })
                        };

                        toast()
                            .accessibility_label(format!("已删除「{name}」，可撤销"))
                            .transition_status(move || entry.get().2)
                            .swipe_direction(Some(SwipeDirection::Right))
                            .on_dismiss_request(move || app.dismiss_toast(id))
                            .pointer_events(PointerEvents::Auto)
                            .w_full()
                            .px(px(10.0))
                            .py(px(6.5))
                            .rounded(px(radius::XL))
                            .bg(move || theme().popover)
                            .border_1()
                            .border_color(move || theme().border)
                            .shadow_lg()
                            .transition_all()
                            .duration(move || if reduce_motion() { 0.0_f32 } else { motion::BASE })
                            .starting_class("opacity-0 translate-y-3")
                            .ending_class("opacity-0 translate-y-2 scale-95")
                            .child(
                                view()
                                    .w_full()
                                    .flex_col()
                                    .gap(px(5.0))
                                    .child(
                                        view()
                                            .w_full()
                                            .flex_row()
                                            .items_center()
                                            .justify_between()
                                            .gap(px(8.0))
                                            // 左侧：垃圾桶图标 + 提示文字（超出截断省略）
                                            .child(
                                                view()
                                                    .flex_row()
                                                    .items_center()
                                                    .gap(px(6.0))
                                                    .flex_1()
                                                    .overflow_hidden()
                                                    .child(icons::icon_svg(
                                                        icons::ICON_TRASH,
                                                        12.0,
                                                        move || theme().muted_foreground,
                                                    ))
                                                    .child(
                                                        text(format!("已删除「{name}」"))
                                                            .text_size(px(font::BODY))
                                                            .text_color(move || theme().popover_foreground)
                                                            .truncate(),
                                                    ),
                                            )
                                            // 右侧：撤销按钮 + 倒计时数字
                                            .child(
                                                view()
                                                    .flex_row()
                                                    .items_center()
                                                    .gap(px(6.0))
                                                    .child(ui::undo_button("撤销", move || app.undo(id)))
                                                    .child(
                                                        text(move || format!("{}s", remaining_secs.get()))
                                                            .text_size(px(font::CAPTION))
                                                            .text_color(move || theme().muted_foreground),
                                                    ),
                                            ),
                                    )
                                    // 底部微细倒计时进度条
                                    .child(
                                        view()
                                            .w_full()
                                            .h(px(2.0))
                                            .rounded_full()
                                            .bg(move || theme().muted.opacity(0.40))
                                            .child(
                                                view()
                                                    .h_full()
                                                    .w_full()
                                                    .rounded_full()
                                                    .bg(ACCENT_GOLD)
                                                    .origin_left()
                                                    .scale_x(move || progress.get())
                                                    .transition_transform()
                                                    .duration(move || {
                                                        if reduce_motion() {
                                                            0.0_f32
                                                        } else {
                                                            0.06
                                                        }
                                                    }),
                                            ),
                                    ),
                            )
                    },
                )),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ReminderItem;
    use crate::storage::AppState;

    #[component]
    fn CardHost() -> impl IntoElement {
        let app = AppCtx::new(AppState {
            items: vec![ReminderItem::new("喝水", "droplet", 1800)],
            ..Default::default()
        });
        view()
            .w(px(320.0))
            .h(px(350.0))
            .child(crate::views::card::render(app))
    }

    #[component]
    fn MultiCardHost() -> impl IntoElement {
        let app = AppCtx::new(AppState {
            items: vec![
                ReminderItem::new("喝水", "droplet", 1800),
                ReminderItem::new("吃药", "pill", 3600),
            ],
            ..Default::default()
        });
        view()
            .w(px(320.0))
            .h(px(350.0))
            .child(
                nsh_base::button()
                    .accessibility_label("连续删除两项")
                    .on_press(move || {
                        let items = app.items.get();
                        if items.len() >= 2 {
                            let id1 = items[0].id;
                            let id2 = items[1].id;
                            app.delete(id1);
                            app.delete(id2);
                        }
                    })
                    .child(text("连续删除两项")),
            )
            .child(crate::views::card::render(app))
    }

    /// 撤销条不是「当场消失」：撤销后条目回来，撤销条先播退场再卸载。
    #[test]
    fn test_toast_leaves_after_its_exit() {
        use nsh_engine::{EngineConfig, Points, Root, Size};
        use nsh_testing::Headless;

        let config = EngineConfig {
            viewport: Size::new(Points(320.0), Points(350.0)),
            ..EngineConfig::new(Root::of::<CardHost>())
        };
        let mut app = Headless::start(config).unwrap();
        app.wait_for("喝水").unwrap();

        // 删除 → 撤销条出现
        app.press("更多操作").unwrap();
        app.wait_for("删除提醒").unwrap();
        app.press("删除提醒").unwrap();
        app.wait_for("已删除「喝水」").unwrap();

        // 撤销 → 条目回来，撤销条播完退场后消失
        app.press("撤销").unwrap();
        app.wait_for("喝水").unwrap();
        app.wait_for_gone("已删除「喝水」", std::time::Duration::from_secs(2))
            .unwrap();
    }

    /// 多层堆叠：连续删除形成多张卡片，每张卡片均可独立撤销。
    #[test]
    fn test_consecutive_deletes_and_sequential_undo() {
        use nsh_engine::{EngineConfig, Points, Root, Size};
        use nsh_testing::Headless;

        let config = EngineConfig {
            viewport: Size::new(Points(320.0), Points(350.0)),
            ..EngineConfig::new(Root::of::<MultiCardHost>())
        };
        let mut app = Headless::start(config).unwrap();
        app.wait_for("喝水").unwrap();
        app.wait_for("吃药").unwrap();

        // 点击测试按钮触发连续删除两项
        app.press("连续删除两项").unwrap();

        // 此时两张卡片均已进入 ToastStack（堆叠层）
        app.wait_for("已删除「吃药」").unwrap();
        app.wait_for("已删除「喝水」").unwrap();

        // 撤销顶层的「吃药」
        app.press("撤销 >> nth=1").unwrap();
        app.wait_for("吃药").unwrap();
        app.wait_for_gone("已删除「吃药」", std::time::Duration::from_secs(2)).unwrap();

        // 底层的「喝水」依然在 stack 中，点击撤销恢复「喝水」
        app.press("撤销").unwrap();
        app.wait_for("喝水").unwrap();

        // 全部撤销后，卡片堆全部退场
        app.wait_for_gone("已删除「喝水」", std::time::Duration::from_secs(2)).unwrap();
    }

    /// 撤销卡片进入退场后还会在 manager 里留一个退场时长，这段时间重复点击
    /// 「撤销」不能把同一条提醒恢复两次（否则列表里会出现两条同 id 的记录并落盘）。
    #[test]
    fn test_undo_twice_restores_once() {
        use nsh_engine::{EngineConfig, Points, Root, Size};
        use nsh_testing::Headless;

        let config = EngineConfig {
            viewport: Size::new(Points(320.0), Points(350.0)),
            ..EngineConfig::new(Root::of::<CardHost>())
        };
        let mut app = Headless::start(config).unwrap();
        app.wait_for("喝水").unwrap();
        app.press("更多操作").unwrap();
        app.wait_for("删除提醒").unwrap();
        app.press("删除提醒").unwrap();
        app.wait_for("已删除「喝水」").unwrap();

        // 第一次恢复；退场期间的第二次点击必须是无操作
        app.press("撤销").unwrap();
        let _ = app.press("撤销");
        app.wait_for("喝水").unwrap();
        assert_eq!(
            app.screen().count("喝水").unwrap(),
            1,
            "重复撤销把同一条提醒恢复了两次"
        );
    }
}
