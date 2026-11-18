//! 主题模块：定义主题模式与品牌高光自适应令牌
//! 基础界面配色完全遵循 NSH 官方标准的 `nsh_base::theme()`（基于 shadcn/ui 设计令牌）

use nsh_core::prelude::*;
use serde::{Deserialize, Serialize};

/// 主题模式枚举
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
    System,
}

impl ThemeMode {
    /// 显示文本
    pub fn label(self) -> &'static str {
        match self {
            ThemeMode::Dark => "深色模式",
            ThemeMode::Light => "浅色模式",
            ThemeMode::System => "跟随系统",
        }
    }

    /// 对应的 NSH Appearance
    pub fn to_appearance(self) -> nsh_core::platform::Appearance {
        match self {
            ThemeMode::Dark => nsh_core::platform::Appearance::Dark,
            ThemeMode::Light => nsh_core::platform::Appearance::Light,
            ThemeMode::System => nsh_core::platform::Appearance::System,
        }
    }
}

/// 品牌金黄强调色（自适应浅色/深色）
pub const ACCENT_GOLD: AdaptiveColor = AdaptiveColor::rgb(0xd97706, 0xf59e0b);
/// 品牌金黄背景微标色（自适应浅色/深色）
pub const ACCENT_GOLD_BG: AdaptiveColor = AdaptiveColor::rgb(0xfef3c7, 0x451a03);
/// 金黄实心底上的文字色
pub const ACCENT_GOLD_FG: AdaptiveColor = AdaptiveColor::rgb(0xffffff, 0x000000);
/// 「已达成 / 已加入」的柔和绿底
pub const ACCENT_DONE_BG: AdaptiveColor = AdaptiveColor::rgb(0xdcfce7, 0x064e3b);
/// 「已达成 / 已加入」的绿描边
pub const ACCENT_DONE_BORDER: AdaptiveColor = AdaptiveColor::rgb(0x86efac, 0x059669);
/// 「已达成 / 已加入」的文字色
pub const ACCENT_DONE_FG: AdaptiveColor = AdaptiveColor::rgb(0x166534, 0x6ee7b7);
/// 「现在就该处理」的实心绿（逾期打卡）
pub const ACCENT_NOW_BG: AdaptiveColor = AdaptiveColor::rgb(0x16a34a, 0x15803d);
/// 「现在就该处理」的绿描边
pub const ACCENT_NOW_BORDER: AdaptiveColor = AdaptiveColor::rgb(0x22c55e, 0x4ade80);
/// 实心绿上的文字与图标色
pub const ACCENT_NOW_FG: AdaptiveColor = AdaptiveColor::rgb(0xffffff, 0xffffff);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_mode_enum() {
        assert_eq!(ThemeMode::default(), ThemeMode::Dark);
        assert_eq!(ThemeMode::Dark.label(), "深色模式");
        assert_eq!(ThemeMode::Light.label(), "浅色模式");
        assert_eq!(ThemeMode::System.label(), "跟随系统");

        assert_eq!(
            ThemeMode::Dark.to_appearance(),
            nsh_core::platform::Appearance::Dark
        );
        assert_eq!(
            ThemeMode::Light.to_appearance(),
            nsh_core::platform::Appearance::Light
        );
        assert_eq!(
            ThemeMode::System.to_appearance(),
            nsh_core::platform::Appearance::System
        );
    }
}
