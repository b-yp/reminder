//! 开机自启（登录时启动）：把挂件登记为登录系统后自动运行。
//!
//! 三个桌面平台各用系统自己的登记方式，不引入额外依赖：
//! - Windows：`HKCU\Software\Microsoft\Windows\CurrentVersion\Run` 下以 `APP_ID` 命名的值
//!   （用 Win32 注册表 API 读写，UTF-16 路径原样保存，不依赖 `reg.exe` 的输出编码）
//! - macOS：`~/Library/LaunchAgents/<APP_ID>.plist`（`RunAtLoad`，只加载于 Aqua 会话）
//! - Linux：`~/.config/autostart/<APP_ID>.desktop`（XDG Autostart 规范）
//!
//! 系统里的登记项就是唯一事实来源：启用状态每次启动直接问系统，不写进 `state.json`。
//! 这样用户在系统设置里手动关掉、或应用换了安装位置时，界面显示都与实际一致。
//!
//! ## 与 NSH 官方 API 的关系
//!
//! NSH 在 main 上已经有统一 API：TSX `Application.launchAtLogin()` /
//! `setLaunchAtLogin()`，Rust `nsh_core::element::desktop::{launch_at_login,
//! set_launch_at_login}`（PR longbridge/nsh#1884，2026-11-13 合入）。但它还没进任何
//! 发布版：0.5.1 是只修 bug 的补丁版，v0.5.1 的源码里没有 `login_item`，官网
//! 发布版文档也没有它（只有 `/versions/main` 文档有）。
//!
//! 官方实现还有一个限制：登录项只提供给 `nsh package` 打包后的独立 App，
//! `nsh run` 下 `launch_at_login()` 恒为 false、写入会报不可用。这里的平台层
//! 在开发运行与打包版行为一致，所以暂时保留。
//!
//! 等包含它的版本发布（并把 `nsh-*` 与 `nsh` CLI 一起升级，或改用 nightly）
//! 之后，删掉本文件的几个平台实现，改为在 UI 回调里 await 官方 API 即可，
//! 上层信号与文案都不用动。

// TODO(nsh): 官方统一 API 已在 main（PR longbridge/nsh#1884），但还没发布。
// 当 `nsh-core` 的某个发布版出现 `desktop/login_item.rs`（即
// `nsh_core::element::desktop::{launch_at_login, set_launch_at_login}`）时：
// 1. 删除本文件的 `windows_imp` / `macos_imp` / `linux_imp` 与 Cargo.toml 的 windows-sys；
// 2. `nsh-*` 依赖与 `nsh` CLI 同步升级到该版本；
// 3. 把 `is_enabled` / `set_enabled` 换成 await 官方 API，`AutostartState` 与设置抽屉 UI 不动。
// 注意：官方登录项只给打包后的独立 App，`nsh run` 下读 false、写入报不可用。

use std::path::Path;

/// 登录项里显示的名字
pub const DISPLAY_NAME: &str = "reminder";

fn unsupported_message() -> String {
    "当前平台不支持开机自启".to_string()
}

/// 桌面平台（macOS / Windows / Linux）才支持开机自启；Web 与移动端没有这个概念
pub fn is_supported() -> bool {
    cfg!(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "linux"
    ))
}

/// 本应用当前是否已登记为开机自启（以系统登记项为准）
pub fn is_enabled() -> bool {
    is_supported() && imp::is_enabled()
}

/// 写入或移除系统启动项。失败时返回可直接展示给用户的中文原因。
pub fn set_enabled(enabled: bool) -> Result<(), String> {
    if !is_supported() {
        return Err(unsupported_message());
    }
    imp::set_enabled(enabled)
}

/// 设置项当前的状态，决定徽章与说明文字
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutostartState {
    /// 已登记，登录后自动启动
    Enabled,
    /// 没有登记
    Disabled,
    /// Web / 移动端没有登录启动项
    Unsupported,
    /// 上次写入或移除被系统拒绝
    Failed,
}

impl AutostartState {
    /// 状态徽章文字
    pub fn label(self) -> &'static str {
        match self {
            AutostartState::Enabled => "已开启",
            AutostartState::Disabled => "已关闭",
            AutostartState::Unsupported => "不可用",
            AutostartState::Failed => "设置失败",
        }
    }

    /// 是否是「需要用户处理」的状态（用警示色显示）
    pub fn is_error(self) -> bool {
        matches!(self, AutostartState::Failed)
    }
}

/// 由「平台是否支持 + 系统登记结果 + 最近一次失败」得出状态
pub fn state(supported: bool, enabled: bool, error: Option<&str>) -> AutostartState {
    if !supported {
        AutostartState::Unsupported
    } else if error.is_some() {
        AutostartState::Failed
    } else if enabled {
        AutostartState::Enabled
    } else {
        AutostartState::Disabled
    }
}

/// 设置项下方的说明文字
pub fn hint(state: AutostartState, error: Option<&str>) -> String {
    match state {
        AutostartState::Unsupported => unsupported_message(),
        AutostartState::Failed => error
            .unwrap_or("系统拒绝了这次修改，请稍后重试")
            .to_string(),
        AutostartState::Enabled => "登录系统后自动启动，无需手动打开".to_string(),
        AutostartState::Disabled => "登录系统后自动运行挂件，重启不漏提醒".to_string(),
    }
}

/// 转义 XML 文本节点与属性值（LaunchAgent plist 的路径里可能有 `&`）
#[cfg(any(target_os = "macos", test))]
fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Windows 启动项里的命令行：路径带空格时必须加引号，登录时才不会被拆成多个参数
#[cfg(any(target_os = "windows", test))]
fn windows_run_command(exe: &Path) -> String {
    format!("\"{}\"", exe.display())
}

/// 判断启动项里登记的命令行是否就是 `exe`：
/// Windows 路径大小写不敏感，斜杠方向与引号写法也可能不同，统一后再比较
#[cfg(any(target_os = "windows", test))]
fn same_windows_path(recorded: &str, exe: &Path) -> bool {
    fn normalize(text: &str) -> String {
        text.trim()
            .trim_matches('"')
            .replace('/', "\\")
            .to_ascii_lowercase()
    }
    let recorded = normalize(recorded);
    !recorded.is_empty() && recorded == normalize(&exe.display().to_string())
}

/// XDG Autostart 的 `Exec=` 值：按 Desktop Entry 规范给路径加引号，
/// 并转义 `\`、`"`，`%` 要写成 `%%` 才不会被当成字段代码
#[cfg(any(target_os = "linux", test))]
fn desktop_exec(exe: &Path) -> String {
    let mut quoted = String::from("\"");
    for ch in exe.display().to_string().chars() {
        match ch {
            '"' => quoted.push_str("\\\""),
            '\\' => quoted.push_str("\\\\"),
            '%' => quoted.push_str("%%"),
            _ => quoted.push(ch),
        }
    }
    quoted.push('"');
    quoted
}

/// Linux 的 autostart 桌面入口文件内容
#[cfg(any(target_os = "linux", test))]
fn autostart_entry(exe: &Path) -> String {
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Version=1.0\n\
         Name={DISPLAY_NAME}\n\
         Comment=健康与生活提醒挂件\n\
         Exec={exec}\n\
         Terminal=false\n\
         StartupNotify=false\n\
         X-GNOME-Autostart-enabled=true\n",
        exec = desktop_exec(exe)
    )
}

/// macOS 的 LaunchAgent 内容：登录时在图形会话里运行这枚可执行文件
#[cfg(any(target_os = "macos", test))]
fn launch_agent_plist(exe: &Path) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{label}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{program}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>LimitLoadToSessionType</key>
    <string>Aqua</string>
    <key>ProcessType</key>
    <string>Interactive</string>
</dict>
</plist>
"#,
        label = xml_escape(crate::APP_ID),
        program = xml_escape(&exe.display().to_string())
    )
}

#[cfg(target_os = "windows")]
mod windows_imp {
    use std::path::PathBuf;

    use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, WIN32_ERROR};
    use windows_sys::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SAM_FLAGS, REG_SZ,
        RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW,
        RegSetValueExW,
    };

    use super::{same_windows_path, windows_run_command};

    /// 当前用户的登录启动项（无需管理员权限）
    pub(super) const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    /// 启动项里的值名，卸载或关闭时按它删除；与 macOS / Linux 的登记项同一个标识
    pub(super) const VALUE_NAME: &str = crate::APP_ID;

    pub(super) fn launch_target() -> Option<PathBuf> {
        std::env::current_exe().ok()
    }

    /// Rust 字符串 → NUL 结尾的 UTF-16（注册表 W 版 API 的入参）
    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// 打开启动项子键；`create` 为真时不存在就创建。调用方负责 `RegCloseKey`
    fn open_key(subkey: &str, access: REG_SAM_FLAGS, create: bool) -> Result<HKEY, String> {
        let subkey = wide(subkey);
        let mut key: HKEY = std::ptr::null_mut();
        let status = if create {
            let mut disposition: u32 = 0;
            unsafe {
                RegCreateKeyExW(
                    HKEY_CURRENT_USER,
                    subkey.as_ptr(),
                    0,
                    std::ptr::null(),
                    0,
                    access,
                    std::ptr::null(),
                    &mut key,
                    &mut disposition,
                )
            }
        } else {
            unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, access, &mut key) }
        };
        if status != ERROR_SUCCESS {
            return Err(format!("无法打开注册表启动项（错误码 {status}）"));
        }
        Ok(key)
    }

    fn close(key: HKEY) {
        unsafe { RegCloseKey(key) };
    }

    fn registry_error(action: &str, status: WIN32_ERROR) -> String {
        format!("{action}：系统返回错误码 {status}")
    }

    /// 读取启动项里登记的命令行；UTF-16 读写，路径里有中文也不会被代码页拆坏
    pub(super) fn read_run_value(subkey: &str) -> Option<String> {
        let key = open_key(subkey, KEY_QUERY_VALUE, false).ok()?;
        let name = wide(VALUE_NAME);
        let mut kind: u32 = 0;
        let mut bytes: u32 = 0;
        let status = unsafe {
            RegQueryValueExW(
                key,
                name.as_ptr(),
                std::ptr::null(),
                &mut kind,
                std::ptr::null_mut(),
                &mut bytes,
            )
        };
        if status != ERROR_SUCCESS || kind != REG_SZ || bytes < 2 {
            close(key);
            return None;
        }
        let mut buffer = vec![0u8; bytes as usize];
        let mut size = bytes;
        let status = unsafe {
            RegQueryValueExW(
                key,
                name.as_ptr(),
                std::ptr::null(),
                &mut kind,
                buffer.as_mut_ptr(),
                &mut size,
            )
        };
        close(key);
        if status != ERROR_SUCCESS {
            return None;
        }
        buffer.truncate(size as usize);
        let units: Vec<u16> = buffer
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        let text = String::from_utf16_lossy(&units);
        let text = text.trim_end_matches('\0').trim();
        (!text.is_empty()).then(|| text.to_string())
    }

    pub(super) fn write_run_value(subkey: &str, data: &str) -> Result<(), String> {
        let key = open_key(subkey, KEY_SET_VALUE, true)?;
        let name = wide(VALUE_NAME);
        let mut units: Vec<u16> = data.encode_utf16().collect();
        units.push(0); // REG_SZ 的结尾 NUL 也要写进去
        let bytes: Vec<u8> = units.iter().flat_map(|unit| unit.to_le_bytes()).collect();
        let status = unsafe {
            RegSetValueExW(
                key,
                name.as_ptr(),
                0,
                REG_SZ,
                bytes.as_ptr(),
                bytes.len() as u32,
            )
        };
        close(key);
        if status == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(registry_error("写入开机自启失败", status))
        }
    }

    pub(super) fn delete_run_value(subkey: &str) -> Result<(), String> {
        let key = open_key(subkey, KEY_SET_VALUE, false)?;
        let name = wide(VALUE_NAME);
        let status = unsafe { RegDeleteValueW(key, name.as_ptr()) };
        close(key);
        match status {
            ERROR_SUCCESS | ERROR_FILE_NOT_FOUND => Ok(()),
            other => Err(registry_error("关闭开机自启失败", other)),
        }
    }

    pub fn is_enabled() -> bool {
        let Some(exe) = launch_target() else {
            return false;
        };
        read_run_value(RUN_KEY).is_some_and(|recorded| same_windows_path(&recorded, &exe))
    }

    pub fn set_enabled(enabled: bool) -> Result<(), String> {
        let exe = launch_target().ok_or_else(|| "无法确定应用的安装路径".to_string())?;
        if enabled {
            write_run_value(RUN_KEY, &windows_run_command(&exe))?;
            // 读回校验：路径里的引号或空格若被拆坏，这里能立刻发现并撤回
            if !is_enabled() {
                let _ = delete_run_value(RUN_KEY);
                return Err("启动项写入后校验失败，已自动撤回".to_string());
            }
            Ok(())
        } else {
            delete_run_value(RUN_KEY)
        }
    }
}

#[cfg(any(target_os = "macos", test))]
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
mod macos_imp {
    use std::fs;
    use std::path::{Path, PathBuf};

    use crate::APP_ID;

    use super::{launch_agent_plist, xml_escape};

    pub(super) fn launch_target() -> Option<PathBuf> {
        std::env::current_exe().ok()
    }

    /// `~/Library/LaunchAgents/<APP_ID>.plist`
    pub(super) fn agent_path() -> Option<PathBuf> {
        let home = std::env::var_os("HOME")?;
        Some(
            PathBuf::from(home)
                .join("Library/LaunchAgents")
                .join(format!("{APP_ID}.plist")),
        )
    }

    /// 已经登记、且登记的就是这份可执行文件
    pub(super) fn is_enabled_at(path: &Path, exe: &Path) -> bool {
        let program = format!("<string>{}</string>", xml_escape(&exe.display().to_string()));
        fs::read_to_string(path).is_ok_and(|text| text.contains(&program))
    }

    pub(super) fn set_enabled_at(path: &Path, exe: &Path, enabled: bool) -> Result<(), String> {
        if enabled {
            if let Some(dir) = path.parent() {
                fs::create_dir_all(dir).map_err(|e| format!("创建 LaunchAgents 目录失败：{e}"))?;
            }
            fs::write(path, launch_agent_plist(exe))
                .map_err(|e| format!("写入登录项失败：{e}"))?;
            if !is_enabled_at(path, exe) {
                let _ = fs::remove_file(path);
                return Err("登录项写入后校验失败，已自动撤回".to_string());
            }
            Ok(())
        } else if path.exists() {
            fs::remove_file(path).map_err(|e| format!("移除登录项失败：{e}"))
        } else {
            Ok(())
        }
    }

    pub fn is_enabled() -> bool {
        let (Some(path), Some(exe)) = (agent_path(), launch_target()) else {
            return false;
        };
        is_enabled_at(&path, &exe)
    }

    pub fn set_enabled(enabled: bool) -> Result<(), String> {
        let path = agent_path().ok_or_else(|| "无法定位 ~/Library/LaunchAgents".to_string())?;
        let exe = launch_target().ok_or_else(|| "无法确定应用的安装路径".to_string())?;
        set_enabled_at(&path, &exe, enabled)
    }
}

#[cfg(any(target_os = "linux", test))]
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
mod linux_imp {
    use std::fs;
    use std::path::{Path, PathBuf};

    use crate::APP_ID;

    use super::{autostart_entry, desktop_exec};

    /// AppImage 启动器会把真实镜像路径放在 `$APPIMAGE` 里，
    /// 此时 `current_exe` 指向临时挂载点，不能写进启动项
    pub(super) fn launch_target() -> Option<PathBuf> {
        if let Some(appimage) = std::env::var_os("APPIMAGE") {
            return Some(PathBuf::from(appimage));
        }
        std::env::current_exe().ok()
    }

    /// `$XDG_CONFIG_HOME/autostart`（未设置时退回 `~/.config/autostart`）
    pub(super) fn autostart_dir() -> Option<PathBuf> {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
            .map(|base| base.join("autostart"))
    }

    pub(super) fn entry_path() -> Option<PathBuf> {
        Some(autostart_dir()?.join(format!("{APP_ID}.desktop")))
    }

    pub(super) fn is_enabled_at(path: &Path, exe: &Path) -> bool {
        fs::read_to_string(path).is_ok_and(|text| text.contains(&desktop_exec(exe)))
    }

    pub(super) fn set_enabled_at(path: &Path, exe: &Path, enabled: bool) -> Result<(), String> {
        if enabled {
            if let Some(dir) = path.parent() {
                fs::create_dir_all(dir).map_err(|e| format!("创建 autostart 目录失败：{e}"))?;
            }
            fs::write(path, autostart_entry(exe))
                .map_err(|e| format!("写入自启动入口失败：{e}"))?;
            if !is_enabled_at(path, exe) {
                let _ = fs::remove_file(path);
                return Err("自启动入口写入后校验失败，已自动撤回".to_string());
            }
            Ok(())
        } else if path.exists() {
            fs::remove_file(path).map_err(|e| format!("移除自启动入口失败：{e}"))
        } else {
            Ok(())
        }
    }

    pub fn is_enabled() -> bool {
        let (Some(path), Some(exe)) = (entry_path(), launch_target()) else {
            return false;
        };
        is_enabled_at(&path, &exe)
    }

    pub fn set_enabled(enabled: bool) -> Result<(), String> {
        let path = entry_path().ok_or_else(|| "无法定位 XDG autostart 目录".to_string())?;
        let exe = launch_target().ok_or_else(|| "无法确定应用的安装路径".to_string())?;
        set_enabled_at(&path, &exe, enabled)
    }
}

#[cfg(not(any(
    target_os = "windows",
    target_os = "macos",
    target_os = "linux"
)))]
mod unsupported_imp {
    /// Web 与移动端没有登录启动项
    pub fn is_enabled() -> bool {
        false
    }

    pub fn set_enabled(_enabled: bool) -> Result<(), String> {
        Err(super::unsupported_message())
    }
}

#[cfg(target_os = "windows")]
use windows_imp as imp;
#[cfg(target_os = "macos")]
use macos_imp as imp;
#[cfg(target_os = "linux")]
use linux_imp as imp;
#[cfg(not(any(
    target_os = "windows",
    target_os = "macos",
    target_os = "linux"
)))]
use unsupported_imp as imp;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::APP_ID;

    #[test]
    fn unsupported_platforms_report_themselves() {
        assert_eq!(state(false, false, None), AutostartState::Unsupported);
        // 不支持时即使系统里恰好有登记，也仍然显示「不可用」
        assert_eq!(state(false, true, None), AutostartState::Unsupported);
        assert_eq!(AutostartState::Unsupported.label(), "不可用");
        assert_eq!(
            hint(AutostartState::Unsupported, None),
            unsupported_message()
        );
    }

    #[test]
    fn states_follow_enabled_and_failure() {
        assert_eq!(state(true, true, None), AutostartState::Enabled);
        assert_eq!(state(true, false, None), AutostartState::Disabled);
        assert_eq!(state(true, false, Some("拒绝访问")), AutostartState::Failed);

        assert_eq!(AutostartState::Enabled.label(), "已开启");
        assert_eq!(AutostartState::Disabled.label(), "已关闭");
        assert_eq!(AutostartState::Failed.label(), "设置失败");
        assert!(!AutostartState::Disabled.is_error());
        assert!(AutostartState::Failed.is_error());

        assert_eq!(
            hint(AutostartState::Enabled, None),
            "登录系统后自动启动，无需手动打开"
        );
        assert_eq!(
            hint(AutostartState::Failed, Some("写入开机自启失败：拒绝访问")),
            "写入开机自启失败：拒绝访问"
        );
        // 失败但没有具体原因时也不能是空文字
        assert!(!hint(AutostartState::Failed, None).is_empty());
    }

    #[cfg(any(target_os = "windows", test))]
    #[test]
    fn windows_command_quotes_the_executable_path() {
        assert_eq!(
            windows_run_command(Path::new(r"C:\Program Files\reminder app\reminder.exe")),
            r#""C:\Program Files\reminder app\reminder.exe""#
        );
        assert_eq!(
            windows_run_command(Path::new(r"C:\no-space\reminder.exe")),
            r#""C:\no-space\reminder.exe""#
        );
    }

    #[cfg(any(target_os = "windows", test))]
    #[test]
    fn windows_paths_compare_ignoring_case_quotes_and_slashes() {
        let exe = Path::new(r"C:\Users\By\AppData\Local\reminder\reminder.exe");
        assert!(same_windows_path(
            r#""c:\users\by\appdata\local\reminder\reminder.exe""#,
            exe
        ));
        assert!(same_windows_path(
            "C:/Users/By/AppData/Local/reminder/reminder.exe",
            exe
        ));
        assert!(!same_windows_path(
            r#""C:\Program Files\old\reminder.exe""#,
            exe
        ));
        assert!(!same_windows_path("", exe));
        assert!(!same_windows_path("   ", exe));
    }

    #[cfg(any(target_os = "linux", test))]
    #[test]
    fn desktop_exec_escapes_specials() {
        assert_eq!(
            desktop_exec(Path::new("/opt/reminder/reminder")),
            r#""/opt/reminder/reminder""#
        );
        // 引号、反斜杠与百分号按 Desktop Entry 规范转义
        assert_eq!(
            desktop_exec(Path::new(r#"/home/a "q"\100%/reminder"#)),
            r#""/home/a \"q\"\\100%%/reminder""#
        );
    }

    #[cfg(any(target_os = "linux", test))]
    #[test]
    fn autostart_entry_keeps_one_exec_line() {
        let entry = autostart_entry(Path::new("/home/by/Applications/reminder"));
        assert!(entry.starts_with("[Desktop Entry]\n"));
        assert!(entry.contains("Type=Application\n"));
        assert!(entry.contains(r#"Exec="/home/by/Applications/reminder""#));
        assert!(entry.contains("X-GNOME-Autostart-enabled=true\n"));
        // Exec 必须独占一行，否则启动器会把后面的键当成参数
        assert_eq!(
            entry.lines().filter(|line| line.starts_with("Exec=")).count(),
            1
        );
    }

    #[cfg(any(target_os = "macos", test))]
    #[test]
    fn launch_agent_escapes_the_program_path() {
        let plist = launch_agent_plist(Path::new("/Applications/A & B/reminder"));
        assert!(plist.contains("<key>RunAtLoad</key>"));
        assert!(plist.contains("<string>Aqua</string>"));
        assert!(plist.contains("<string>/Applications/A &amp; B/reminder</string>"));
        assert!(plist.contains(&format!("<string>{APP_ID}</string>")));
    }

    #[cfg(any(target_os = "macos", test))]
    #[test]
    fn xml_escape_covers_markup_characters() {
        assert_eq!(
            xml_escape("a&b<c>d\"e'f"),
            "a&amp;b&lt;c&gt;d&quot;e&apos;f"
        );
    }

    /// 真实走一遍注册表读写，验证带空格的路径不会被拆坏。
    /// 使用独立的临时子键，不碰真正的 Run 键：
    /// `cargo test autostart_registry_roundtrip -- --ignored`
    #[cfg(target_os = "windows")]
    #[test]
    #[ignore = "会写入 HKCU\\Software\\reminder-autostart-test，需要手动运行"]
    fn autostart_registry_roundtrip() {
        use std::process::Command;

        let key = r"Software\reminder-autostart-test";
        let exe = Path::new(r"C:\Program Files\reminder app\reminder.exe");
        let data = windows_run_command(exe);

        windows_imp::write_run_value(key, &data).expect("write run value");
        let recorded = windows_imp::read_run_value(key).expect("read back");
        assert_eq!(recorded, data);
        assert!(same_windows_path(&recorded, exe));

        windows_imp::delete_run_value(key).expect("delete run value");
        assert!(windows_imp::read_run_value(key).is_none());
        // 重复删除也要成功（关闭已关闭的开机自启不应报错）
        windows_imp::delete_run_value(key).expect("delete twice");

        // 清掉空键本身，跑完不留痕
        let _ = Command::new("reg")
            .args(["delete", &format!(r"HKCU\{key}"), "/f"])
            .output();
    }

    /// 临时目录里的文件路径（每个用例一个目录，互不干扰）
    fn temp_path(tag: &str, name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "reminder_autostart_{tag}_{}",
            std::process::id()
        ));
        let _ = std::fs::create_dir_all(&dir);
        dir.join(name)
    }

    fn remove_temp_file(path: &Path) {
        if let Some(dir) = path.parent() {
            let _ = std::fs::remove_dir_all(dir);
        }
    }

    /// macOS 的 LaunchAgent 文件也在本机跑一遍（模块在 `cfg(test)` 下交叉编译）：
    /// 登记、换路径后的失效判断、移除、二次移除
    #[test]
    fn macos_agent_file_roundtrip() {
        let path = temp_path("macos", &format!("{APP_ID}.plist"));
        let exe = Path::new("/Applications/reminder.app/Contents/MacOS/reminder");

        assert!(!macos_imp::is_enabled_at(&path, exe));
        macos_imp::set_enabled_at(&path, exe, true).expect("enable");
        assert!(macos_imp::is_enabled_at(&path, exe));
        // 登记的是另一份可执行文件时不算已开启（应用移动过位置）
        assert!(!macos_imp::is_enabled_at(&path, Path::new("/old/reminder")));
        assert!(
            std::fs::read_to_string(&path)
                .expect("read plist")
                .contains("<key>RunAtLoad</key>")
        );

        macos_imp::set_enabled_at(&path, exe, false).expect("disable");
        assert!(!macos_imp::is_enabled_at(&path, exe));
        assert!(!path.exists());
        // 已关闭时再关一次也不报错
        macos_imp::set_enabled_at(&path, exe, false).expect("disable twice");

        remove_temp_file(&path);
    }

    /// Linux 的 XDG autostart 入口文件同样在本机跑一遍
    #[test]
    fn linux_autostart_file_roundtrip() {
        let path = temp_path("linux", &format!("{APP_ID}.desktop"));
        let exe = Path::new("/home/by/Applications/reminder");

        assert!(!linux_imp::is_enabled_at(&path, exe));
        linux_imp::set_enabled_at(&path, exe, true).expect("enable");
        assert!(linux_imp::is_enabled_at(&path, exe));
        assert!(!linux_imp::is_enabled_at(&path, Path::new("/old/reminder")));
        let entry = std::fs::read_to_string(&path).expect("read entry");
        assert!(entry.contains(r#"Exec="/home/by/Applications/reminder""#));

        linux_imp::set_enabled_at(&path, exe, false).expect("disable");
        assert!(!linux_imp::is_enabled_at(&path, exe));
        assert!(!path.exists());
        linux_imp::set_enabled_at(&path, exe, false).expect("disable twice");

        remove_temp_file(&path);
    }
}
