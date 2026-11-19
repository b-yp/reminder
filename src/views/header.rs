//! 顶部 Header：左侧 App 微标、标题与数量徽章，右侧挂件操作按钮组
//! （新建、暂停、偏好设置、收起为胶囊、收进托盘）。

use nsh_base::theme;
use nsh_core::prelude::*;

use crate::app::AppCtx;
use crate::constants::LOGO;
use crate::icons;
use crate::theme_mode::ACCENT_GOLD_STRONG;
use crate::ui;
use crate::window::hide_to_tray;

/// 顶部 Header：左侧金黄微标与标题，右侧挂件操作按钮组（新建、暂停、偏好设置、收起为胶囊、收进托盘）
pub fn render(app: AppCtx) -> impl IntoElement {
    let AppCtx {
        items,
        is_all_paused,
        show_add,
        show_settings,
        ..
    } = app;
    let on_enter_mini = move || app.enter_mini();
    let on_save = move || app.save();
    let count_text = computed(move || items.with(|list| format!("{} 项", list.len())));

    view()
        .window_drag(true)
        .flex_row()
        .items_center()
        .justify_between()
        .pb(px(4.0))
        // 左侧：微标 + 标题 + 数量指示
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(
                    // App Logo 微标
                    image(LOGO).size(px(22.0)).rounded(px(5.0)),
                )
                .child(
                    text("reminder 提醒")
                        .text_size(px(13.0))
                        .text_color(move || theme().foreground),
                )
                .child(
                    // 进行中数量指示小徽章
                    view()
                        .px(px(5.0))
                        .py(px(1.0))
                        .rounded(px(10.0))
                        .bg(move || theme().muted)
                        .child(
                            text(move || count_text.get())
                                .text_size(px(10.0))
                                .text_color(move || theme().muted_foreground),
                        ),
                ),
        )
        // 右侧挂件专属操作按钮组：新建提醒、暂停/恢复全部、偏好设置、收起胶囊、收进托盘
        .child(
            view()
                .window_drag(false) // 按钮区域不触发窗口拖拽
                .flex_row()
                .items_center()
                .gap(px(4.0))
                // 新建提醒
                .child(
                    view()
                        .test_id("add-toggle")
                        .child(ui::icon_button(
                            "新建提醒",
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
                // 偏好设置
                .child(
                    view()
                        .test_id("settings-toggle")
                        .child(ui::icon_button(
                            "偏好设置",
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
                        .tooltip(ui::tooltip_bubble("偏好设置")),
                )
                // 收起为迷你灵动岛胶囊
                .child(
                    view()
                        .child(ui::icon_button(
                            "收起为迷你胶囊",
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
