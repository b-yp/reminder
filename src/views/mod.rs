//! 视图层：只组合 `ui` 控件与 `AppCtx` 状态，不碰平台窗口代码。
//!
//! 每个视图都只接收一个 `AppCtx`（`Copy`），写操作走 `AppCtx` 的方法；
//! 这里没有 `nsh_platform_*`，也没有持久化细节。

pub mod add_drawer;
pub mod card;
pub mod footer;
pub mod header;
pub mod list;
pub mod mini;
pub mod presets;
pub mod settings;
pub mod toast;
