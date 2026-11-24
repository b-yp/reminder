//! 开机自启设置区：开关直接写系统启动项（Windows 注册表 Run 项 / macOS LaunchAgent /
//! Linux XDG autostart），状态徽章以系统回读结果为准；写入被拒绝时开关退回真实状态并给出原因。

use nsh_base::theme;
use nsh_core::prelude::*;

use crate::app::AppCtx;
use crate::autostart;
use crate::icons;
use crate::theme_mode::{ACCENT_DANGER_FG, ACCENT_GOLD_STRONG};
use crate::ui;
use crate::ui::tokens::font;

/// 开机自启设置区：开关直接写系统启动项（Windows 注册表 Run 项 / macOS LaunchAgent /
/// Linux XDG autostart），状态徽章以系统回读结果为准；写入被拒绝时开关退回真实状态并给出原因
pub fn render(app: AppCtx) -> impl IntoElement {
    let AppCtx {
        autostart_enabled,
        autostart_error,
        ..
    } = app;
    // 桌面（macOS / Windows / Linux）才有登录启动项；Web 与移动端整块置灰
    let supported = autostart::is_supported();
    let current_state = move || {
        autostart::state(
            supported,
            autostart_enabled.get(),
            autostart_error.get().as_deref(),
        )
    };

    view()
        .flex_col()
        .gap(px(4.0))
        .opacity(move || if supported { 1.0_f32 } else { 0.55_f32 })
        // 标题行：电源图标 + 名称 + 启用开关
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
                        .child(icons::icon_svg(icons::ICON_POWER, 12.0, ACCENT_GOLD_STRONG))
                        .child(
                            text("开机自启")
                                .text_size(px(font::TITLE))
                                .text_color(move || theme().foreground),
                        ),
                )
                .child(ui::setting_switch(
                    autostart_enabled,
                    !supported,
                    "开机自启",
                    move |next| match autostart::set_enabled(next) {
                        Ok(()) => {
                            autostart_error.set(None);
                            // 以系统回读为准：登记成功但被系统改写时，开关显示真实状态
                            autostart_enabled.set(autostart::is_enabled());
                        }
                        Err(error) => {
                            autostart_enabled.set(autostart::is_enabled());
                            autostart_error.set(Some(error));
                        }
                    },
                )),
        )
        // 说明行：状态解释 + 状态徽章
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .gap(px(6.0))
                .child(
                    text(move || {
                        autostart::hint(current_state(), autostart_error.get().as_deref())
                    })
                    .text_size(px(font::CAPTION))
                    .text_color(move || {
                        if current_state().is_error() {
                            ACCENT_DANGER_FG
                        } else {
                            theme().muted_foreground
                        }
                    }),
                )
                .child(ui::status_badge(
                    move || match current_state() {
                        autostart::AutostartState::Enabled => ui::BadgeTone::Accent,
                        autostart::AutostartState::Failed => ui::BadgeTone::Danger,
                        _ => ui::BadgeTone::Muted,
                    },
                    move || current_state().label(),
                )),
        )
}
