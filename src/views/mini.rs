//! 迷你胶囊形态（Mini Pill / Dynamic Island）：仅 196×44，显示最优先提醒，
//! 支持一键打卡、一键展开为完整卡片。

use nsh_base::theme;
use nsh_core::prelude::*;

use crate::app::AppCtx;
use crate::icons;
use crate::localtime;
use crate::theme_mode::{ACCENT_GOLD_STRONG, OVERDUE_FG};
use crate::ui;

/// 迷你胶囊挂件形态（Mini Pill / Dynamic Island）：仅 220x44 像素，支持全屏自由拖动、显示最优先提醒、一键完成打卡与一键展开
pub fn render(app: AppCtx) -> impl IntoElement {
    let AppCtx {
        items,
        is_all_paused,
        ..
    } = app;
    let on_expand = move || app.enter_full();
    let on_save = move || app.save();
    let active_item_info = computed(move || {
        items.with(|list| {
            if list.is_empty() {
                return None;
            }
            // 优先展示第一个已到期的任务
            if let Some((idx, it)) = list.iter().enumerate().find(|(_, it)| it.is_expired()) {
                return Some((idx, it.clone(), list.len()));
            }
            // 其次展示剩余时间最短的未暂停任务
            let min_active = list
                .iter()
                .enumerate()
                .filter(|(_, it)| !it.is_paused)
                .min_by_key(|(_, it)| it.remaining_secs);
            if let Some((idx, it)) = min_active {
                return Some((idx, it.clone(), list.len()));
            }
            // 否则展示列表第一项
            Some((0, list[0].clone(), list.len()))
        })
    });

    view()
        .window_drag(true) // 按住胶囊任意空白处可随意拖动
        .w_full()
        .h_full()
        .px(px(8.0))
        .py(px(4.0))
        .flex_row()
        .items_center()
        .justify_between()
        // 左侧微型环形进度图标
        .child(
            view()
                .w(px(28.0))
                .h(px(28.0))
                .items_center()
                .justify_center()
                .text_color(move || theme().foreground)
                .child(
                    svg(move || {
                        if let Some((_, it, _)) = active_item_info.get() {
                            let is_exp = it.is_expired();
                            let color = icons::get_item_color(&it.icon);
                            let ratio = if is_exp { 1.0 } else { it.progress_ratio() };
                            let markup = icons::mini_ring_progress_svg(&it.icon, ratio, color);
                            nsh_core::element::ImageSrc::svg_markup(&markup)
                        } else {
                            let markup = icons::mini_ring_progress_svg("bell", 1.0, "#eab308");
                            nsh_core::element::ImageSrc::svg_markup(&markup)
                        }
                    })
                    .w(px(28.0))
                    .h(px(28.0)),
                ),
        )
        // 中间文本信息（点击文本也可展开回卡片）
        .child(
            view()
                .window_drag(true)
                .flex_1()
                .min_w(px(0.0))
                .h_full()
                .justify_center()
                .px(px(6.0))
                .child(
                    view()
                        .w_full()
                        .min_w(px(0.0))
                        .flex_col()
                        .justify_center()
                        .gap(px(1.0))
                        // 上行：任务名称与多项小指示
                        .child(
                            view()
                                .w_full()
                                .min_w(px(0.0))
                                .flex_row()
                                .items_center()
                                .gap(px(4.0))
                                .child(
                                    text(move || {
                                        if let Some((_, it, _)) = active_item_info.get() {
                                            it.name
                                        } else {
                                            "暂无提醒".to_string()
                                        }
                                    })
                                    .text_size(px(12.0))
                                    .text_color(move || theme().foreground)
                                    .truncate(),
                                )
                                .child(when(
                                    move || {
                                        active_item_info.with(|info| {
                                            info.as_ref().map_or(false, |(_, _, total)| *total > 1)
                                        })
                                    },
                                    move || {
                                        text(move || {
                                            active_item_info.with(|info| {
                                                info.as_ref().map_or(
                                                    "".to_string(),
                                                    |(idx, _, total)| {
                                                        format!("{}/{}", idx + 1, total)
                                                    },
                                                )
                                            })
                                        })
                                        .flex_shrink_0()
                                        .text_size(px(10.5))
                                        .text_color(move || theme().muted_foreground)
                                    },
                                )),
                        )
                        // 下行：倒计时时间或警示状态
                        .child(
                            view().flex_row().items_center().gap(px(4.0)).child(
                                text(move || {
                                    if is_all_paused.get() {
                                        "已暂停".to_string()
                                    } else if let Some((_, it, _)) = active_item_info.get() {
                                        if it.is_expired() {
                                            "⚠️ 到期打卡!".to_string()
                                        } else {
                                            it.format_remaining()
                                        }
                                    } else {
                                        "--:--".to_string()
                                    }
                                })
                                .text_size(px(11.0))
                                .text_color(move || {
                                    if is_all_paused.get() {
                                        theme().muted_foreground
                                    } else if active_item_info.with(|info| {
                                        info.as_ref().map_or(false, |(_, it, _)| it.is_expired())
                                    }) {
                                        OVERDUE_FG
                                    } else {
                                        ACCENT_GOLD_STRONG
                                    }
                                }),
                            ),
                        ),
                ),
        )
        // 右侧操作按钮组（快捷打卡、展开为卡片）
        .child(
            view()
                .window_drag(false) // 按钮区域不触发拖拽
                .flex_shrink_0()
                .flex_row()
                .items_center()
                .gap(px(4.0))
                // 快捷打卡对勾按钮（特别是在到期时一键打卡；无任务时处于禁用态）
                .child(
                    view()
                        .child(ui::icon_button(
                            "打卡完成",
                            move || {
                                active_item_info.with(|info| {
                                    info.as_ref().map_or(false, |(_, it, _)| it.is_expired())
                                })
                            },
                            move || active_item_info.get().is_none(),
                            move || {
                                if let Some((idx, _, _)) = active_item_info.get() {
                                    let now = localtime::now();
                                    items.update(|list| {
                                        if idx < list.len() {
                                            list[idx].complete(&now);
                                        }
                                    });
                                    on_save();
                                }
                            },
                            icons::icon_svg(icons::ICON_CHECK, 12.0, move || {
                                theme().muted_foreground
                            }),
                        ))
                        .tooltip(ui::tooltip_bubble("完成当前打卡")),
                )
                // 展开为大卡片按钮
                .child(
                    view()
                        .test_id("expand-mini-btn")
                        .child(ui::icon_button(
                            "展开为卡片",
                            || false,
                            || false,
                            on_expand,
                            icons::icon_svg(icons::ICON_EXPAND, 11.0, move || {
                                theme().muted_foreground
                            }),
                        ))
                        .tooltip(ui::tooltip_bubble("展开为完整卡片")),
                ),
        )
}
