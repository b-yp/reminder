//! 应用入口：装配状态上下文、恢复窗口摆放、挂载应用级副作用，然后把根视图交给视图层。
//!
//! 这一层负责「窗口准备好、状态就绪」，不含任何具体控件的外观；
//! 完整卡片与迷你胶囊分别在 `views::card` / `views::mini`。

mod ctx;
mod state;

pub use ctx::{AppCtx, PendingDelete};

use nsh_base::theme;
use nsh_core::desktop::WindowFrame;
use nsh_core::element::desktop;
use nsh_core::prelude::*;

use crate::constants::{MINI_HEIGHT, MINI_WIDTH, WINDOW_HEIGHT, WINDOW_WIDTH};
use crate::storage::load_state;
use crate::views;
use crate::window;

/// 应用程序根组件
#[component]
pub fn App() -> impl IntoElement {
    // 0. 加载本地数据持久化配置（含离线时间流逝补偿、自增 ID 同步与历史窗口摆放）
    let initial_state = load_state();

    // 预设透明度（macOS 立即生效；Windows 此时还没有 HWND，由 setup_floating_window 补上）
    window::set_window_opacity(initial_state.opacity);

    let (init_w, init_h) = if initial_state.is_mini {
        (MINI_WIDTH, MINI_HEIGHT)
    } else {
        (WINDOW_WIDTH, WINDOW_HEIGHT)
    };

    // 1. 挂件专用配置：在 Windows 与 macOS 上完成平台特定的窗口底层配置
    window::setup_floating_window(initial_state.opacity);

    // 2. 窗口原生桌面属性：置顶悬浮 (Floating)
    desktop::main_window().set(window::floating_window_attributes());

    // 3. 恢复上次摆放与尺寸：窗口摆放交给 NSH 的 placement（逻辑点），显示器拔插后自动贴回可见区域
    let window_size = size(px(init_w), px(init_h));
    match initial_state.window_placement {
        Some(saved) => {
            // 尺寸以当前形态（卡片/胶囊）为准，位置沿用上次摆放
            let bounds = Bounds::new(saved.bounds.origin, window_size);
            desktop::main_window().set_placement(saved.with_bounds(bounds));
        }
        None => desktop::main_window().set_bounds(WindowFrame {
            width: Some(window_size.width),
            height: Some(window_size.height),
            ..Default::default()
        }),
    }

    // 4. 响应式状态管理（由持久化数据初始化，之后所有视图共享同一个上下文）
    let app = AppCtx::new(initial_state);

    // 5. 应用级副作用：托盘、关闭拦截、主题 / 不透明度联动、热键、每秒心跳
    state::install(app);

    // 根视图容器：基于 NSH 官方 theme().card 与 theme().border，原生自适应明暗，支持随意拖拽
    // Windows 下由 Win11 DWM 硬件级圆角与边框 (DWMWCP_ROUND) 托管裁剪与边框阴影，
    // 若在根视图重复叠加 rounded 与 border 会因曲率不一致产生四角双层重影，故仅在非 Windows (如 macOS) 应用视图层圆角边框。
    let root = view()
        .window_drag(true) // 按住卡片空白处即可随意拖动窗口
        .w_full()
        .h_full()
        .bg(move || theme().card);

    #[cfg(not(target_os = "windows"))]
    let root = root
        .border_1()
        .border_color(move || theme().border)
        .rounded(px(crate::ui::tokens::radius::MAX));

    root.child(
        when(move || app.is_mini.get(), move || views::mini::render(app))
            .otherwise(move || views::card::render(app)),
    )
}
