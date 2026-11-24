//! 全局热键设置区：键帽提示当前组合键、开关控制是否监听、状态徽章说明注册结果，
//! 「换一组」在候选组合键间轮换（组合键被其他应用占用时用它换一个）。

use nsh_base::theme;
use nsh_core::prelude::*;

use crate::app::AppCtx;
use crate::hotkey::{self, HotkeyStatus};
use crate::icons;
use crate::theme_mode::{ACCENT_DANGER_FG, ACCENT_GOLD_STRONG};
use crate::ui;
use crate::ui::tokens::{font, radius};

/// 全局热键设置区：键帽提示当前组合键、开关控制是否监听、状态徽章说明注册结果，
/// 「换一组」在候选组合键间轮换（组合键被其他应用占用时用它换一个）
pub fn render(app: AppCtx) -> impl IntoElement {
    let AppCtx {
        hotkey,
        hotkey_enabled,
        hotkey_status,
        ..
    } = app;
    let on_save = move || app.save();
    // 桌面（macOS / Windows / Linux）才有系统级热键；Web 与移动端显示为不可用
    let supported = os().is_desktop();
    let keycaps = computed(move || hotkey::keycaps(&hotkey.get()));

    view()
        .flex_col()
        .gap(px(4.0))
        // 标题行：键盘图标 + 名称 + 启用开关
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        .child(icons::icon_svg(
                            icons::ICON_KEYBOARD,
                            12.0,
                            ACCENT_GOLD_STRONG,
                        ))
                        .child(
                            text("全局热键")
                                .text_size(px(font::TITLE))
                                .text_color(move || theme().foreground),
                        ),
                )
                .child(ui::setting_switch(
                    hotkey_enabled,
                    !supported,
                    "启用全局热键",
                    move |next| {
                        hotkey_enabled.set(next);
                        on_save();
                    },
                )),
        )
        // 键帽行：逐个按键 + 注册状态徽章
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .gap(px(6.0))
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(3.0))
                        .opacity(move || {
                            if hotkey_enabled.get() {
                                1.0_f32
                            } else {
                                0.45_f32
                            }
                        })
                        .child(each(
                            keycaps,
                            |cap| cap.clone(),
                            |cap, _| {
                                view()
                                    .px(px(4.0))
                                    .py(px(1.0))
                                    .rounded(px(radius::SM))
                                    .bg(move || theme().secondary)
                                    .border_1()
                                    .border_color(move || theme().border)
                                    .child(
                                        text(move || cap.get())
                                            .text_size(px(font::CAPTION))
                                            .text_color(move || theme().muted_foreground),
                                    )
                            },
                        )),
                )
                .child(ui::status_badge(
                    move || match hotkey_status.get() {
                        HotkeyStatus::Active => ui::BadgeTone::Accent,
                        HotkeyStatus::Taken => ui::BadgeTone::Danger,
                        _ => ui::BadgeTone::Muted,
                    },
                    move || hotkey_status.get().label(),
                )),
        )
        // 说明行：状态解释 + 「换一组」
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .gap(px(6.0))
                .child(
                    text(move || hotkey_status.get().hint())
                        .text_size(px(font::CAPTION))
                        .text_color(move || {
                            if hotkey_status.get().is_error() {
                                ACCENT_DANGER_FG
                            } else {
                                theme().muted_foreground
                            }
                        }),
                )
                .child(when(
                    move || supported && hotkey_enabled.get(),
                    move || {
                        ui::text_button("换一组", false, move || {
                            let next = hotkey::next_preset(&hotkey.get());
                            hotkey.set(next.to_string());
                            on_save();
                        })
                    },
                )),
        )
}
