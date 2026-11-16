//! The app's Rust UI: reminder 桌面通用健康与生活提醒悬浮挂件
//! 纯 Rust 编写、完全遵循 NSH 官方设计规范与 shadcn/ui 主题系统、支持明暗主题自由切换与跟随系统、支持随意拖拽、矢量环形进度图标

pub mod icons;
pub mod model;
pub mod storage;
pub mod theme_mode;

use storage::load_state;
pub use storage::AppState;

use std::time::Duration;
use nsh_core::prelude::*;
use nsh_core::element::{desktop, input, AdaptiveColor};
use nsh_core::desktop::{WindowAttributes, WindowFrame, WindowFrameStyle, WindowLevel};
use nsh_base::{SliderValue, slider, collapsible, theme};
use model::ReminderItem;
use theme_mode::{ThemeMode, ACCENT_GOLD, ACCENT_GOLD_BG};

const LOGO: ImageAsset = asset!("logo.png");

#[cfg(target_os = "windows")]
use nsh_platform_windows as platform;
#[cfg(target_os = "macos")]
use nsh_platform_macos as platform;

#[cfg(target_os = "windows")]
static APP_HWND: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

/// 窗口不透明度下限：再低就看不见挂件了（设置里的滑杆下限与之一致）
const MIN_OPACITY: f32 = 0.20;

/// 把任意输入收敛到可用的不透明度区间
fn clamp_opacity(opacity: f32) -> f32 {
    opacity.clamp(MIN_OPACITY, 1.0)
}

/// 设置窗口整体不透明度：Windows 用分层窗口 alpha，macOS 用 `NSWindow.alphaValue`。
/// HWND 尚未就绪时（挂载最初几十毫秒）静默跳过，由 `setup_floating_window` 在拿到句柄后补上初值。
#[cfg(target_os = "windows")]
fn set_window_opacity(opacity: f32) {
    let opacity = clamp_opacity(opacity);
    let raw = APP_HWND.load(std::sync::atomic::Ordering::SeqCst);
    if raw == 0 {
        return;
    }
    use platform::windows::Win32::Foundation::{HWND, COLORREF};
    use platform::windows::Win32::UI::WindowsAndMessaging::*;

    let hwnd = HWND(raw as *mut _);
    unsafe {
        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
        if (ex_style & (WS_EX_LAYERED.0 as i32)) == 0 {
            let _ = SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style | WS_EX_LAYERED.0 as i32);
        }
        let alpha = (opacity * 255.0).round() as u8;
        let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), alpha, LWA_ALPHA);
    }
}

/// 设置 macOS 原生窗口透明度
#[cfg(target_os = "macos")]
fn set_window_opacity(opacity: f32) {
    let opacity = clamp_opacity(opacity);
    dispatch2::DispatchQueue::main().exec_async(move || {
        use platform::objc2::MainThreadMarker;
        use platform::objc2_app_kit::NSApplication;
        if let Some(mtm) = MainThreadMarker::new() {
            let app = NSApplication::sharedApplication(mtm);
            for window in app.windows().iter() {
                window.setAlphaValue(opacity as f64);
            }
        }
    });
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn set_window_opacity(_opacity: f32) {}



/// macOS 挂件专用配置：在主线程剥离窗口边框、隐藏交通灯并启用透明底色与阴影
#[cfg(target_os = "macos")]
fn configure_macos_window() {
    use platform::objc2::MainThreadMarker;
    use platform::objc2_app_kit::{
        NSApplication, NSColor, NSWindowButton, NSWindowStyleMask, NSWindowTitleVisibility,
    };

    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    for window in app.windows().iter() {
        // 1. 设置内容充满整个窗口区域 (FullSizeContentView)，隐藏系统标题栏与文字
        let mut mask = window.styleMask();
        mask |= NSWindowStyleMask::FullSizeContentView;
        mask &= !NSWindowStyleMask::Resizable;
        window.setStyleMask(mask);

        window.setTitleVisibility(NSWindowTitleVisibility::Hidden);
        window.setTitlebarAppearsTransparent(true);

        // 2. 彻底隐藏红绿灯控制按钮（关闭、最小化、缩放）
        if let Some(btn) = window.standardWindowButton(NSWindowButton::CloseButton) {
            btn.setHidden(true);
        }
        if let Some(btn) = window.standardWindowButton(NSWindowButton::MiniaturizeButton) {
            btn.setHidden(true);
        }
        if let Some(btn) = window.standardWindowButton(NSWindowButton::ZoomButton) {
            btn.setHidden(true);
        }

        // 3. 设置窗口底层为 100% 透明，彻底剥离 macOS 默认窗口灰色背景与边框
        window.setOpaque(false);
        window.setBackgroundColor(Some(&NSColor::clearColor()));

        // 4. 启用原生系统阴影与背景随意拖拽
        window.setHasShadow(true);
        window.setMovableByWindowBackground(true);
        window.invalidateShadow();
    }
}

/// macOS 挂件专用：配置为纯净无边框悬浮卡片，移除全部系统控制按钮与标题栏非客户区，保留原生圆角与阴影
#[cfg(target_os = "macos")]
fn setup_floating_window(_initial_opacity: f32) {
    dispatch2::DispatchQueue::main().exec_async(configure_macos_window);

    // 在主线程启动并渲染前几个周期内再次确认配置，确保覆盖 AppKit 任何后续重置
    std::thread::spawn(|| {
        for delay in [30, 80, 200, 500] {
            std::thread::sleep(std::time::Duration::from_millis(delay));
            dispatch2::DispatchQueue::main().exec_async(configure_macos_window);
        }
    });
}

/// Windows 挂件专用：获取 HWND 句柄以支持运行时透明度调节与窗口图标
#[cfg(target_os = "windows")]
fn setup_floating_window(initial_opacity: f32) {
    std::thread::spawn(move || {
        use platform::windows::Win32::Foundation::{HWND, LPARAM};
        use platform::windows::Win32::System::Threading::GetCurrentProcessId;
        use platform::windows::Win32::UI::WindowsAndMessaging::*;
        use platform::windows::core::BOOL;

        unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
            unsafe {
                let mut pid = 0;
                let _ = GetWindowThreadProcessId(hwnd, Some(&mut pid));
                if pid == GetCurrentProcessId() {
                    let mut class = [0u16; 256];
                    let clen = GetClassNameW(hwnd, &mut class);
                    let class_name = String::from_utf16_lossy(&class[..clen as usize]);
                    if class_name == "NshWindow" {
                        *(lparam.0 as *mut HWND) = hwnd;
                        return BOOL(0);
                    }
                }
            }
            BOOL(1)
        }

        for _ in 0..50 {
            std::thread::sleep(std::time::Duration::from_millis(30));
            let mut target_hwnd = HWND::default();
            let _ = unsafe { EnumWindows(Some(enum_proc), LPARAM(&mut target_hwnd as *mut _ as isize)) };

            if !target_hwnd.is_invalid() {
                APP_HWND.store(target_hwnd.0 as isize, std::sync::atomic::Ordering::SeqCst);
                apply_window_icon(target_hwnd);
                // 补上挂载时还无法应用的不透明度
                if initial_opacity < 1.0 {
                    set_window_opacity(initial_opacity);
                }
                break;
            }
        }
    });
}

/// 窗口图标文件：`nsh run` 的 runner 只在 `$NSH_APP_DIR/icon.ico` 找图标，而 `nsh build` 不生成它，
/// 所以退一步用已随 bundle 拷贝的 `assets/@<package>/app.ico`；`cargo run` 则用源码目录里的那份。
#[cfg(target_os = "windows")]
fn window_icon_path() -> Option<std::path::PathBuf> {
    use std::path::PathBuf;
    let bundled = PathBuf::from("assets")
        .join(format!("@{}", env!("CARGO_PKG_NAME")))
        .join("app.ico");
    let nsh_dir = std::env::var_os("NSH_APP_DIR").map(PathBuf::from).or_else(|| {
        std::env::current_exe()
            .ok()?
            .parent()
            .map(|dir| dir.join("nsh"))
    });
    nsh_dir
        .into_iter()
        .flat_map(|dir| [dir.join("icon.ico"), dir.join(&bundled)])
        .chain([PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/app.ico"
        ))])
        .find(|path| path.is_file())
}

/// 为窗口设置任务栏 / Alt+Tab / 托盘回退用的图标。
/// 与 NSH 原生 `window::set_icon` 同一套做法：按窗口 DPI 取系统大/小图标尺寸加载，只发 `WM_SETICON`；
/// 图标句柄进程内常驻，不 `DestroyIcon`。
#[cfg(target_os = "windows")]
fn apply_window_icon(hwnd: platform::windows::Win32::Foundation::HWND) {
    use platform::windows::Win32::Foundation::{LPARAM, WPARAM};
    use platform::windows::Win32::UI::HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi};
    use platform::windows::Win32::UI::WindowsAndMessaging::*;
    use platform::windows::core::HSTRING;

    let Some(path) = window_icon_path() else {
        return;
    };
    let path = HSTRING::from(path.as_os_str());
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    for (metric, which) in [(SM_CXICON, ICON_BIG), (SM_CXSMICON, ICON_SMALL)] {
        let side = unsafe { GetSystemMetricsForDpi(metric, dpi) };
        // SAFETY: 图标文件存在且一直保留；句柄比窗口活得久。
        let Ok(icon) = (unsafe { LoadImageW(None, &path, IMAGE_ICON, side, side, LR_LOADFROMFILE) })
        else {
            continue;
        };
        unsafe {
            SendMessageW(
                hwnd,
                WM_SETICON,
                Some(WPARAM(which as usize)),
                Some(LPARAM(icon.0 as isize)),
            );
        }
    }
}

/// 统一窗口尺寸规范（逻辑像素，自动由 NSH 处理 High-DPI 缩放，杜绝模糊与尺寸割裂）
pub const WINDOW_WIDTH: f32 = 320.0;
pub const WINDOW_HEIGHT: f32 = 350.0;
/// 迷你胶囊形态尺寸（极简悬浮小药丸，仅占用极少屏幕空间）
pub const MINI_WIDTH: f32 = 196.0;
pub const MINI_HEIGHT: f32 = 44.0;

/// 仅设置「置顶悬浮」层级的窗口属性，设置极小 min_size 防止 Win32 限制最小追踪尺寸
/// （`WindowAttributes` 为 `non_exhaustive`，故用 Default 赋值而非结构体字面量）
fn floating_window_attributes() -> WindowAttributes {
    let mut attributes = WindowAttributes::default();
    attributes.level = Some(WindowLevel::Floating);
    attributes.frame = Some(WindowFrameStyle::None);
    attributes.min_size = Some(Some(size(px(100.0), px(36.0))));
    attributes
}

/// 触发倒计时到期提醒（声音、系统原生通知、窗口置顶唤起）
pub fn trigger_reminder_alarm(item_name: &str) {
    // 1. 窗口唤起置顶：确保用户即使在打游戏或全屏工作也能看到悬浮挂件
    desktop::main_window().focus();

    // 2. 播放系统提醒音
    #[cfg(target_os = "windows")]
    unsafe {
        use platform::windows::Win32::System::Diagnostics::Debug::MessageBeep;
        use platform::windows::Win32::UI::WindowsAndMessaging::MB_ICONEXCLAMATION;
        let _ = MessageBeep(MB_ICONEXCLAMATION);
    }
    #[cfg(not(target_os = "windows"))]
    {
        use std::io::Write;
        print!("\x07");
        let _ = std::io::stdout().flush();
    }

    // 3. 发送跨平台系统原生通知 (Windows Toast / macOS Notification / Linux D-Bus)
    let name = item_name.to_string();
    std::thread::spawn(move || {
        let _ = notify_rust::Notification::new()
            .summary("reminder 定时提醒")
            .body(&format!("{} 到时间啦！请及时打卡或推迟。", name))
            .timeout(notify_rust::Timeout::Milliseconds(6000))
            .show();
    });
}

#[native_module("reminder/native")]
fn native(registry: &mut Registry) {
    registry.assets(assets!());
    registry.register::<App>();
}

/// 预设提醒模板（通用丰富场景）
struct PresetReminder {
    name: &'static str,
    icon_key: &'static str,
    minutes: u32,
}

const PRESETS: &[PresetReminder] = &[
    PresetReminder { name: "喝水", icon_key: "droplet", minutes: 30 },
    PresetReminder { name: "起身活动", icon_key: "walk", minutes: 45 },
    PresetReminder { name: "护眼远眺", icon_key: "eye", minutes: 20 },
    PresetReminder { name: "番茄专注", icon_key: "clock", minutes: 25 },
    PresetReminder { name: "带薪拉屎", icon_key: "toilet", minutes: 90 },
    PresetReminder { name: "站立拉伸", icon_key: "activity", minutes: 60 },
];

/// 应用程序根组件
#[component]
pub fn App() -> impl IntoElement {
    // 0. 加载本地数据持久化配置（含离线时间流逝补偿、自增 ID 同步与历史窗口摆放）
    let initial_state = load_state();

    // 预设透明度（macOS 立即生效；Windows 此时还没有 HWND，由 setup_floating_window 补上）
    set_window_opacity(initial_state.opacity);

    let (init_w, init_h) = if initial_state.is_mini {
        (MINI_WIDTH, MINI_HEIGHT)
    } else {
        (WINDOW_WIDTH, WINDOW_HEIGHT)
    };

    // 1. 挂件专用配置：在 Windows 与 macOS 上完成平台特定的窗口底层配置
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    setup_floating_window(initial_state.opacity);

    // 2. 窗口原生桌面属性：置顶悬浮 (Floating)
    desktop::main_window().set(floating_window_attributes());

    // 配置 Windows / macOS 系统托盘与状态栏通知区域图标及单击唤起
    desktop::set_tray(Some(
        desktop::tray()
            .icon(LOGO)
            .tooltip("reminder 提醒")
            .on_click(|_| {
                desktop::main_window().show();
                desktop::main_window().focus();
            }),
    ));

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

    // 4. 响应式状态管理（由持久化数据初始化）
    let theme_mode = signal(initial_state.theme_mode);
    let items = signal(initial_state.items);
    let is_all_paused = signal(initial_state.is_all_paused);
    let opacity = signal(initial_state.opacity);
    let is_mini = signal(initial_state.is_mini);
    let last_saved_placement = signal(initial_state.window_placement);
    let show_settings = signal(false);
    let show_presets = signal(false);
    let show_custom_form = signal(false);

    // 统一配置持久化回调
    let save_app_state = {
        move || {
            let win_placement = desktop::main_window()
                .state()
                .map(|state| state.placement)
                .or_else(|| last_saved_placement.get());
            storage::persist_current(
                &items.get(),
                theme_mode.get(),
                opacity.get(),
                is_mini.get(),
                is_all_paused.get(),
                win_placement,
            );
        }
    };

    // 主题与底层平台/渲染器联动：切换 NSH 官方色彩方案与系统外观，并在后续改动时自动保存
    let theme_initialized = stored(false);
    effect(move || {
        let mode = theme_mode.get();
        nsh_core::element::set_color_scheme(mode.to_appearance());
        if !theme_initialized.get() {
            theme_initialized.replace(true);
        } else {
            save_app_state();
        }
    });

    // 窗口不透明度响应式联动：当 opacity 发生变化时，实时调用原生窗口 API 并在后续改动时自动保存
    let opacity_initialized = stored(false);
    effect(move || {
        let val = opacity.get();
        set_window_opacity(val);
        if !opacity_initialized.get() {
            opacity_initialized.replace(true);
        } else {
            save_app_state();
        }
    });

    // 每秒驱动倒计时更新与到期提醒，检测窗口拖动并定期同步时间戳（提供离线流逝时间基准）与位置变动
    let tick_count = std::rc::Rc::new(std::cell::Cell::new(0u64));
    let tick_count_cloned = tick_count.clone();
    let tick_timer = stored(None::<Spawned<()>>);
    tick_timer.replace(Some(set_interval(Duration::from_secs(1), move || {
        let count = tick_count_cloned.get() + 1;
        tick_count_cloned.set(count);

        // 1. 窗口摆放变动检测与自动持久化（拖拽释放后 1 秒内保存）
        let mut pos_changed = false;
        if let Some(state) = desktop::main_window().state() {
            if last_saved_placement.get() != Some(state.placement) {
                last_saved_placement.set(Some(state.placement));
                pos_changed = true;
            }
        }

        // 2. 倒计时与提醒驱动
        let mut state_changed = pos_changed;
        if !is_all_paused.get() {
            let mut newly_expired = Vec::new();
            items.update(|list| {
                for item in list.iter_mut() {
                    if item.tick() {
                        newly_expired.push(item.name.clone());
                        state_changed = true;
                    }
                }
            });
            for name in newly_expired {
                trigger_reminder_alarm(&name);
            }
        }

        // 每 15 秒同步一次（确保离线补偿时间戳最新），或在窗口移动/任务到期时立即同步
        if count % 15 == 0 || state_changed {
            save_app_state();
        }
    })));

    let enter_mini = move || {
        is_mini.set(true);
        show_settings.set(false);
        desktop::main_window().set_bounds(WindowFrame {
            width: Some(px(MINI_WIDTH)),
            height: Some(px(MINI_HEIGHT)),
            ..Default::default()
        });
        save_app_state();
    };

    let enter_full = move || {
        is_mini.set(false);
        desktop::main_window().set_bounds(WindowFrame {
            width: Some(px(WINDOW_WIDTH)),
            height: Some(px(WINDOW_HEIGHT)),
            ..Default::default()
        });
        save_app_state();
    };

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
        .rounded(px(12.0));

    root.child(
        when(
            move || is_mini.get(),
            move || render_mini_capsule(items, is_all_paused, enter_full, save_app_state),
        )
        .otherwise(move || {
            render_full_card(
                items,
                is_all_paused,
                show_settings,
                show_presets,
                show_custom_form,
                opacity,
                theme_mode,
                enter_mini,
                save_app_state,
            )
        }),
    )
}

/// 完整卡片视图形态（320x350 像素，包含顶栏、核心滚动区与底部统计）
fn render_full_card(
    items: Signal<Vec<ReminderItem>>,
    is_all_paused: Signal<bool>,
    show_settings: Signal<bool>,
    show_presets: Signal<bool>,
    show_custom_form: Signal<bool>,
    opacity: Signal<f32>,
    theme_mode: Signal<ThemeMode>,
    on_enter_mini: impl Fn() + Copy + 'static,
    on_save: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    view()
        .w_full()
        .h_full()
        .p(px(14.0))
        .flex_col()
        .justify_between()
        // 1. 顶栏：通用「reminder 提醒」金黄微标与标题，右侧挂件操作按钮组（暂停、设置、收起胶囊、关闭）
        .child(render_header(items, is_all_paused, show_settings, on_enter_mini, on_save))
        // 2. 核心内容滚动区：抽屉展开与列表统一容纳，高了自行平滑滚动，大窗口全程稳定
        .child(
            scroll()
                .window_drag(true)
                .w_full()
                .flex_1()
                .my(px(4.0))
                .scrollbar(false)
                .child(
                    view()
                        .w_full()
                        .flex_col()
                        .gap(px(6.0))
                        // 新建/设置抽屉（包含外观主题选择、透明度调节、推荐预设与自定义提醒）
                        .child(render_drawer(
                            show_settings,
                            show_presets,
                            show_custom_form,
                            items,
                            opacity,
                            theme_mode,
                            on_save,
                        ))
                        // 核心提醒列表
                        .child(render_items(items, on_save)),
                ),
        )
        // 3. 底部状态行（通用统计：「进行中 X 项 · 今日打卡 Y 次」，若处于半透明状态则额外指示不透明度）
        .child(render_footer(items, opacity))
}

/// 迷你胶囊挂件形态（Mini Pill / Dynamic Island）：仅 220x44 像素，支持全屏自由拖动、显示最优先提醒、一键完成打卡与一键展开
fn render_mini_capsule(
    items: Signal<Vec<ReminderItem>>,
    is_all_paused: Signal<bool>,
    on_expand: impl Fn() + Copy + 'static,
    on_save: impl Fn() + Copy + 'static,
) -> impl IntoElement {
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
                            let color = if is_exp { "#ef4444" } else { icons::get_item_color(&it.icon) };
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
                .h_full()
                .justify_center()
                .px(px(6.0))
                .child(
                    view()
                        .flex_col()
                        .justify_center()
                        .gap(px(1.0))
                        // 上行：任务名称与多项小指示
                        .child(
                            view()
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
                                    .text_color(move || theme().foreground),
                                )
                                .child(
                                    when(
                                        move || active_item_info.with(|info| info.as_ref().map_or(false, |(_, _, total)| *total > 1)),
                                        move || {
                                            text(move || {
                                                active_item_info.with(|info| {
                                                    info.as_ref().map_or("".to_string(), |(idx, _, total)| {
                                                        format!("{}/{}", idx + 1, total)
                                                    })
                                                })
                                            })
                                            .text_size(px(9.5))
                                            .text_color(move || theme().muted_foreground)
                                        },
                                    ),
                                ),
                        )
                        // 下行：倒计时时间或警示状态
                        .child(
                            view()
                                .flex_row()
                                .items_center()
                                .gap(px(4.0))
                                .child(
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
                                        } else if active_item_info.with(|info| info.as_ref().map_or(false, |(_, it, _)| it.is_expired())) {
                                            theme().destructive
                                        } else {
                                            ACCENT_GOLD
                                        }
                                    }),
                                ),
                        ),
                ),
        )
        // 右侧操作按钮组（快捷打卡、展开为卡片、关闭）
        .child(
            view()
                .window_drag(false) // 按钮区域不触发拖拽
                .flex_row()
                .items_center()
                .gap(px(4.0))
                // 快捷打卡对勾按钮（特别是在到期时一键打卡）
                .child(
                    view()
                        .w(px(22.0))
                        .h(px(22.0))
                        .rounded(px(5.0))
                        .items_center()
                        .justify_center()
                        .bg(move || {
                            let is_exp = active_item_info.with(|info| info.as_ref().map_or(false, |(_, it, _)| it.is_expired()));
                            if is_exp {
                                ACCENT_GOLD_BG
                            } else {
                                theme().secondary
                            }
                        })
                        .border_1()
                        .border_color(move || {
                            let is_exp = active_item_info.with(|info| info.as_ref().map_or(false, |(_, it, _)| it.is_expired()));
                            if is_exp {
                                ACCENT_GOLD
                            } else {
                                theme().border
                            }
                        })
                        .active(|s| s.opacity(0.5))
                        .on_press(move || {
                            if let Some((idx, _, _)) = active_item_info.get() {
                                items.update(|list| {
                                    if idx < list.len() {
                                        list[idx].complete();
                                    }
                                });
                                on_save();
                            }
                        })
                        .child(
                            icons::icon_svg(
                                icons::ICON_CHECK,
                                12.0,
                                move || {
                                    let is_exp = active_item_info.with(|info| info.as_ref().map_or(false, |(_, it, _)| it.is_expired()));
                                    if is_exp {
                                        ACCENT_GOLD
                                    } else {
                                        theme().muted_foreground
                                    }
                                },
                            ),
                        ),
                )
                // 展开为大卡片按钮
                .child(
                    view()
                        .test_id("expand-mini-btn")
                        .w(px(22.0))
                        .h(px(22.0))
                        .rounded(px(5.0))
                        .items_center()
                        .justify_center()
                        .bg(move || theme().secondary)
                        .border_1()
                        .border_color(move || theme().border)
                        .active(|s| s.opacity(0.5))
                        .on_press(on_expand)
                        .child(
                            icons::icon_svg(
                                icons::ICON_EXPAND,
                                11.0,
                                move || theme().muted_foreground,
                            ),
                        ),
                ),
        )
}

/// 顶部 Header：左侧金黄微标与标题，右侧挂件操作按钮组（暂停、设置/添加、收起为胶囊、关闭退出）
fn render_header(
    items: Signal<Vec<ReminderItem>>,
    is_all_paused: Signal<bool>,
    show_settings: Signal<bool>,
    on_enter_mini: impl Fn() + Copy + 'static,
    on_save: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let count_text = computed(move || items.with(|list| format!("{} 项", list.len())));

    view()
        .window_drag(true)
        .flex_row()
        .items_center()
        .justify_between()
        .pb(px(4.0))
        // 左侧：微标 + 标题 + 数量指示
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(
                    // App Logo 微标
                    image(LOGO)
                        .size(px(24.0))
                        .rounded(px(6.0)),
                )
                .child(
                    text("reminder 提醒")
                        .text_size(px(13.5))
                        .text_color(move || theme().foreground),
                )
                .child(
                    // 进行中数量指示小徽章
                    view()
                        .px(px(6.0))
                        .py(px(1.5))
                        .rounded(px(10.0))
                        .bg(move || theme().muted)
                        .child(
                            text(move || count_text.get())
                                .text_size(px(10.5))
                                .text_color(move || theme().muted_foreground),
                        ),
                ),
        )
        // 右侧挂件专属操作按钮组：暂停/恢复全部、展开设置/新建抽屉、关闭退出挂件
        .child(
            view()
                .window_drag(false) // 按钮区域不触发窗口拖拽
                .flex_row()
                .items_center()
                .gap(px(6.0))
                // 暂停 / 播放全部
                .child(
                    view()
                        .w(px(22.0))
                        .h(px(22.0))
                        .rounded(px(5.0))
                        .items_center()
                        .justify_center()
                        .bg(move || theme().secondary)
                        .active(|s| s.opacity(0.5))
                        .on_press(move || {
                            is_all_paused.update(|v| *v = !*v);
                            on_save();
                        })
                        .child(
                            when(
                                move || is_all_paused.get(),
                                move || icons::icon_svg(icons::ICON_PLAY, 11.0, ACCENT_GOLD),
                            )
                            .otherwise(move || icons::icon_svg(icons::ICON_PAUSE, 11.0, move || theme().muted_foreground)),
                        ),
                )
                // 设置 / 新建
                .child(
                    view()
                        .w(px(22.0))
                        .h(px(22.0))
                        .rounded(px(5.0))
                        .items_center()
                        .justify_center()
                        .bg(move || {
                            if show_settings.get() {
                                ACCENT_GOLD_BG
                            } else {
                                theme().secondary
                            }
                        })
                        .border_1()
                        .border_color(move || {
                            if show_settings.get() {
                                ACCENT_GOLD
                            } else {
                                theme().border
                            }
                        })
                        .active(|s| s.opacity(0.5))
                        .on_press(move || show_settings.update(|v| *v = !*v))
                        .child(
                            icons::dynamic_icon_svg(
                                move || icons::ICON_SETTINGS,
                                12.0,
                                move || {
                                    if show_settings.get() {
                                        ACCENT_GOLD
                                    } else {
                                        theme().muted_foreground
                                    }
                                },
                            ),
                        ),
                )
                // 收起为迷你灵动岛胶囊
                .child(
                    view()
                        .w(px(22.0))
                        .h(px(22.0))
                        .rounded(px(5.0))
                        .items_center()
                        .justify_center()
                        .bg(move || theme().secondary)
                        .border_1()
                        .border_color(move || theme().border)
                        .active(|s| s.opacity(0.5))
                        .on_press(on_enter_mini)
                        .child(
                            icons::icon_svg(
                                icons::ICON_MINIMIZE,
                                11.0,
                                move || theme().muted_foreground,
                            ),
                        ),
                )
                // 退出挂件 (轻量悬浮叉叉)
                .child(
                    view()
                        .w(px(22.0))
                        .h(px(22.0))
                        .rounded(px(5.0))
                        .items_center()
                        .justify_center()
                        .bg(move || theme().secondary)
                        .active(|s| s.opacity(0.5))
                        .on_press(move || {
                            on_save();
                            desktop::main_window().close();
                        })
                        .child(
                            icons::icon_svg(icons::ICON_X, 11.0, move || theme().muted_foreground),
                        ),
                ),
        )
}

/// 核心列表：基于 each 原语基于 key 响应式高效追踪每个 item 的变化
fn render_items(
    items: Signal<Vec<ReminderItem>>,
    on_save: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    view()
        .window_drag(true)
        .w_full()
        .flex_col()
        .gap(px(6.0))
        .child(each(
            items,
            |item| item.id,
            move |item_sig, _| render_item_row(item_sig, items, on_save),
        ))
}

/// 单个提醒行：左侧发光进度图标 + 中间名称倒计时 + 完成次数 + 右侧 [✓] [+5] [🗑]
fn render_item_row(
    item_sig: ReadSignal<ReminderItem>,
    items: Signal<Vec<ReminderItem>>,
    on_save: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let item_id = item_sig.get_untracked().id;
    let icon_key = item_sig.get_untracked().icon.clone();

    view()
        .window_drag(true)
        .flex_row()
        .items_center()
        .justify_between()
        .py(px(4.0))
        .px(px(4.0))
        .rounded(px(6.0))
        // 到期时以官方 destructive 柔和警示底色高亮
        .bg(move || {
            if item_sig.get().is_expired() {
                theme().destructive.opacity(0.12)
            } else {
                AdaptiveColor::TRANSPARENT
            }
        })
        // 左半部分：圆环进度图标 + 标题/时间
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(10.0))
                // 左侧发光圆环进度图标（到期时红光警示，SVG 底轨原生自适应明暗）
                .child(
                    view()
                        .w(px(36.0))
                        .h(px(36.0))
                        .items_center()
                        .justify_center()
                        .text_color(move || theme().foreground)
                        .child(
                            svg(move || {
                                let it = item_sig.get();
                                let is_exp = it.is_expired();
                                let color = if is_exp { "#ef4444" } else { icons::get_item_color(&icon_key) };
                                let ratio = if is_exp { 1.0 } else { it.progress_ratio() };
                                let markup = icons::ring_progress_svg(&icon_key, ratio, color);
                                nsh_core::element::ImageSrc::svg_markup(&markup)
                            })
                            .w(px(36.0))
                            .h(px(36.0)),
                        ),
                )
                // 标题与倒计时（包含周期微标，点击周期可快速切换常见时长，到期时红字警示）
                .child(
                    view()
                        .flex_col()
                        .justify_center()
                        .gap(px(2.0))
                        .child(
                            view()
                                .flex_row()
                                .items_center()
                                .gap(px(5.0))
                                .child(
                                    text(move || item_sig.get().name.clone())
                                        .text_size(px(13.0))
                                        .text_color(move || theme().foreground),
                                )
                                .child(
                                    view()
                                        .window_drag(false)
                                        .px(px(4.0))
                                        .py(px(0.5))
                                        .rounded(px(3.0))
                                        .bg(move || theme().muted)
                                        .border_1()
                                        .border_color(move || theme().border)
                                        .active(|s| s.opacity(0.5))
                                        .on_press(move || {
                                            items.update(|list| {
                                                if let Some(it) = list.iter_mut().find(|x| x.id == item_id) {
                                                    let cur_m = it.interval_secs / 60;
                                                    let next_m = match cur_m {
                                                        0..=19 => 20,
                                                        20..=24 => 25,
                                                        25..=29 => 30,
                                                        30..=44 => 45,
                                                        45..=59 => 60,
                                                        60..=89 => 90,
                                                        90..=119 => 120,
                                                        _ => 15,
                                                    };
                                                    it.update_interval_mins(next_m);
                                                }
                                            });
                                            on_save();
                                        })
                                        .child(
                                            text(move || format!("{}m", item_sig.get().interval_secs / 60))
                                                .text_size(px(10.0))
                                                .text_color(move || theme().muted_foreground),
                                        ),
                                ),
                        )
                        .child(
                            view()
                                .flex_row()
                                .items_center()
                                .gap(px(3.0))
                                .child(
                                    when(
                                        move || item_sig.get().is_expired(),
                                        move || icons::icon_svg(icons::ICON_ALERT, 11.0, move || theme().destructive),
                                    ),
                                )
                                .child(
                                    text(move || item_sig.get().format_remaining())
                                        .text_size(px(11.5))
                                        .text_color(move || {
                                            if item_sig.get().is_expired() {
                                                theme().destructive
                                            } else {
                                                theme().muted_foreground
                                            }
                                        }),
                                ),
                        ),
                ),
        )
        // 右半部分：完成度「0 / 8」 + 胶囊按钮组 [✓] [+5] [🗑]
        .child(
            view()
                .window_drag(false)
                .flex_row()
                .items_center()
                .gap(px(6.0))
                // 完成度计数
                .child(
                    text(move || format!("{}/8", item_sig.get().completed_count))
                        .text_size(px(12.0))
                        .text_color(move || theme().muted_foreground),
                )
                // 操作按钮组
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(4.0))
                        // [✓] 打卡按钮（到期时强化为翡翠绿微亮框）
                        .child(
                            view()
                                .w(px(28.0))
                                .h(px(24.0))
                                .rounded(px(5.0))
                                .bg(move || {
                                    if item_sig.get().is_expired() {
                                        AdaptiveColor::rgb(0x16a34a, 0x15803d)
                                    } else {
                                        theme().secondary
                                    }
                                })
                                .border_1()
                                .border_color(move || {
                                    if item_sig.get().is_expired() {
                                        AdaptiveColor::rgb(0x22c55e, 0x4ade80)
                                    } else {
                                        theme().border
                                    }
                                })
                                .items_center()
                                .justify_center()
                                .active(|s| s.opacity(0.5))
                                .on_press(move || {
                                    items.update(|list| {
                                        if let Some(it) = list.iter_mut().find(|x| x.id == item_id) {
                                            it.complete();
                                        }
                                    });
                                    on_save();
                                })
                                .child(
                                    icons::dynamic_icon_svg(
                                        move || icons::ICON_CHECK,
                                        12.0,
                                        move || {
                                            if item_sig.get().is_expired() {
                                                AdaptiveColor::rgb(0xffffff, 0xffffff)
                                            } else {
                                                theme().foreground
                                            }
                                        },
                                    ),
                                ),
                        )
                        // [+5] 延期 5 分钟按钮
                        .child(
                            view()
                                .w(px(28.0))
                                .h(px(24.0))
                                .rounded(px(5.0))
                                .bg(move || theme().secondary)
                                .border_1()
                                .border_color(move || theme().border)
                                .items_center()
                                .justify_center()
                                .active(|s| s.opacity(0.5))
                                .on_press(move || {
                                    items.update(|list| {
                                        if let Some(it) = list.iter_mut().find(|x| x.id == item_id) {
                                            it.snooze(300);
                                        }
                                    });
                                    on_save();
                                })
                                .child(
                                    text("+5")
                                        .text_size(px(10.5))
                                        .text_color(move || theme().secondary_foreground),
                                ),
                        )
                        // [🗑] 移除该项提醒
                        .child(
                            view()
                                .w(px(22.0))
                                .h(px(24.0))
                                .rounded(px(5.0))
                                .items_center()
                                .justify_center()
                                .bg(move || theme().secondary)
                                .active(|s| s.opacity(0.5))
                                .on_press(move || {
                                    items.update(|list| list.retain(|x| x.id != item_id));
                                    on_save();
                                })
                                .child(
                                    icons::icon_svg(
                                        icons::ICON_TRASH,
                                        11.0,
                                        move || theme().muted_foreground,
                                    ),
                                ),
                        ),
                ),
        )
}

/// 底部统计行：细线分隔 + 灰白小字通用统计 + 半透明时指示不透明度状态
fn render_footer(
    items: Signal<Vec<ReminderItem>>,
    opacity: Signal<f32>,
) -> impl IntoElement {
    let summary_text = computed(move || {
        items.with(|list| {
            let total_completed: u32 = list.iter().map(|i| i.completed_count).sum();
            let count = list.len();
            format!("进行中 {} 项 · 今日打卡 {} 次", count, total_completed)
        })
    });

    view()
        .window_drag(true)
        .flex_row()
        .items_center()
        .justify_between()
        .pt(px(6.0))
        .border_t_1()
        .border_color(move || theme().border)
        .child(
            text(move || summary_text.get())
                .text_size(px(11.0))
                .text_color(move || theme().muted_foreground),
        )
        .child(
            when(
                move || opacity.get() < 0.99,
                move || {
                    text(move || format!("不透明度 {:.0}%", opacity.get() * 100.0))
                        .text_size(px(10.5))
                        .text_color(ACCENT_GOLD)
                },
            ),
        )
}

/// 渲染单个常用推荐预设卡片（精致卡片、无勾选、清晰时间标识、单例防重）
fn render_preset_card(
    preset: &'static PresetReminder,
    items: Signal<Vec<ReminderItem>>,
    on_save: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let name = preset.name;
    let icon = preset.icon_key;
    let mins = preset.minutes;

    let is_running = computed(move || items.with(|list| list.iter().any(|x| x.name == name)));

    view()
        .flex_1()
        .px(px(7.0))
        .py(px(5.0))
        .rounded(px(5.0))
        .bg(move || {
            if is_running.get() {
                AdaptiveColor::rgb(0xdcfce7, 0x064e3b)
            } else {
                theme().secondary
            }
        })
        .border_1()
        .border_color(move || {
            if is_running.get() {
                AdaptiveColor::rgb(0x86efac, 0x059669)
            } else {
                theme().border
            }
        })
        .active(|s| s.opacity(0.6))
        .on_press(move || {
            if !is_running.get() {
                items.update(|list| {
                    list.push(ReminderItem::new(
                        name,
                        icon,
                        mins * 60,
                    ));
                });
                on_save();
            }
        })
        .flex_row()
        .items_center()
        .justify_between()
        // 左侧：对应矢量图标 + 预设名称
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(5.0))
                .child(
                    icons::icon_svg(
                        icons::get_icon_markup(icon),
                        12.0,
                        icons::get_item_adaptive(icon),
                    ),
                )
                .child(
                    text(name)
                        .text_size(px(10.5))
                        .text_color(move || {
                            if is_running.get() {
                                AdaptiveColor::rgb(0x166534, 0x6ee7b7)
                            } else {
                                theme().foreground
                            }
                        }),
                ),
        )
        // 右侧：时长徽章
        .child(
            view()
                .px(px(4.0))
                .py(px(1.0))
                .rounded(px(3.0))
                .bg(move || {
                    if is_running.get() {
                        AdaptiveColor::rgb(0xbbf7d0, 0x065f46)
                    } else {
                        theme().muted
                    }
                })
                .child(
                    text(format!("{}m", mins))
                        .text_size(px(9.5))
                        .text_color(move || {
                            if is_running.get() {
                                AdaptiveColor::rgb(0x14532d, 0xa7f3d0)
                            } else {
                                theme().muted_foreground
                            }
                        }),
                ),
        )
}

/// 设置与添加新提醒抽屉（点击齿轮展开，支持外观主题切换、窗口不透明度调节、推荐快速开启与优雅折叠的自定义新提醒）
fn render_drawer(
    show: Signal<bool>,
    show_presets: Signal<bool>,
    show_custom_form: Signal<bool>,
    items: Signal<Vec<ReminderItem>>,
    opacity: Signal<f32>,
    theme_mode: Signal<ThemeMode>,
    on_save: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let custom_name = signal(String::new());
    let custom_mins_str = signal("30".to_string());
    let custom_icon = signal("bell".to_string());
    let error_tip = signal(Option::<String>::None);

    collapsible()
        .open(show)
        .reveal(true)
        .content(
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
                                            ACCENT_GOLD,
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
                                        .py(px(1.5))
                                        .rounded(px(4.0))
                                        .bg(ACCENT_GOLD_BG)
                                        .child(
                                            text(move || theme_mode.get().label())
                                                .text_size(px(10.5))
                                                .text_color(ACCENT_GOLD),
                                        ),
                                ),
                        )
                        .child(
                            view()
                                .w_full()
                                .flex_row()
                                .gap(px(4.0))
                                .children([
                                    (ThemeMode::Light, "浅色模式", icons::ICON_SUN),
                                    (ThemeMode::Dark, "深色模式", icons::ICON_MOON),
                                    (ThemeMode::System, "跟随系统", icons::ICON_LAPTOP),
                                ].into_iter().map(|(mode, label, icon)| {
                                    let is_selected = computed(move || theme_mode.get() == mode);
                                    view()
                                        .flex_1()
                                        .h(px(24.0))
                                        .rounded(px(5.0))
                                        .bg(move || {
                                            if is_selected.get() {
                                                ACCENT_GOLD_BG
                                            } else {
                                                theme().secondary
                                            }
                                        })
                                        .border_1()
                                        .border_color(move || {
                                            if is_selected.get() {
                                                ACCENT_GOLD
                                            } else {
                                                theme().border
                                            }
                                        })
                                        .items_center()
                                        .justify_center()
                                        .active(|s| s.opacity(0.6))
                                        .on_press(move || theme_mode.set(mode))
                                        .child(
                                            view()
                                                .flex_row()
                                                .items_center()
                                                .gap(px(4.0))
                                                .child(
                                                    icons::icon_svg(
                                                        icon,
                                                        11.0,
                                                        move || {
                                                            if is_selected.get() {
                                                                ACCENT_GOLD
                                                            } else {
                                                                theme().muted_foreground
                                                            }
                                                        },
                                                    ),
                                                )
                                                .child(
                                                    text(label)
                                                        .text_size(px(10.5))
                                                        .text_color(move || {
                                                            if is_selected.get() {
                                                                ACCENT_GOLD
                                                            } else {
                                                                theme().muted_foreground
                                                            }
                                                        }),
                                                ),
                                        )
                                })),
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
                                            ACCENT_GOLD,
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
                                        .py(px(1.5))
                                        .rounded(px(4.0))
                                        .bg(ACCENT_GOLD_BG)
                                        .child(
                                            text(move || format!("{:.0}%", opacity.get() * 100.0))
                                                .text_size(px(11.0))
                                                .text_color(ACCENT_GOLD),
                                        ),
                                ),
                        )
                        .child(
                            view()
                                .w_full()
                                .px(px(2.0))
                                .child(
                                    slider()
                                        .min(f64::from(MIN_OPACITY) * 100.0)
                                        .max(100.0)
                                        .step(1.0)
                                        .value(move || SliderValue::Single((opacity.get() * 100.0).round() as f64))
                                        .on_change(move |v: &SliderValue| {
                                            opacity.set(clamp_opacity((v.start() / 100.0) as f32));
                                        })
                                        .accessibility_label("窗口不透明度")
                                        .track_style(|s| s.w_full().h(px(20.0)).justify_center())
                                        .indicator_style(move |s| {
                                            s.h(px(4.0)).rounded_full().bg(theme().muted)
                                        })
                                        .range_style(move |s| {
                                            s.rounded_full().bg(ACCENT_GOLD)
                                        })
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
                // 3. 基于 Collapsible 的常用推荐区域：支持平滑折叠，预设 6 项精炼对称双列卡片排版
                .child(
                    collapsible()
                        .open(show_presets)
                        .reveal(true)
                        .child(
                            view()
                                .w_full()
                                .px(px(8.0))
                                .py(px(4.0))
                                .rounded(px(5.0))
                                .bg(move || theme().secondary)
                                .border_1()
                                .border_color(move || {
                                    if show_presets.get() {
                                        ACCENT_GOLD
                                    } else {
                                        theme().border
                                    }
                                })
                                .active(|s| s.opacity(0.6))
                                .on_press(move || show_presets.update(|v| *v = !*v))
                                .flex_row()
                                .items_center()
                                .justify_between()
                                .child(
                                    view()
                                        .flex_row()
                                        .items_center()
                                        .gap(px(5.0))
                                        .child(icons::icon_svg(
                                            icons::ICON_STAR,
                                            11.5,
                                            ACCENT_GOLD,
                                        ))
                                        .child(
                                            text("常用推荐")
                                                .text_size(px(11.0))
                                                .text_color(ACCENT_GOLD),
                                        )
                                        .child(
                                            text("(6项预设)")
                                                .text_size(px(10.0))
                                                .text_color(move || theme().muted_foreground),
                                        ),
                                )
                                .child(
                                    view()
                                        .flex_row()
                                        .items_center()
                                        .gap(px(3.0))
                                        .child(
                                            text(move || if show_presets.get() { "收起" } else { "展开" })
                                                .text_size(px(10.5))
                                                .text_color(move || theme().muted_foreground),
                                        )
                                        .child(
                                            icons::dynamic_icon_svg(
                                                move || {
                                                    if show_presets.get() {
                                                        icons::ICON_CHEVRON_UP
                                                    } else {
                                                        icons::ICON_CHEVRON_DOWN
                                                    }
                                                },
                                                11.0,
                                                move || theme().muted_foreground,
                                            ),
                                        ),
                                ),
                        )
                        .content(
                            view()
                                .w_full()
                                .mt(px(4.0))
                                .p(px(6.0))
                                .rounded(px(6.0))
                                .bg(move || theme().card)
                                .border_1()
                                .border_color(move || theme().border)
                                .flex_col()
                                .gap(px(4.5))
                                .children(PRESETS.chunks(2).map(|pair| {
                                    view()
                                        .w_full()
                                        .flex_row()
                                        .gap(px(4.5))
                                        .children(pair.iter().map(|preset| {
                                            render_preset_card(preset, items, on_save)
                                        }))
                                })),
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
                // 4. 基于 Collapsible 的自定义新提醒区域：默认收起，点击平滑展开完整配置面板
                .child(
                    collapsible()
                        .open(show_custom_form)
                        .reveal(true)
                        .child(
                            view()
                                .w_full()
                                .px(px(8.0))
                                .py(px(4.0))
                                .rounded(px(5.0))
                                .bg(move || theme().secondary)
                                .border_1()
                                .border_color(move || {
                                    if show_custom_form.get() {
                                        ACCENT_GOLD
                                    } else {
                                        theme().border
                                    }
                                })
                                .active(|s| s.opacity(0.6))
                                .on_press(move || show_custom_form.update(|v| *v = !*v))
                                .flex_row()
                                .items_center()
                                .justify_between()
                                .child(
                                    view()
                                        .flex_row()
                                        .items_center()
                                        .gap(px(5.0))
                                        .child(icons::icon_svg(
                                            icons::ICON_EDIT,
                                            11.5,
                                            ACCENT_GOLD,
                                        ))
                                        .child(
                                            text("自定义新提醒")
                                                .text_size(px(11.0))
                                                .text_color(ACCENT_GOLD),
                                        ),
                                )
                                .child(
                                    view()
                                        .flex_row()
                                        .items_center()
                                        .gap(px(3.0))
                                        .child(
                                            text(move || if show_custom_form.get() { "收起" } else { "配置" })
                                                .text_size(px(10.5))
                                                .text_color(move || theme().muted_foreground),
                                        )
                                        .child(
                                            icons::dynamic_icon_svg(
                                                move || {
                                                    if show_custom_form.get() {
                                                        icons::ICON_CHEVRON_UP
                                                    } else {
                                                        icons::ICON_CHEVRON_DOWN
                                                    }
                                                },
                                                11.0,
                                                move || theme().muted_foreground,
                                            ),
                                        ),
                                ),
                        )
                        .content(
                            view()
                                .w_full()
                                .mt(px(4.0))
                                .p(px(8.0))
                                .rounded(px(6.0))
                                .bg(move || theme().card)
                                .border_1()
                                .border_color(move || theme().border)
                                .flex_col()
                                .gap(px(6.0))
                                // Field 1: 提醒名称
                                .child(
                                    view()
                                        .flex_col()
                                        .gap(px(3.0))
                                        .child(
                                            view()
                                                .flex_row()
                                                .items_center()
                                                .justify_between()
                                                .child(
                                                    text("提醒事项名称")
                                                        .text_size(px(10.0))
                                                        .text_color(move || theme().muted_foreground),
                                                )
                                                .child(
                                                    text(move || error_tip.get().unwrap_or_default())
                                                        .text_size(px(10.0))
                                                        .text_color(move || theme().destructive),
                                                ),
                                        )
                                        .child(
                                            input()
                                                .placeholder("如：站姿切换、滴眼药水、工位通风")
                                                .value(move || custom_name.get())
                                                .on_change(move |s: &str| {
                                                    custom_name.set(s.to_string());
                                                    error_tip.set(None);
                                                })
                                                .w_full()
                                                .h(px(25.0))
                                                .px(px(8.0))
                                                .rounded(px(4.0))
                                                .bg(move || theme().background)
                                                .border_1()
                                                .border_color(move || theme().input)
                                                .text_size(px(11.0))
                                                .text_color(move || theme().foreground),
                                        ),
                                )
                                // Field 2: 循环周期（键盘直接输入 + 1分钟微调 + 快捷填入）
                                .child(
                                    view()
                                        .flex_col()
                                        .gap(px(3.0))
                                        .child(
                                            view()
                                                .flex_row()
                                                .items_center()
                                                .justify_between()
                                                .child(
                                                    text("循环周期")
                                                        .text_size(px(10.0))
                                                        .text_color(move || theme().muted_foreground),
                                                )
                                                .child(
                                                    view()
                                                        .flex_row()
                                                        .items_center()
                                                        .gap(px(3.0))
                                                        // 减 1 分钟
                                                        .child(
                                                            view()
                                                                .w(px(18.0))
                                                                .h(px(19.0))
                                                                .rounded(px(3.0))
                                                                .bg(move || theme().secondary)
                                                                .items_center()
                                                                .justify_center()
                                                                .active(|s| s.opacity(0.5))
                                                                .on_press(move || {
                                                                    let cur = custom_mins_str.get().parse::<u32>().unwrap_or(30);
                                                                    let next = cur.saturating_sub(1).max(1);
                                                                    custom_mins_str.set(next.to_string());
                                                                    error_tip.set(None);
                                                                })
                                                                .child(text("-").text_size(px(11.0)).text_color(move || theme().secondary_foreground)),
                                                        )
                                                        // 数字文本输入框：支持点击键盘任意输入
                                                        .child(
                                                            input()
                                                                .placeholder("30")
                                                                .value(move || custom_mins_str.get())
                                                                .on_change(move |s: &str| {
                                                                    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).take(4).collect();
                                                                    custom_mins_str.set(digits);
                                                                    error_tip.set(None);
                                                                })
                                                                .w(px(38.0))
                                                                .h(px(19.0))
                                                                .px(px(4.0))
                                                                .rounded(px(3.0))
                                                                .bg(move || theme().background)
                                                                .border_1()
                                                                .border_color(ACCENT_GOLD)
                                                                .text_size(px(11.0))
                                                                .text_color(ACCENT_GOLD),
                                                        )
                                                        // 加 1 分钟
                                                        .child(
                                                            view()
                                                                .w(px(18.0))
                                                                .h(px(19.0))
                                                                .rounded(px(3.0))
                                                                .bg(move || theme().secondary)
                                                                .items_center()
                                                                .justify_center()
                                                                .active(|s| s.opacity(0.5))
                                                                .on_press(move || {
                                                                    let cur = custom_mins_str.get().parse::<u32>().unwrap_or(30);
                                                                    let next = (cur + 1).min(1440);
                                                                    custom_mins_str.set(next.to_string());
                                                                    error_tip.set(None);
                                                                })
                                                                .child(text("+").text_size(px(11.0)).text_color(move || theme().secondary_foreground)),
                                                        )
                                                        .child(
                                                            text("分钟")
                                                                .text_size(px(10.0))
                                                                .text_color(move || theme().muted_foreground),
                                                        ),
                                                ),
                                        )
                                        // 快捷填入颗粒度胶囊
                                        .child(
                                            view()
                                                .flex_row()
                                                .items_center()
                                                .justify_between()
                                                .gap(px(3.0))
                                                .child(
                                                    text("快捷填入:")
                                                        .text_size(px(9.5))
                                                        .text_color(move || theme().muted_foreground),
                                                )
                                                .child(
                                                    view()
                                                        .flex_row()
                                                        .gap(px(3.0))
                                                        .children([15, 25, 30, 45, 60, 90].iter().map(|&mins| {
                                                            let is_sel = computed(move || custom_mins_str.get() == mins.to_string());
                                                            view()
                                                                .px(px(5.0))
                                                                .py(px(1.5))
                                                                .rounded(px(3.0))
                                                                .bg(move || {
                                                                    if is_sel.get() {
                                                                        ACCENT_GOLD_BG
                                                                    } else {
                                                                        theme().secondary
                                                                    }
                                                                })
                                                                .border_1()
                                                                .border_color(move || {
                                                                    if is_sel.get() {
                                                                        ACCENT_GOLD
                                                                    } else {
                                                                        theme().border
                                                                    }
                                                                })
                                                                .active(|s| s.opacity(0.6))
                                                                .on_press(move || {
                                                                    custom_mins_str.set(mins.to_string());
                                                                    error_tip.set(None);
                                                                })
                                                                .child(
                                                                    text(format!("{}m", mins))
                                                                        .text_size(px(9.5))
                                                                        .text_color(move || {
                                                                            if is_sel.get() {
                                                                                ACCENT_GOLD
                                                                            } else {
                                                                                theme().muted_foreground
                                                                            }
                                                                        }),
                                                                )
                                                        })),
                                                ),
                                        ),
                                )
                                // Field 3: 图标分类与高光色（16 种丰富主题，两排 8 列精致排布）
                                .child(
                                    view()
                                        .flex_col()
                                        .gap(px(3.0))
                                        .child(
                                            text("图标主题")
                                                .text_size(px(10.0))
                                                .text_color(move || theme().muted_foreground),
                                        )
                                        .child(
                                            view()
                                                .flex_col()
                                                .gap(px(3.0))
                                                // 第一排：生活与健康 (喝水、咖啡、活动、拉伸、护眼、服药、静心、暖阳)
                                                .child(
                                                    view()
                                                        .flex_row()
                                                        .items_center()
                                                        .justify_between()
                                                        .children([
                                                            "droplet", "coffee", "walk", "activity",
                                                            "eye", "pill", "heart", "sun",
                                                        ].iter().map(|&ic| {
                                                            let is_sel = computed(move || custom_icon.get() == ic);
                                                            view()
                                                                .w(px(24.0))
                                                                .h(px(24.0))
                                                                .rounded(px(4.0))
                                                                .bg(move || {
                                                                    if is_sel.get() {
                                                                        icons::get_item_adaptive(ic).opacity(0.20)
                                                                    } else {
                                                                        theme().secondary
                                                                    }
                                                                })
                                                                .border_1()
                                                                .border_color(move || {
                                                                    if is_sel.get() {
                                                                        icons::get_item_adaptive(ic)
                                                                    } else {
                                                                        theme().border
                                                                    }
                                                                })
                                                                .items_center()
                                                                .justify_center()
                                                                .active(|s| s.opacity(0.5))
                                                                .on_press(move || custom_icon.set(ic.to_string()))
                                                                .child(
                                                                    icons::icon_svg(
                                                                        icons::get_icon_markup(ic),
                                                                        12.0,
                                                                        move || {
                                                                            if is_sel.get() {
                                                                                icons::get_item_adaptive(ic)
                                                                            } else {
                                                                                theme().muted_foreground
                                                                            }
                                                                        },
                                                                    ),
                                                                )
                                                        })),
                                                )
                                                // 第二排：专注与日程 (番茄、阅读、办公、目标、小憩、轻食、排便、闹钟)
                                                .child(
                                                    view()
                                                        .flex_row()
                                                        .items_center()
                                                        .justify_between()
                                                        .children([
                                                            "clock", "book", "laptop", "target",
                                                            "moon", "apple", "toilet", "bell",
                                                        ].iter().map(|&ic| {
                                                            let is_sel = computed(move || custom_icon.get() == ic);
                                                            view()
                                                                .w(px(24.0))
                                                                .h(px(24.0))
                                                                .rounded(px(4.0))
                                                                .bg(move || {
                                                                    if is_sel.get() {
                                                                        icons::get_item_adaptive(ic).opacity(0.20)
                                                                    } else {
                                                                        theme().secondary
                                                                    }
                                                                })
                                                                .border_1()
                                                                .border_color(move || {
                                                                    if is_sel.get() {
                                                                        icons::get_item_adaptive(ic)
                                                                    } else {
                                                                        theme().border
                                                                    }
                                                                })
                                                                .items_center()
                                                                .justify_center()
                                                                .active(|s| s.opacity(0.5))
                                                                .on_press(move || custom_icon.set(ic.to_string()))
                                                                .child(
                                                                    icons::icon_svg(
                                                                        icons::get_icon_markup(ic),
                                                                        12.0,
                                                                        move || {
                                                                            if is_sel.get() {
                                                                                icons::get_item_adaptive(ic)
                                                                            } else {
                                                                                theme().muted_foreground
                                                                            }
                                                                        },
                                                                    ),
                                                                )
                                                        })),
                                                ),
                                        ),
                                )
                                // Field 4: 底部操作栏
                                .child(
                                    view()
                                        .flex_row()
                                        .items_center()
                                        .justify_between()
                                        .pt(px(2.0))
                                        .child(
                                            view()
                                                .px(px(8.0))
                                                .h(px(22.0))
                                                .rounded(px(4.0))
                                                .bg(move || theme().secondary)
                                                .items_center()
                                                .justify_center()
                                                .active(|s| s.opacity(0.5))
                                                .on_press(move || {
                                                    show_custom_form.set(false);
                                                    error_tip.set(None);
                                                })
                                                .child(
                                                    text("收起")
                                                        .text_size(px(10.5))
                                                        .text_color(move || theme().muted_foreground),
                                                ),
                                        )
                                        .child(
                                            view()
                                                .px(px(12.0))
                                                .h(px(22.0))
                                                .rounded(px(4.0))
                                                .bg(ACCENT_GOLD)
                                                .items_center()
                                                .justify_center()
                                                .active(|s| s.opacity(0.6))
                                                .on_press(move || {
                                                    let name = custom_name.get().trim().to_string();
                                                    if name.is_empty() {
                                                        error_tip.set(Some("请输入名称".to_string()));
                                                        return;
                                                    }
                                                    let mins = match custom_mins_str.get().trim().parse::<u32>() {
                                                        Ok(m) if m > 0 => m,
                                                        _ => {
                                                            error_tip.set(Some("请输入有效分钟数(≥1)".to_string()));
                                                            return;
                                                        }
                                                    };
                                                    if mins > 1440 {
                                                        error_tip.set(Some("时长上限为 1440 分钟 (24小时)".to_string()));
                                                        return;
                                                    }
                                                    if items.with(|list| list.iter().any(|x| x.name == name)) {
                                                        error_tip.set(Some("该提醒已在列表中".to_string()));
                                                        return;
                                                    }
                                                    items.update(|list| {
                                                        list.push(ReminderItem::new(
                                                            &name,
                                                            custom_icon.get(),
                                                            mins * 60,
                                                        ));
                                                    });
                                                    on_save();
                                                    custom_name.set(String::new());
                                                    error_tip.set(None);
                                                    show_custom_form.set(false);
                                                })
                                                .child(
                                                    text("开启提醒")
                                                        .text_size(px(11.0))
                                                        .text_color(AdaptiveColor::rgb(0xffffff, 0x000000)),
                                                ),
                                        ),
                                ),
                        ),
                ),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_dimension_constants() {
        assert!(MINI_WIDTH < WINDOW_WIDTH);
        assert!(MINI_HEIGHT < WINDOW_HEIGHT);
        assert_eq!(MINI_WIDTH, 196.0);
        assert_eq!(MINI_HEIGHT, 44.0);
    }

    #[test]
    fn test_opacity_clamp_range() {
        assert_eq!(clamp_opacity(0.0), MIN_OPACITY);
        assert_eq!(clamp_opacity(0.15), MIN_OPACITY);
        assert_eq!(clamp_opacity(0.85), 0.85);
        assert_eq!(clamp_opacity(1.0), 1.0);
        assert_eq!(clamp_opacity(1.5), 1.0);
    }

    #[test]
    fn test_input_element() {
        use nsh_core::element::input;
        let _ = input().placeholder("测试输入");
    }

    #[test]
    fn test_full_persistence_integration() {
        let temp_dir = std::env::temp_dir().join(format!("reminder_integ_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let test_file = temp_dir.join("state.json");

        // 一份窗口摆放样本（`WindowPlacement` 为 non_exhaustive，Default + 字段赋值）
        let mut placement = nsh_core::desktop::WindowPlacement::default();
        placement.bounds = Bounds::new(point(px(500.0), px(300.0)), size(px(320.0), px(350.0)));

        let mut state = storage::AppState::default();
        state.opacity = 0.75;
        state.theme_mode = ThemeMode::Light;
        state.window_placement = Some(placement);
        state.is_mini = true;
        state.items.push(ReminderItem::new("自定义喝咖啡", "coffee", 1800));
        state.items[0].completed_count = 12;

        let res = storage::save_state_to_path(&mut state, &test_file);
        assert!(res.is_ok());

        let loaded = storage::load_state_from_path(&test_file);
        assert_eq!(loaded.opacity, 0.75);
        assert_eq!(loaded.theme_mode, ThemeMode::Light);
        assert_eq!(loaded.window_placement, Some(placement));
        assert!(loaded.is_mini);
        assert_eq!(loaded.items.len(), 4);
        assert_eq!(loaded.items[0].completed_count, 12);
        assert_eq!(loaded.items[3].name, "自定义喝咖啡");

        let _ = std::fs::remove_file(&test_file);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
