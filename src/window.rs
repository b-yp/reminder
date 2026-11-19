//! 平台窗口层：不透明度、无边框悬浮窗配置、Windows/macOS 原生窗口句柄与图标、托盘语义。
//!
//! 这一层只和 `nsh_core::desktop` 与平台 crate 打交道，不认识任何提醒业务；
//! 视图层（`views`）不该导入这里的任何平台类型。

use nsh_core::desktop::{WindowAttributes, WindowFrameStyle, WindowLevel};
use nsh_core::element::desktop;
use nsh_core::prelude::*;

#[cfg(target_os = "macos")]
use nsh_platform_macos as platform;
#[cfg(target_os = "windows")]
use nsh_platform_windows as platform;

#[cfg(target_os = "windows")]
static APP_HWND: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

/// 窗口不透明度下限：再低就看不见挂件了（设置里的滑杆下限与之一致）
pub(crate) const MIN_OPACITY: f32 = 0.20;

/// 把任意输入收敛到可用的不透明度区间
pub(crate) fn clamp_opacity(opacity: f32) -> f32 {
    opacity.clamp(MIN_OPACITY, 1.0)
}

/// 设置窗口整体不透明度：Windows 用分层窗口 alpha，macOS 用 `NSWindow.alphaValue`。
/// HWND 尚未就绪时（挂载最初几十毫秒）静默跳过，由 `setup_floating_window` 在拿到句柄后补上初值。
#[cfg(target_os = "windows")]
pub(crate) fn set_window_opacity(opacity: f32) {
    let opacity = clamp_opacity(opacity);
    let raw = APP_HWND.load(std::sync::atomic::Ordering::SeqCst);
    if raw == 0 {
        return;
    }
    use platform::windows::Win32::Foundation::{COLORREF, HWND};
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
pub(crate) fn set_window_opacity(opacity: f32) {
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
pub(crate) fn set_window_opacity(_opacity: f32) {}

/// macOS 挂件专用配置：在主线程剥离窗口边框、隐藏交通灯，并让原生阴影跟随圆角卡片
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

        // 3. 窗口底色再压一次透明：真正生效的是 `WindowAttributes::transparent`
        //    （NSH 会连根容器视图的底色一起清掉），这里防止改 styleMask 后 AppKit 恢复默认底色
        window.setOpaque(false);
        window.setBackgroundColor(Some(&NSColor::clearColor()));

        // 4. 原生阴影跟着窗口形状（圆角卡片）走：内容画出来后重新失效一次
        window.setHasShadow(true);
        window.setMovableByWindowBackground(true);
        window.invalidateShadow();
    }
}

/// macOS 挂件专用：配置为纯净无边框悬浮卡片，移除全部系统控制按钮与标题栏非客户区，保留原生圆角与阴影
#[cfg(target_os = "macos")]
pub(crate) fn setup_floating_window(_initial_opacity: f32) {
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
pub(crate) fn setup_floating_window(initial_opacity: f32) {
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
            let _ = unsafe {
                EnumWindows(Some(enum_proc), LPARAM(&mut target_hwnd as *mut _ as isize))
            };

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

/// 其他平台没有需要额外处理的原生窗口句柄：保持挂件默认行为。
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub(crate) fn setup_floating_window(_initial_opacity: f32) {}

/// 窗口图标文件：`nsh run` 的 runner 只在 `$NSH_APP_DIR/icon.ico` 找图标，而 `nsh build` 不生成它，
/// 所以退一步用已随 bundle 拷贝的 `assets/@<package>/app.ico`；`cargo run` 则用源码目录里的那份。
#[cfg(target_os = "windows")]
fn window_icon_path() -> Option<std::path::PathBuf> {
    use std::path::PathBuf;
    let bundled = PathBuf::from("assets")
        .join(format!("@{}", env!("CARGO_PKG_NAME")))
        .join("app.ico");
    let nsh_dir = std::env::var_os("NSH_APP_DIR")
        .map(PathBuf::from)
        .or_else(|| {
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
        let Ok(icon) =
            (unsafe { LoadImageW(None, &path, IMAGE_ICON, side, side, LR_LOADFROMFILE) })
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

/// 挂件窗口的 NSH 属性：置顶悬浮 + 无系统边框，并设极小 min_size 防止 Win32 限制最小追踪尺寸
/// （`WindowAttributes` 为 `non_exhaustive`，故用 Default 赋值而非结构体字面量）
pub(crate) fn floating_window_attributes() -> WindowAttributes {
    let mut attributes = WindowAttributes::default();
    attributes.level = Some(WindowLevel::Floating);
    attributes.frame = Some(WindowFrameStyle::None);
    attributes.min_size = Some(Some(size(px(100.0), px(36.0))));
    // 关闭请求先交给 App：挂件不是普通文档窗口，点叉号只收进托盘，不结束进程
    attributes.close_on_press = Some(false);
    // macOS 没有系统托管的窗口圆角（Win11 的 DWM 会裁掉四角），窗口自带的那层
    // 不透明底色会把根视图圆角外的四角填成直角。交给 NSH 的 transparent：
    // 它同时清掉窗口与容器视图的底色，四角才真正透出桌面。
    #[cfg(target_os = "macos")]
    {
        attributes.transparent = Some(true);
    }
    attributes
}

/// 顶栏叉号、Alt+F4、任务栏关闭等请求都只收进托盘，绝不结束进程。
/// 真正退出只放在托盘菜单里；系统关机 / 注销由平台结束进程，不需要窗口先关。
pub(crate) fn hide_to_tray() {
    desktop::main_window().hide();
}

/// 托盘菜单里的退出项：只写「退出」；退出后进程结束，提醒自然停止。
pub(crate) const TRAY_EXIT_LABEL: &str = "退出";

/// 托盘菜单第一项跟随窗口可见性：看得见就给「隐藏窗口」，看不见就给「显示窗口」。
pub(crate) fn tray_window_label(visible: bool) -> &'static str {
    if visible {
        "隐藏窗口"
    } else {
        "显示窗口"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// macOS 没有系统托管的窗口圆角：窗口必须声明透明，根视图圆角外的四角才不会被
    /// 窗口底色填成直角（Windows 由 DWM 裁剪窗口四角，不需要）。
    #[cfg(target_os = "macos")]
    #[test]
    fn macos_widget_window_is_transparent() {
        assert_eq!(floating_window_attributes().transparent, Some(true));
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
    fn close_button_is_reported_to_the_app_instead_of_closing_the_window() {
        // 平台先报告 closePress，由 App 收进托盘；这是「点叉号不退出」的底层开关
        assert_eq!(floating_window_attributes().close_on_press, Some(false));
    }

    #[test]
    fn tray_window_label_follows_visibility() {
        assert_eq!(tray_window_label(true), "隐藏窗口");
        assert_eq!(tray_window_label(false), "显示窗口");
    }

    #[test]
    fn tray_exit_label_is_just_quit() {
        assert_eq!(TRAY_EXIT_LABEL, "退出");
    }
}
