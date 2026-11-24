//! 底部撤销 Toast：NSH `toast()` 语义根（role=alert、支持滑动划走），
//! 悬浮在底栏上方，带任务名、撤销按钮与悬停暂停的倒计时进度条。

use nsh_base::{SwipeDirection, ToastTransitionStatus, theme, toast};
use nsh_core::prelude::*;

use crate::app::{AppCtx, PendingDelete};
use crate::constants::UNDO_SECONDS;
use crate::icons;
use crate::theme_mode::ACCENT_GOLD;
use crate::ui;
use crate::ui::tokens::{font, motion, radius};

/// 底部浮层撤销 Toast：基于 NSH `toast()` 语义根（role=alert、支持滑动划走），
/// 悬浮在底栏上方，带有柔和阴影、精致边框、垃圾桶图标、任务名、撤销按钮及动态倒计时。
///
/// `leaving` 为真时进入 `ending` 阶段：`card` 会在这个阶段继续挂载 `TOAST_EXIT_MS`，
/// 播完退场动画再卸载，而不是当场消失。
pub fn render(app: AppCtx, pending: PendingDelete, leaving: bool) -> impl IntoElement {
    let AppCtx {
        clock,
        is_toast_hovered: is_hovered,
        ..
    } = app;
    let on_undo = move || app.undo();
    let on_dismiss = move || app.dismiss_toast();
    let name = pending.item.name.clone();
    let remaining = computed(move || {
        pending
            .deadline
            .saturating_sub(clock.get())
            .min(UNDO_SECONDS)
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
            toast()
                .accessibility_label("已删除，可撤销")
                .transition_status(if leaving {
                    ToastTransitionStatus::Ending
                } else {
                    ToastTransitionStatus::Starting
                })
                .swipe_direction(Some(SwipeDirection::Right))
                .on_dismiss_request(move || on_dismiss())
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
                        .on_mouse_enter(move |_: &MouseEvent| is_hovered.set(true))
                        .on_mouse_leave(move |_: &MouseEvent| is_hovered.set(false))
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
                                // 右侧：撤销按钮 + 倒计时
                                .child(
                                    view()
                                        .flex_row()
                                        .items_center()
                                        .gap(px(6.0))
                                        .child(ui::undo_button("撤销", move || on_undo()))
                                        .child(
                                            text(move || format!("{}s", remaining.get()))
                                                .text_size(px(font::CAPTION))
                                                .text_color(move || theme().muted_foreground),
                                        ),
                                ),
                        )
                        // 底部微细倒计时进度条（Sonner / Apple 风格：平滑流逝，悬停暂停时自动定格）
                        // 用 scaleX 而不是宽度：每秒推进只动合成层，不触发布局。
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
                                        .scale_x(move || {
                                            remaining.get() as f32 / UNDO_SECONDS as f32
                                        })
                                        .transition_transform()
                                        .duration(move || {
                                            if reduce_motion() {
                                                0.0_f32
                                            } else {
                                                motion::BASE
                                            }
                                        }),
                                ),
                        ),
                ),
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
}
