//! 底部撤销 Toast：NSH `toast()` 语义根（role=alert、支持滑动划走），
//! 悬浮在底栏上方，带任务名、撤销按钮与悬停暂停的倒计时进度条。

use nsh_base::{SwipeDirection, theme, toast};
use nsh_core::prelude::*;

use crate::app::{AppCtx, PendingDelete};
use crate::constants::UNDO_SECONDS;
use crate::icons;
use crate::theme_mode::ACCENT_GOLD;
use crate::ui;

/// 底部浮层撤销 Toast：基于 NSH `toast()` 语义根（role=alert、支持滑动划走），
/// 悬浮在底栏上方，带有柔和阴影、精致边框、垃圾桶图标、任务名、撤销按钮及动态倒计时。
pub fn render(app: AppCtx, pending: PendingDelete) -> impl IntoElement {
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
                .swipe_direction(Some(SwipeDirection::Right))
                .on_dismiss_request(move || on_dismiss())
                .pointer_events(PointerEvents::Auto)
                .w_full()
                .px(px(10.0))
                .py(px(6.5))
                .rounded(px(8.0))
                .bg(move || theme().popover)
                .border_1()
                .border_color(move || theme().border)
                .shadow_lg()
                .transition_all()
                .duration(200.0)
                .starting_class("opacity-0 translate-y-3")
                .ending_class("opacity-0 scale-95")
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
                                                .text_size(px(11.0))
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
                                                .text_size(px(10.0))
                                                .text_color(move || theme().muted_foreground),
                                        ),
                                ),
                        )
                        // 底部微细倒计时进度条（Sonner / Apple 风格：平滑流逝，悬停暂停时自动定格）
                        .child(
                            view()
                                .w_full()
                                .h(px(2.0))
                                .rounded_full()
                                .bg(move || theme().muted.opacity(0.40))
                                .child(
                                    view()
                                        .h_full()
                                        .rounded_full()
                                        .bg(ACCENT_GOLD)
                                        .w(move || {
                                            let ratio =
                                                remaining.get() as f32 / UNDO_SECONDS as f32;
                                            px(272.0 * ratio.clamp(0.0, 1.0))
                                        })
                                        .transition_all()
                                        .duration(move || {
                                            if reduce_motion() { 0.0_f32 } else { 300.0_f32 }
                                        }),
                                ),
                        ),
                ),
        )
}
