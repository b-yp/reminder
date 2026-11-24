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
use crate::updater::{self, Status};

use super::AppCtx;

/// 挂载全部应用级副作用。`App()` 每个实例调用一次。
pub fn install(app: AppCtx) {
    install_tray(app);
    install_close_handler(app);
    install_theme_effect(app);
    install_opacity_effect(app);
    install_hotkey_effect(app);
    install_tick(app);
    install_update(app);
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
                // 到点：先播退场动画，`leave_toast` 播完再卸载
                app.leave_toast();
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

/// 应用更新：后台线程的消息回到引擎线程、启动后静默查一次、退出应用时替换。
///
/// 取清单、下载、校验、解包都在 `updater` 自己的线程里跑；这里只按 100ms 排空结果，
/// 提醒的每秒心跳不会被网络拖慢。替换发生在退出时（托盘的「退出」、⌘Q、系统注销都走
/// 同一条路），所以点击「退出并更新」和平时退出对用户是同一件事。
fn install_update(app: AppCtx) {
    // 开发运行没有打包配置，也就没有可替换的安装：不连管道、不查网络
    if matches!(app.update.get(), Status::Unavailable) {
        return;
    }

    let events = stored(Some(updater::connect()));
    // 已解包校验、摆在应用旁边的新版本：留住它，Drop 会把摆好的目录删掉
    let staged = stored(None::<nsh_updater::install::Staged>);
    // 排空定时器自己也要留住：`Spawned` 一丢，任务就取消了
    let pump = stored(None::<Spawned<()>>);

    // 上次替换没删掉的旧文件在这里清掉：一启动就做，不依赖这次有没有联网
    updater::clean_leftovers();

    // 这个 effect 只读两个「开关」：是否在跑、是否要装。**不要**在这里直接读整个
    // `Status`：effect 每次重跑都会先取消它上一次运行里创建的定时器，而下载进度是
    // 「Downloading → Downloading」的连续写入——直接读状态的话，第一条进度消息就会
    // 把刚挂上的 100ms 排空定时器取消，界面从此停在 0%（下载其实还在后台跑，
    // 消息全堆在 channel 里没人取）。两个 computed 只在开关真的翻转时才通知。
    let busy = computed(move || app.update.get().is_busy());
    let installing = computed(move || matches!(app.update.get(), Status::Installing));

    // 只在检查 / 下载时开 100ms 的排空定时器：空闲的挂件不该每秒醒十次
    effect(move || {
        // 上一次运行挂的定时器已随这次重跑被框架取消，句柄不能再当成「还在跑」
        if let Some(previous) = pump.with(Option::take) {
            previous.cancel();
        }
        // 用户点了「重启并更新」：现在换文件，换完让应用自己拉起新版本
        if installing.get() {
            match staged.try_with(|slot| slot.take()).flatten() {
                Some(pending) => match updater::install_now(pending) {
                    Ok(()) => nsh_core::element::desktop::restart(),
                    Err(message) => app.update.set(Status::Failed(message)),
                },
                // 理论上到不了：Ready 只会在刚摆好新版本时出现
                None => app.update.set(Status::Failed("更新文件已失效，请重新下载".to_owned())),
            }
            return;
        }
        if !busy.get() {
            return;
        }
        pump.replace(Some(set_interval(Duration::from_millis(100), move || {
            events.with(|slot| {
                let Some(events) = slot.as_ref() else {
                    return;
                };
                loop {
                    match events.try_recv() {
                        Ok(updater::Event::Checked(result)) => match result {
                            Ok(Some(found)) => app.update.set(Status::Available(found)),
                            Ok(None) => app.update.set(Status::UpToDate),
                            Err(message) => app.update.set(Status::Failed(message)),
                        },
                        Ok(updater::Event::Progress { received, total }) => {
                            // 用户中途又点了一次检查时，旧的下载进度不该覆盖新状态
                            if let Status::Downloading { version, .. } = app.update.get() {
                                app.update.set(Status::Downloading {
                                    version,
                                    received,
                                    total,
                                });
                            }
                        }
                        Ok(updater::Event::Staged(result)) => match result {
                            Ok((new, version)) => {
                                staged.replace(Some(new));
                                app.update.set(Status::Ready { version });
                            }
                            Err(message) => app.update.set(Status::Failed(message)),
                        },
                        Err(std::sync::mpsc::TryRecvError::Empty) => break,
                        // 管道断了（理论上只在进程退出时）：不必空转
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
                    }
                }
            });
        })));
    });

    // 启动后静默查一次：只把结果留在状态里，不弹窗、不抢焦点
    let startup = stored(None::<Spawned<()>>);
    startup.replace(Some(set_timeout(Duration::from_secs(15), move || {
        if matches!(app.update.get(), Status::Idle) {
            app.check_update();
        }
    })));

    // 退出时替换：界面已经收起、数据已经落盘，这里只做一次很快的目录 / 文件替换
    desktop::on_app_quit(move || {
        let pending = staged.try_with(|slot| slot.take()).flatten();
        if let Some(pending) = pending
            && let Err(error) = updater::install_at_quit(pending)
        {
            eprintln!("[reminder] 退出时更新失败：{error}");
        }
        async {}
    });
}
