//! reminder：桌面通用健康与生活提醒悬浮挂件。
//!
//! 纯 Rust 编写、遵循 NSH 官方设计规范与 shadcn/ui 主题系统，支持明暗主题自由切换与跟随系统、
//! 随意拖拽、矢量环形进度图标、全局热键一键隐藏 / 呼出悬浮窗、关闭收进托盘继续提醒、登录系统自动启动。
//!
//! 结构：`app`（根组件 + 状态上下文 + 应用级副作用）、`views`（纯外观组合）、
//! `window`（平台窗口层）、`ui`（控件外观层），以及 `model` / `storage` / `hotkey` 等领域模块。

mod alarm;
mod app;
pub mod autostart;
mod constants;
pub mod hotkey;
pub mod icons;
pub mod identity;
pub mod localtime;
pub mod model;
pub(crate) mod presets;
pub mod storage;
pub mod theme_mode;
pub mod ui;
pub(crate) mod views;
mod window;

pub use app::App;
pub use constants::{MINI_HEIGHT, MINI_WIDTH, WINDOW_HEIGHT, WINDOW_WIDTH};
pub use identity::APP_ID;
pub use storage::AppState;

use nsh_core::prelude::*;

#[native_module("reminder/native")]
fn native(registry: &mut Registry) {
    registry.assets(assets!());
    registry.register::<App>();
}
