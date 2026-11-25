//! 顶部 Header：左侧 App 微标与标题，右侧静默的挂件操作按钮组
//! （新建、暂停、偏好设置、收起为胶囊、收进托盘）。

use nsh_base::theme;
use nsh_core::prelude::*;

use crate::app::AppCtx;
use crate::constants::LOGO;
use crate::icons;
use crate::theme_mode::{ACCENT_GOLD, ACCENT_GOLD_STRONG};
use crate::ui;
use crate::ui::tokens::{font, radius, space};
use crate::updater::Status as UpdateStatus;
use crate::window::hide_to_tray;

/// 顶部 Header：左侧金黄微标与标题，右侧挂件操作按钮组（新建、暂停、偏好设置、收起为胶囊、收进托盘）。
///
/// 右侧都是静默图标：只有「对应的抽屉正开着」才浮出金黄底，窗口里的内容才是主角。
/// 提醒数量只在底栏出现一次，避免和顶栏重复。
pub fn render(app: AppCtx) -> impl IntoElement {
    let AppCtx {
        is_all_paused,
        show_add,
        show_settings,
        update,
        ..
    } = app;
    let on_enter_mini = move || app.enter_mini();
    let on_save = move || app.save();

    view()
        .window_drag(true)
        .flex_row()
        .items_center()
        .justify_between()
        .pb(px(space::TIGHT))
        // 左侧：微标 + 标题
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(space::SNUG))
                .child(
                    // App Logo 微标
                    image(LOGO).size(px(22.0)).rounded(px(radius::LG)),
                )
                .child(
                    text("reminder 提醒")
                        .text_size(px(font::TITLE))
                        .text_color(move || theme().foreground),
                ),
        )
        // 右侧挂件专属操作按钮组：新建提醒、暂停/恢复全部、偏好设置、收起胶囊、收进托盘
        .child(
            view()
                .window_drag(false) // 按钮区域不触发窗口拖拽
                .flex_row()
                .items_center()
                .gap(px(space::TIGHT))
                // 新建提醒
                .child(
                    view()
                        .test_id("add-toggle")
                        .child(ui::icon_button(
                            "新建提醒",
                            ui::IconTone::Ghost,
                            move || show_add.get(),
                            || false,
                            move || app.toggle_add(),
                            icons::dynamic_icon_svg(
                                move || icons::ICON_PLUS,
                                12.0,
                                move || {
                                    if show_add.get() {
                                        ACCENT_GOLD_STRONG
                                    } else {
                                        theme().muted_foreground
                                    }
                                },
                            ),
                        ))
                        .tooltip(ui::tooltip_bubble("新建提醒")),
                )
                // 暂停 / 播放全部
                .child(
                    view()
                        .child(ui::icon_button(
                            "暂停或恢复全部提醒",
                            ui::IconTone::Ghost,
                            move || is_all_paused.get(),
                            || false,
                            move || {
                                is_all_paused.update(|v| *v = !*v);
                                on_save();
                            },
                            when(
                                move || is_all_paused.get(),
                                move || icons::icon_svg(icons::ICON_PLAY, 11.0, ACCENT_GOLD_STRONG),
                            )
                            .otherwise(move || {
                                icons::icon_svg(icons::ICON_PAUSE, 11.0, move || {
                                    theme().muted_foreground
                                })
                            }),
                        ))
                        .tooltip(ui::dynamic_tooltip_bubble(move || {
                            if is_all_paused.get() {
                                "恢复全部提醒"
                            } else {
                                "暂停全部提醒"
                            }
                        })),
                )
                // 偏好设置（发现新版本或待重启更新时，右上角亮起金色角标）
                .child(
                    view()
                        .test_id("settings-toggle")
                        .relative()
                        .child(ui::icon_button(
                            "偏好设置",
                            ui::IconTone::Ghost,
                            move || show_settings.get(),
                            || false,
                            move || app.toggle_settings(),
                            icons::dynamic_icon_svg(
                                move || icons::ICON_SETTINGS,
                                12.0,
                                move || {
                                    if show_settings.get() {
                                        ACCENT_GOLD_STRONG
                                    } else {
                                        theme().muted_foreground
                                    }
                                },
                            ),
                        ))
                        .child(when(
                            move || update.get().has_notice(),
                            move || {
                                view()
                                    .test_id("update-badge")
                                    .absolute()
                                    .top(px(2.0))
                                    .right(px(2.0))
                                    .w(px(7.0))
                                    .h(px(7.0))
                                    .rounded(px(99.0))
                                    .bg(ACCENT_GOLD)
                                    .border_1()
                                    .border_color(move || theme().background)
                                    .pointer_events(PointerEvents::None)
                            },
                        ))
                        .tooltip(ui::dynamic_tooltip_bubble(move || match update.get() {
                            UpdateStatus::Available(_) => "偏好设置 · 发现新版本",
                            UpdateStatus::Ready { .. } => "偏好设置 · 新版本待重启",
                            _ => "偏好设置",
                        })),
                )
                // 收起为迷你灵动岛胶囊
                .child(
                    view()
                        .child(ui::icon_button(
                            "收起为迷你胶囊",
                            ui::IconTone::Ghost,
                            || false,
                            || false,
                            on_enter_mini,
                            icons::icon_svg(icons::ICON_MINIMIZE, 11.0, move || {
                                theme().muted_foreground
                            }),
                        ))
                        .tooltip(ui::tooltip_bubble("收起为迷你胶囊")),
                )
                // 收进托盘（不是退出）：提醒计时与托盘图标继续运行，退出只放在托盘菜单里
                .child(
                    view()
                        .test_id("hide-to-tray-btn")
                        .child(ui::icon_button(
                            "收进托盘，继续提醒",
                            ui::IconTone::Ghost,
                            || false,
                            || false,
                            move || {
                                on_save();
                                hide_to_tray();
                            },
                            icons::icon_svg(icons::ICON_X, 11.0, move || theme().muted_foreground),
                        ))
                        .tooltip(ui::tooltip_bubble("收进托盘，继续提醒")),
                ),
        )
}
