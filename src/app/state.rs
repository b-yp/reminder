//! `App()` 的挂载副作用：托盘、关闭拦截、主题 / 不透明度联动、热键注册、每秒心跳。
//!
//! 这些是「应用级」的行为，不属于任何单个视图；放在一处后，`App()` 本身只剩
//! 状态装配与根视图组合。

use std::time::Duration;

use nsh_core::element::desktop;
use nsh_core::prelude::*;

use crate::alarm::{alarm_label, trigger_reminder_alarm};
use crate::constants::LOGO;
use crate::hotkey::{self, HotkeyStatus};

use super::AppCtx;

/// 挂载全部应用级副作用。`App()` 每个实例调用一次。
pub fn install(app: AppCtx) {
    install_tray(app);
    install_close_handler(app);
    install_theme_effect(app);
    install_opacity_effect(app);
    install_hotkey_effect(app);
    install_tick(app);
}

/// 系统托盘：左键唤回窗口，右键菜单是唯一真正的退出入口。
/// 第一项跟随窗口可见性在「显示窗口 / 隐藏窗口」之间切换；退出项只写「退出」。
fn install_tray(app: AppCtx) {
    let window_visible = computed(move || {
        desktop::main_window()
            .state()
            .is_some_and(|state| state.visible)
    });
    effect(move || {
        let visible = window_visible.get();
        desktop::set_tray(Some(
            desktop::tray()
                .icon(LOGO)
                .tooltip("reminder 提醒 · 点击显示窗口")
                .menu([
                    desktop::MenuItem::new(crate::window::tray_window_label(visible)).on_select(
                        move || {
                            if visible {
                                app.save();
                                crate::window::hide_to_tray();
                            } else {
                                desktop::main_window().show();
                                desktop::main_window().focus();
                            }
                        },
                    ),
                    desktop::MenuItem::separator(),
                    desktop::MenuItem::new(crate::window::TRAY_EXIT_LABEL).on_select(move || {
                        app.save();
                        desktop::quit();
                    }),
                ])
                .on_click(|_| {
                    desktop::main_window().show();
                    desktop::main_window().focus();
                }),
        ));
    });
}

/// 顶栏叉号、Alt+F4、任务栏关闭等请求先到这里：保存后收进托盘，提醒计时继续。
/// 系统注销 / 关机由平台结束进程；窗口关闭本身永远不退出应用。
fn install_close_handler(app: AppCtx) {
    desktop::main_window().on_close_press(move |_reason| {
        app.save();
        crate::window::hide_to_tray();
    });
}

/// 主题与底层平台/渲染器联动：切换 NSH 官方色彩方案与系统外观，并在后续改动时自动保存。
fn install_theme_effect(app: AppCtx) {
    let theme_initialized = stored(false);
    effect(move || {
        let mode = app.theme_mode.get();
        nsh_core::element::set_color_scheme(mode.to_appearance());
        if !theme_initialized.get() {
            theme_initialized.replace(true);
        } else {
            app.save();
        }
    });
}

/// 窗口不透明度响应式联动：opacity 变化时实时调用原生窗口 API，并在后续改动时自动保存。
fn install_opacity_effect(app: AppCtx) {
    let opacity_initialized = stored(false);
    effect(move || {
        let val = app.opacity.get();
        crate::window::set_window_opacity(val);
        if !opacity_initialized.get() {
            opacity_initialized.replace(true);
        } else {
            app.save();
        }
    });
}

/// 全局热键安装：组合键或开关变化时先释放旧按键再按新配置注册，
/// 系统接受后热键可隐藏 / 呼出悬浮窗；注册结果（生效 / 被占用）回写到设置抽屉的状态徽章。
fn install_hotkey_effect(app: AppCtx) {
    let hotkey_generation = stored(0u64);
    effect(move || {
        let enabled = app.hotkey_enabled.get();
        let combo = app.hotkey.get();
        // 每次重新注册都换一代，旧的注册结果回来时不再覆盖新状态
        let generation = hotkey_generation.get() + 1;
        hotkey_generation.replace(generation);

        if !os().is_desktop() {
            app.hotkey_status.set(HotkeyStatus::Unsupported);
            return;
        }
        desktop::unregister_all_global_shortcuts();
        if !enabled {
            app.hotkey_status.set(HotkeyStatus::Disabled);
            return;
        }
        app.hotkey_status.set(HotkeyStatus::Pending);
        hotkey::register(combo, generation, hotkey_generation, app.hotkey_status, 3);
    });
}

/// 每秒驱动倒计时更新与到期提醒，检测窗口拖动并定期同步时间戳（提供离线流逝时间基准）与位置变动。
fn install_tick(app: AppCtx) {
    let tick_count = std::rc::Rc::new(std::cell::Cell::new(0u64));
    let tick_count_cloned = tick_count.clone();
    let tick_timer = stored(None::<Spawned<()>>);
    tick_timer.replace(Some(set_interval(Duration::from_secs(1), move || {
        let count = tick_count_cloned.get() + 1;
        tick_count_cloned.set(count);

        // 1. 窗口摆放变动检测与自动持久化（拖拽释放后 1 秒内保存）
        let mut pos_changed = false;
        if let Some(state) = desktop::main_window().state() {
            if app.last_saved_placement.get() != Some(state.placement) {
                app.last_saved_placement.set(Some(state.placement));
                pos_changed = true;
            }
        }

        // 2. 撤销窗口：时钟推进；到期只收起提示（删除早已落盘）
        app.clock.set(crate::localtime::unix_now());
        if let Some(mut pending) = app.pending_delete.get() {
            if app.is_toast_hovered.get() {
                // 鼠标悬停在 Toast 上时顺延 deadline，暂停倒计时
                pending.deadline = pending.deadline.saturating_add(1);
                app.pending_delete.set(Some(pending));
            } else if crate::localtime::unix_now() >= pending.deadline {
                app.is_toast_hovered.set(false);
                app.pending_delete.set(None);
            }
        }

        // 3. 倒计时与提醒驱动（定点提醒按本地墙钟判断，间隔提醒逐秒递减）
        let mut state_changed = pos_changed;
        if !app.is_all_paused.get() {
            let now = crate::localtime::now();
            let mut newly_expired = Vec::new();
            app.items.update(|list| {
                for item in list.iter_mut() {
                    if item.tick(&now) {
                        newly_expired.push(item.clone());
                        state_changed = true;
                    }
                }
            });
            for item in newly_expired {
                trigger_reminder_alarm(&alarm_label(&item));
            }
        }

        // 每 15 秒同步一次（确保离线补偿时间戳最新），或在窗口移动/任务到期时立即同步
        if count % 15 == 0 || state_changed {
            app.save();
        }
    })));
}
