//! 到期提醒：系统提示音 + 原生通知 + 把挂件窗口唤到最前。
//!
//! 提醒文案（定点提醒带上「每天 12:00」）也只在这里写一份，避免各处拼字符串。

use nsh_core::element::desktop;

#[cfg(target_os = "windows")]
use nsh_platform_windows as platform;

use crate::model::ReminderItem;

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

/// 提醒文案：定点提醒带上「每天 12:00」，让人一眼知道是哪一次到点
pub(crate) fn alarm_label(item: &ReminderItem) -> String {
    if item.is_daily() {
        format!("{}（每天 {}）", item.name, item.format_interval())
    } else {
        item.name.clone()
    }
}
