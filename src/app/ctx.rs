//! 应用状态上下文：把 `App()` 里创建的所有响应式信号收进一个 `Copy` 结构体。
//!
//! 视图只接收这一个参数（`fn render(app: AppCtx)`），不再逐层透传
//! `items / is_all_paused / show_add / …` 十几个信号与回调闭包；
//! 写操作也收敛成方法（`app.save()`、`app.delete(id)`、`app.enter_mini()`），
//! 行为只有一份，界面代码只负责组合控件。

use nsh_base::{ToastManager, ToastMotion, ToastOptions, ToastStackState, ToastTransitionStatus};
use nsh_core::desktop::{WindowFrame, WindowPlacement};
use nsh_core::element::desktop;
use nsh_core::prelude::*;

use crate::constants::{MINI_HEIGHT, MINI_WIDTH, UNDO_SECONDS, WINDOW_HEIGHT, WINDOW_WIDTH};
use crate::hotkey::{self, HotkeyStatus};
use crate::model::{ReminderItem, undo_restore_item};
use crate::storage::AppState;
#[cfg(not(test))]
use crate::storage::persist_current;
use crate::theme_mode::ThemeMode;
use crate::updater;

/// 刚删除、还留在底部撤销 Toast 里的提醒（窗口关闭后即为最终删除）。
#[derive(Clone, PartialEq)]
pub struct PendingDelete {
    pub item: ReminderItem,
    pub prev_id: Option<u64>,
    pub next_id: Option<u64>,
    pub remaining_ms: std::rc::Rc<std::cell::Cell<u64>>,
}

/// 撤销 Toast 的运动参数：退场时长与卡片自己的 transition（`motion::BASE`）对齐，
/// 否则 manager 会在退场动画播完之前就把卡片卸载（`ToastMotion::sonner()` 默认是 200ms）。
pub fn toast_motion() -> ToastMotion {
    ToastMotion {
        exit_duration: std::time::Duration::from_millis(crate::ui::tokens::motion::BASE as u64),
        ..ToastMotion::sonner()
    }
}

/// 应用的响应式状态：一个值就能交给任何视图。
///
/// 所有字段都是 `Signal`（`Copy`），所以 `AppCtx` 本身也是 `Copy`，
/// 可以在任意多个 `move` 闭包里按值捕获，不需要 `Rc`。
#[derive(Clone, Copy)]
pub struct AppCtx {
    pub items: Signal<Vec<ReminderItem>>,
    pub is_all_paused: Signal<bool>,
    pub opacity: Signal<f32>,
    pub is_mini: Signal<bool>,
    pub theme_mode: Signal<ThemeMode>,
    pub hotkey: Signal<String>,
    pub hotkey_enabled: Signal<bool>,
    pub hotkey_status: Signal<HotkeyStatus>,
    pub autostart_enabled: Signal<bool>,
    pub autostart_error: Signal<Option<String>>,
    pub last_saved_placement: Signal<Option<WindowPlacement>>,
    pub show_add: Signal<bool>,
    pub show_settings: Signal<bool>,
    /// 空列表入口请求「新建」抽屉复位一次：抽屉已经打开时 `show_add.set(true)` 是空操作，
    /// 靠这个 tick 把页签和草稿强制拉回干净首屏。
    pub add_reset: Signal<u64>,
    /// 官方 ToastStack 状态：记录卡片堆是否被鼠标悬停 / 触碰展开
    pub toast_stack_state: ToastStackState,
    /// 官方 ToastManager：负责卡片堆的层叠生命周期、自动超时与进退场
    pub toast_manager: Stored<ToastManager<u64, PendingDelete>>,
    /// 当前可见的前 3 条 Toast（id, PendingDelete, 进退场状态）
    pub toast_entries: Signal<Vec<(u64, PendingDelete, ToastTransitionStatus)>>,
    /// 倒计时时钟脉冲（驱动卡片上的倒计时数字与细微进度条平滑动画）
    pub toast_tick: Signal<u64>,
    /// 应用更新：检查 / 下载 / 退出时替换的状态
    pub update: Signal<updater::Status>,
    /// 当前版本：以打包进应用的那份配置为准（开发运行退回 crate 版本）
    pub update_version: Signal<String>,
}

impl AppCtx {
    /// 从持久化状态创建全部信号；`AppState` 里的数据在这里被接管一次。
    pub fn new(state: AppState) -> Self {
        // 更新配置只认打包时写进资源目录的那份：开发运行没有它，也就没有可替换的安装
        let (update, update_version) = match updater::bundled() {
            Some((_, config)) => (updater::Status::Idle, config.version),
            None => (
                updater::Status::Unavailable,
                env!("CARGO_PKG_VERSION").to_owned(),
            ),
        };
        Self {
            items: signal(state.items),
            is_all_paused: signal(state.is_all_paused),
            opacity: signal(state.opacity),
            is_mini: signal(state.is_mini),
            theme_mode: signal(state.theme_mode),
            hotkey: signal(hotkey::normalize(&state.hotkey)),
            hotkey_enabled: signal(state.hotkey_enabled),
            hotkey_status: signal(HotkeyStatus::Pending),
            // 开机自启以系统启动项为准：每次启动回读，用户可能在系统设置里改过
            autostart_enabled: signal(crate::autostart::is_enabled()),
            autostart_error: signal(Option::<String>::None),
            last_saved_placement: signal(state.window_placement),
            show_add: signal(false),
            show_settings: signal(false),
            add_reset: signal(0u64),
            toast_stack_state: ToastStackState::new(),
            toast_manager: stored(ToastManager::<u64, PendingDelete>::new(toast_motion())),
            toast_entries: signal(Vec::new()),
            toast_tick: signal(0u64),
            update: signal(update),
            update_version: signal(update_version),
        }
    }

    /// 统一持久化：把当前状态快照落盘（含窗口摆放）。
    ///
    /// 测试构建里绝不落盘：headless 视图测试会真的点击「预设 / 打卡」按钮，
    /// 写到真实用户数据目录既污染用户状态，也会让随后的 `nsh test` 读到脏数据。
    pub fn save(&self) {
        #[cfg(test)]
        return;
        #[cfg(not(test))]
        self.persist();
    }

    #[cfg(not(test))]
    fn persist(&self) {
        let win_placement = desktop::main_window()
            .state()
            .map(|state| state.placement)
            .or_else(|| self.last_saved_placement.get());
        persist_current(
            &self.items.get(),
            self.theme_mode.get(),
            self.opacity.get(),
            self.is_mini.get(),
            self.is_all_paused.get(),
            &self.hotkey.get(),
            self.hotkey_enabled.get(),
            win_placement,
        );
    }

    /// 删除：立刻从列表移除并落盘，把副本加入层叠 ToastStack。
    pub fn delete(&self, id: u64) {
        let mut removed = None;
        crate::ui::animate_rows();
        self.items.update(|list| {
            if let Some(pos) = list.iter().position(|x| x.id == id) {
                let prev_id = if pos > 0 {
                    Some(list[pos - 1].id)
                } else {
                    None
                };
                let next_id = if pos + 1 < list.len() {
                    Some(list[pos + 1].id)
                } else {
                    None
                };
                let item = list.remove(pos);
                removed = Some((prev_id, next_id, item));
            }
        });
        if let Some((prev_id, next_id, item)) = removed {
            let item_id = item.id;
            let pending = PendingDelete {
                item,
                prev_id,
                next_id,
                remaining_ms: std::rc::Rc::new(std::cell::Cell::new(UNDO_SECONDS * 1000)),
            };
            self.toast_manager.with(|m| {
                m.push(
                    item_id,
                    pending,
                    ToastOptions::timeout(Some(std::time::Duration::from_secs(UNDO_SECONDS))),
                    nsh_core::time::now(),
                );
            });
            self.sync_toasts();
            self.save();
        }
    }

    /// 撤销指定的被删提醒（按原相对位置插回并重新落盘，随后该 Toast 卡片退场）。
    pub fn undo(&self, id: u64) {
        let pending = self.toast_manager.with(|m| {
            let pending = m.get(&id).cloned()?;
            // 只有还在正常显示的卡片能恢复一次。`dismiss` 对已经进入退场
            // （撤销过 / 被划走 / 已到期）的卡片返回 false：它在 manager 里
            // 还要留到退场动画播完，这期间重复点击不能把同一条提醒插两遍。
            m.dismiss(&id, nsh_core::time::now()).then_some(pending)
        });
        let Some(pending) = pending else {
            return;
        };
        crate::ui::animate_rows();
        self.items.update(|list| {
            // 兜底：列表里已经有同 id 的提醒就不再插一次。
            if list.iter().all(|x| x.id != pending.item.id) {
                undo_restore_item(list, pending.item.clone(), pending.prev_id, pending.next_id);
            }
        });
        self.save();
        self.sync_toasts();
    }

    /// 撤销最新的一项（例如 Escape 键或快捷键）
    pub fn undo_latest(&self) {
        let latest = self
            .toast_entries
            .get()
            .iter()
            .rev()
            .find(|(_, _, s)| *s != ToastTransitionStatus::Ending)
            .map(|(id, _, _)| *id);
        if let Some(id) = latest {
            self.undo(id);
        }
    }

    /// 划走或关闭指定的 Toast
    pub fn dismiss_toast(&self, id: u64) {
        self.toast_manager.with(|m| {
            m.dismiss(&id, nsh_core::time::now());
        });
        self.sync_toasts();
    }

    /// 同步最新的可见 Toast 列表
    pub fn sync_toasts(&self) {
        self.toast_entries.set(self.toast_manager.with(|m| {
            m.visible(3)
                .map(|(id, item, status)| (*id, item.clone(), status))
                .collect::<Vec<_>>()
        }));
    }

    /// 展开「新建提醒」抽屉（收起设置抽屉，并强制复位草稿）。
    pub fn open_add(&self) {
        self.show_add.set(true);
        self.show_settings.set(false);
        // 抽屉已经打开时 set(true) 是空操作，用 tick 让页签和草稿复位
        self.add_reset.update(|tick| *tick = tick.wrapping_add(1));
    }

    /// 顶栏「新建提醒」按钮：开 / 收抽屉，两个抽屉互斥。
    pub fn toggle_add(&self) {
        let next = !self.show_add.get();
        self.show_add.set(next);
        if next {
            self.show_settings.set(false);
        }
    }

    /// 顶栏「偏好设置」按钮：开 / 收抽屉，两个抽屉互斥。
    pub fn toggle_settings(&self) {
        let next = !self.show_settings.get();
        self.show_settings.set(next);
        if next {
            self.show_add.set(false);
        }
    }

    /// 收起为迷你胶囊：两个抽屉都收掉，窗口跟着缩到胶囊尺寸。
    pub fn enter_mini(&self) {
        self.is_mini.set(true);
        self.show_add.set(false);
        self.show_settings.set(false);
        desktop::main_window().set_bounds(WindowFrame {
            width: Some(px(MINI_WIDTH)),
            height: Some(px(MINI_HEIGHT)),
            ..Default::default()
        });
        self.save();
    }

    /// 从胶囊展开为完整卡片。
    pub fn enter_full(&self) {
        self.is_mini.set(false);
        desktop::main_window().set_bounds(WindowFrame {
            width: Some(px(WINDOW_WIDTH)),
            height: Some(px(WINDOW_HEIGHT)),
            ..Default::default()
        });
        self.save();
    }

    /// 检查更新：同一时间只跑一个；没有打包配置（开发运行）时不做任何事。
    /// 已经下载好等退出替换时也不再检查，免得把「待更新」刷掉。
    pub fn check_update(&self) {
        let status = self.update.get();
        if status.is_busy()
            || matches!(
                status,
                updater::Status::Unavailable | updater::Status::Ready { .. }
            )
        {
            return;
        }
        self.update.set(updater::Status::Checking);
        updater::start_check();
    }

    /// 下载新版本：下载、按清单校验，解包摆到应用旁边，退出时替换。
    /// 装不进去的目录（只读、系统包）不下载，界面给的是下载页。
    pub fn download_update(&self) {
        let updater::Status::Available(found) = self.update.get() else {
            return;
        };
        if !found.can_install {
            return;
        }
        self.update.set(updater::Status::Downloading {
            version: found.version.clone(),
            received: 0,
            total: found.size,
        });
        updater::start_download(found);
    }

    /// 已经下载好时点它：立刻替换并重启，应用自己拉起新版本。
    /// 什么都不点也没关系——退出应用时同样会替换。
    pub fn restart_and_update(&self) {
        if !matches!(self.update.get(), updater::Status::Ready { .. }) {
            return;
        }
        // 真正的替换由 `state::install_update` 的副作用做：它拿着已经摆好的新版本
        self.update.set(updater::Status::Installing);
    }
}
