//! 全局热键模块：系统级快捷键一键隐藏 / 呼出悬浮窗。
//!
//! 组合键沿用 NSH 的快捷键写法（默认为 `mod+alt+r`，macOS 显示为 `⌥⌘R`）：
//! `mod` 在 macOS 上是 Command，其他平台是 Control。NSH 把这些按键注册给操作系统，
//! 因此无论前台是哪个应用，按下组合键都能收到回调，挂件无需抢焦点。

use std::time::Duration;

use nsh_core::desktop::Shortcut;
use nsh_core::element::desktop;
use nsh_core::prelude::*;

/// 默认组合键：Ctrl / ⌘ + Alt + R（R 代表 Reminder）
pub const DEFAULT_HOTKEY: &str = "mod+alt+r";

/// 设置里「换一组」轮换的候选组合键，默认值排第一。
/// 都带 `mod`，避开只有单个字母的全局按键（那会吞掉所有应用的普通输入）。
pub const HOTKEY_PRESETS: &[&str] = &["mod+alt+r", "mod+shift+space", "mod+alt+p"];

/// 全局热键当前的可注册状态，用于设置抽屉里的状态徽章。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HotkeyStatus {
    /// 已提交给系统，等待确认
    Pending,
    /// 系统已接受，按键已生效
    Active,
    /// 组合键被其他应用占用，或系统拒绝注册
    Taken,
    /// 用户关闭了全局热键
    Disabled,
    /// 当前平台没有全局热键（Web 与移动端）
    Unsupported,
}

impl HotkeyStatus {
    /// 状态徽章文字
    pub fn label(self) -> &'static str {
        match self {
            HotkeyStatus::Pending => "检测中",
            HotkeyStatus::Active => "已生效",
            HotkeyStatus::Taken => "被占用",
            HotkeyStatus::Disabled => "已关闭",
            HotkeyStatus::Unsupported => "不可用",
        }
    }

    /// 状态下面的说明文字
    pub fn hint(self) -> &'static str {
        match self {
            HotkeyStatus::Pending => "正在向系统注册全局热键…",
            HotkeyStatus::Active => "任意应用中按下即可隐藏 / 呼出悬浮窗",
            HotkeyStatus::Taken => "该组合键已被占用，换一组试试",
            HotkeyStatus::Disabled => "已关闭，悬浮窗保持常显",
            HotkeyStatus::Unsupported => "当前平台不支持全局热键",
        }
    }

    /// 是否是「需要用户处理」的状态（用警示色显示）
    pub fn is_error(self) -> bool {
        matches!(self, HotkeyStatus::Taken)
    }
}

/// 本机把 `mod` 读作 Command 还是 Control（与 `register_global_shortcut` 一致）
fn mod_is_meta() -> bool {
    cfg!(target_os = "macos")
}

/// 解析组合键；不是合法快捷键时返回 `None`
pub fn parse(keystroke: &str) -> Option<Shortcut> {
    parse_on(keystroke, mod_is_meta())
}

/// 按指定平台解析组合键（测试里同时覆盖 macOS 与 Windows 的写法）
fn parse_on(keystroke: &str, mac: bool) -> Option<Shortcut> {
    Shortcut::parse(keystroke.trim(), mac).ok()
}

/// 规范化持久化里读到的组合键：统一小写，非法或为空时退回默认值
pub fn normalize(keystroke: &str) -> String {
    let text = keystroke.trim();
    match parse(text) {
        Some(_) => text.to_ascii_lowercase(),
        None => DEFAULT_HOTKEY.to_string(),
    }
}

/// 组合键的键帽文字。修饰键按平台习惯排序：
/// macOS 为 Control → Option → Shift → Command（`["⌥", "⌘", "R"]`，与系统菜单一致），
/// 其他平台为 Ctrl → Alt → Shift → Win（`["Ctrl", "Alt", "R"]`）。
pub fn keycaps(keystroke: &str) -> Vec<String> {
    keycaps_on(keystroke, mod_is_meta())
}

/// 按指定平台生成键帽文字（见 [`keycaps`]）
fn keycaps_on(keystroke: &str, mac: bool) -> Vec<String> {
    let Some(shortcut) = parse_on(keystroke, mac) else {
        // 非法组合键按默认值展示，与 `normalize` 的退化方向一致
        return keycaps_on(DEFAULT_HOTKEY, mac);
    };
    let mut caps = Vec::with_capacity(5);
    // Apple HIG 与 Chromium 的 macOS 规范：⌃⌥⇧⌘，Command 放在最后
    if shortcut.ctrl {
        caps.push(if mac { "⌃".to_string() } else { "Ctrl".to_string() });
    }
    if shortcut.alt {
        caps.push(if mac { "⌥".to_string() } else { "Alt".to_string() });
    }
    if shortcut.shift {
        caps.push(if mac { "⇧".to_string() } else { "Shift".to_string() });
    }
    if shortcut.meta {
        caps.push(if mac { "⌘".to_string() } else { "Win".to_string() });
    }
    caps.push(shortcut.key.label());
    caps
}

/// 组合键的完整文字标签（页脚提示用）：macOS 按系统习惯写作 `⌥⌘R`（不加 `+`），
/// 其他平台写作 `Ctrl+Alt+R`。
pub fn label(keystroke: &str) -> String {
    label_on(keystroke, mod_is_meta())
}

/// 按指定平台生成组合键标签（见 [`label`]）
fn label_on(keystroke: &str, mac: bool) -> String {
    let separator = if mac { "" } else { "+" };
    keycaps_on(keystroke, mac).join(separator)
}

/// 轮换到下一组候选组合键；当前值不在候选里时从头开始
pub fn next_preset(current: &str) -> &'static str {
    match HOTKEY_PRESETS.iter().position(|preset| *preset == current) {
        Some(index) => HOTKEY_PRESETS[(index + 1) % HOTKEY_PRESETS.len()],
        None => HOTKEY_PRESETS[0],
    }
}

/// 按下热键时对窗口做的事
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowToggle {
    /// 窗口可见：收起来
    Hide,
    /// 窗口不可见（隐藏、最小化或还没打开）：呼出并聚焦
    Show,
}

/// 根据当前可见性决定隐藏还是呼出
pub fn toggle_action(visible: bool) -> WindowToggle {
    if visible {
        WindowToggle::Hide
    } else {
        WindowToggle::Show
    }
}

/// 热键回调：可见就收起，隐藏就呼出并聚焦。挂件被隐藏后仍可从托盘图标找回。
pub fn toggle_floating_window() {
    let window = desktop::main_window();
    let visible = window.state().is_some_and(|state| state.visible);
    match toggle_action(visible) {
        WindowToggle::Hide => window.hide(),
        WindowToggle::Show => {
            window.show();
            window.focus();
        }
    }
}

/// 把 `combo` 注册为全局热键，并把结果写进 `status`。
///
/// 挂载最初几十毫秒里平台窗口可能还没就绪（`RegisterHotKey` 会因此失败），
/// 所以失败后短暂等待再重试 `attempts_left - 1` 次；每次重试前核对
/// `generations`，组合键已经换过就不再补注册旧按键。
pub fn register(
    combo: String,
    generation: u64,
    generations: Stored<u64>,
    status: Signal<HotkeyStatus>,
    attempts_left: u8,
) {
    spawn(async move {
        let mut attempts_left = attempts_left.max(1);
        loop {
            let answer = desktop::register_global_shortcut(&combo, toggle_floating_window);
            let registered = answer.await;
            if generations.get() != generation {
                return;
            }
            if registered {
                status.set(HotkeyStatus::Active);
                return;
            }
            attempts_left -= 1;
            if attempts_left == 0 {
                status.set(HotkeyStatus::Taken);
                return;
            }
            timer(Duration::from_millis(300)).await;
            if generations.get() != generation {
                return;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 两个平台的修饰键顺序都由 `*_on` 显式断言，不依赖测试所在的主机。
    #[test]
    fn keycaps_follow_each_platforms_modifier_order() {
        // Windows / Linux：Ctrl → Alt → Shift → Win
        assert_eq!(
            keycaps_on(DEFAULT_HOTKEY, false),
            vec!["Ctrl", "Alt", "R"]
        );
        assert_eq!(label_on(DEFAULT_HOTKEY, false), "Ctrl+Alt+R");
        assert_eq!(
            keycaps_on("ctrl+alt+shift+cmd+k", false),
            vec!["Ctrl", "Alt", "Shift", "Win", "K"]
        );

        // macOS：⌃ → ⌥ → ⇧ → ⌘，Command 在最后（Apple HIG / Chromium 规范）
        assert_eq!(keycaps_on(DEFAULT_HOTKEY, true), vec!["⌥", "⌘", "R"]);
        assert_eq!(label_on(DEFAULT_HOTKEY, true), "⌥⌘R");
        assert_eq!(
            keycaps_on("ctrl+alt+shift+cmd+k", true),
            vec!["⌃", "⌥", "⇧", "⌘", "K"]
        );

        // 平台入口与对应实现一致
        let mac = mod_is_meta();
        assert_eq!(keycaps(DEFAULT_HOTKEY), keycaps_on(DEFAULT_HOTKEY, mac));
        assert_eq!(label(DEFAULT_HOTKEY), label_on(DEFAULT_HOTKEY, mac));
    }

    #[test]
    fn default_hotkey_is_a_valid_modifier_combo() {
        assert!(parse(DEFAULT_HOTKEY).is_some());
        assert!(parse_on(DEFAULT_HOTKEY, true).is_some());
        // mac 上 `mod` 是 Command，不是 Control
        let shortcut = parse_on(DEFAULT_HOTKEY, true).unwrap();
        assert!(shortcut.meta && shortcut.alt && !shortcut.ctrl);
    }

    #[test]
    fn every_preset_parses_and_labels_every_key() {
        for mac in [false, true] {
            for preset in HOTKEY_PRESETS {
                let caps = keycaps_on(preset, mac);
                assert!(caps.len() >= 2, "{preset} has no modifier");
                assert!(caps.iter().all(|cap| !cap.is_empty()));
                assert_eq!(label_on(preset, mac), caps.join(if mac { "" } else { "+" }));
            }
        }
    }

    #[test]
    fn normalize_keeps_valid_combos_and_falls_back_otherwise() {
        assert_eq!(normalize(" MOD+ALT+R "), "mod+alt+r");
        assert_eq!(normalize(""), DEFAULT_HOTKEY);
        assert_eq!(normalize("not+a+shortcut"), DEFAULT_HOTKEY);
        assert_eq!(normalize("ctrl+"), DEFAULT_HOTKEY);
    }

    #[test]
    fn next_preset_cycles_and_restarts() {
        let first = HOTKEY_PRESETS[0];
        let second = HOTKEY_PRESETS[1];
        assert_eq!(next_preset(first), second);
        assert_eq!(next_preset(second), HOTKEY_PRESETS[2]);
        assert_eq!(
            next_preset(HOTKEY_PRESETS[HOTKEY_PRESETS.len() - 1]),
            first
        );
        assert_eq!(next_preset("cmd+shift+k"), first);
    }

    #[test]
    fn visibility_decides_the_action() {
        assert_eq!(toggle_action(true), WindowToggle::Hide);
        assert_eq!(toggle_action(false), WindowToggle::Show);
    }

    #[test]
    fn statuses_explain_themselves() {
        for status in [
            HotkeyStatus::Pending,
            HotkeyStatus::Active,
            HotkeyStatus::Taken,
            HotkeyStatus::Disabled,
            HotkeyStatus::Unsupported,
        ] {
            assert!(!status.label().is_empty());
            assert!(!status.hint().is_empty());
        }
        assert!(HotkeyStatus::Taken.is_error());
        assert!(!HotkeyStatus::Active.is_error());
    }
}
