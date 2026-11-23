//! 偏好设置抽屉：外观主题、窗口不透明度、全局热键、开机自启、应用更新。

mod autostart_setting;
mod hotkey_setting;
mod update_setting;

use nsh_base::{SliderValue, collapsible, slider, theme};
use nsh_core::prelude::*;

use crate::app::AppCtx;
use crate::icons;
use crate::theme_mode::{ACCENT_GOLD, ACCENT_GOLD_BG, ACCENT_GOLD_STRONG, ThemeMode};
use crate::ui;
use crate::window::{MIN_OPACITY, clamp_opacity};

/// 偏好设置抽屉（点击齿轮展开，支持外观主题切换、窗口不透明度调节、全局热键与开机自启）
pub fn render(app: AppCtx) -> impl IntoElement {
    let AppCtx {
        show_settings: show,
        opacity,
        theme_mode,
        ..
    } = app;
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
            // 1. 外观主题切换控制区 (浅色模式 / 深色模式 / 跟随系统)
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
                                view()
                                    .flex_row()
                                    .items_center()
                                    .gap(px(6.0))
                                    .child(icons::dynamic_icon_svg(
                                        move || match theme_mode.get() {
                                            ThemeMode::Light => icons::ICON_SUN,
                                            ThemeMode::Dark => icons::ICON_MOON,
                                            ThemeMode::System => icons::ICON_LAPTOP,
                                        },
                                        12.0,
                                        ACCENT_GOLD_STRONG,
                                    ))
                                    .child(
                                        text("外观主题")
                                            .text_size(px(11.5))
                                            .text_color(move || theme().foreground),
                                    ),
                            )
                            .child(
                                view()
                                    .px(px(6.0))
                                    .py(px(2.0))
                                    .rounded(px(4.0))
                                    .bg(ACCENT_GOLD_BG)
                                    .child(
                                        text(move || theme_mode.get().label())
                                            .text_size(px(10.5))
                                            .text_color(ACCENT_GOLD_STRONG),
                                    ),
                            ),
                    )
                    .child(ui::choice_group(
                        [
                            (ThemeMode::Light, "浅色模式", icons::ICON_SUN),
                            (ThemeMode::Dark, "深色模式", icons::ICON_MOON),
                            (ThemeMode::System, "跟随系统", icons::ICON_LAPTOP),
                        ]
                        .into_iter()
                        .map(|(mode, label, icon)| {
                            // 三选一，底座 nsh/base 的 toggle_group + toggle
                            ui::segment_toggle(
                                label,
                                icon,
                                move || theme_mode.get() == mode,
                                move |next: bool| {
                                    if next {
                                        theme_mode.set(mode);
                                    }
                                },
                            )
                        }),
                    )),
            )
            // 细分割线
            .child(
                view()
                    .w_full()
                    .my(px(2.0))
                    .border_t_1()
                    .border_color(move || theme().border),
            )
            // 2. 窗口不透明度调节区 (纯粹专注的 NSH 官方原生 slider 组件)
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
                                view()
                                    .flex_row()
                                    .items_center()
                                    .gap(px(6.0))
                                    .child(icons::icon_svg(
                                        icons::ICON_OPACITY,
                                        12.0,
                                        ACCENT_GOLD_STRONG,
                                    ))
                                    .child(
                                        text("窗口不透明度")
                                            .text_size(px(11.5))
                                            .text_color(move || theme().foreground),
                                    ),
                            )
                            .child(
                                view()
                                    .px(px(6.0))
                                    .py(px(2.0))
                                    .rounded(px(4.0))
                                    .bg(ACCENT_GOLD_BG)
                                    .child(
                                        text(move || format!("{:.0}%", opacity.get() * 100.0))
                                            .text_size(px(11.0))
                                            .text_color(ACCENT_GOLD_STRONG),
                                    ),
                            ),
                    )
                    .child(
                        view().w_full().px(px(2.0)).child(
                            slider()
                                .min(f64::from(MIN_OPACITY) * 100.0)
                                .max(100.0)
                                .step(1.0)
                                .value(move || {
                                    SliderValue::Single((opacity.get() * 100.0).round() as f64)
                                })
                                .on_change(move |v: &SliderValue| {
                                    opacity.set(clamp_opacity((v.start() / 100.0) as f32));
                                })
                                .accessibility_label("窗口不透明度")
                                .track_style(|s| s.w_full().h(px(20.0)).justify_center())
                                .indicator_style(move |s| {
                                    s.h(px(4.0)).rounded_full().bg(theme().muted)
                                })
                                .range_style(move |s| s.rounded_full().bg(ACCENT_GOLD))
                                .thumb_style(move |s| {
                                    s.size(px(13.0))
                                        .rounded_full()
                                        .bg(theme().background)
                                        .border_1()
                                        .border_color(ACCENT_GOLD)
                                }),
                        ),
                    ),
            )
            // 细分割线
            .child(
                view()
                    .w_full()
                    .my(px(2.0))
                    .border_t_1()
                    .border_color(move || theme().border),
            )
            // 3. 全局热键：一键隐藏 / 呼出悬浮窗（可在任意应用中使用）
            .child(hotkey_setting::render(app))
            // 细分割线
            .child(
                view()
                    .w_full()
                    .my(px(2.0))
                    .border_t_1()
                    .border_color(move || theme().border),
            )
            // 4. 开机自启：登录系统后自动运行挂件，开关直接写系统启动项
            .child(autostart_setting::render(app))
            // 细分割线
            .child(
                view()
                    .w_full()
                    .my(px(2.0))
                    .border_t_1()
                    .border_color(move || theme().border),
            )
            // 5. 应用更新：当前版本、检查 / 下载新版本，退出应用时替换
            .child(update_setting::render(app)),
    )
}
