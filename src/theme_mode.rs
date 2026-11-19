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
/// 用于主行动实心底、开关打开轨道、滑块轨迹、输入框边框及选中描边。
/// 浅色为 Amber-700 (#b45309)，使白字在浅色模式下的实测对比度达到 5.00:1（符合 WCAG AA 4.5:1 要求）。
pub const ACCENT_GOLD: AdaptiveColor = AdaptiveColor::rgb(0xb45309, 0xf59e0b);
/// 品牌金黄背景浅底色（自适应浅色/深色，用于徽章、胶囊浅底）
pub const ACCENT_GOLD_BG: AdaptiveColor = AdaptiveColor::rgb(0xfef3c7, 0x451a03);
/// 金黄实心底上的文字色（浅色白字 #ffffff 实测 5.00:1，深色石板黑字 #1c1917 实测 8.14:1）
pub const ACCENT_GOLD_FG: AdaptiveColor = AdaptiveColor::rgb(0xffffff, 0x1c1917);
/// 品牌高对比度金色前景（自适应浅色/深色，用于浅金底或中性底上的文字与关键图标）
/// 浅色为深金褐 #92400e（浅金底实测 6.40:1，白底实测 6.82:1），深色为明亮金 #fcd34d（实测 10.39:1），
/// 用于徽章文字、胶囊、打卡对勾、抽屉主板块标题图标、底栏指示及倒计时文字。
pub const ACCENT_GOLD_STRONG: AdaptiveColor = AdaptiveColor::rgb(0x92400e, 0xfcd34d);
/// 「已达成 / 已加入」的柔和绿底
pub const ACCENT_DONE_BG: AdaptiveColor = AdaptiveColor::rgb(0xdcfce7, 0x064e3b);
/// 「已达成 / 已加入」的绿描边
pub const ACCENT_DONE_BORDER: AdaptiveColor = AdaptiveColor::rgb(0x86efac, 0x059669);
/// 「已达成 / 已加入」的文字色
pub const ACCENT_DONE_FG: AdaptiveColor = AdaptiveColor::rgb(0x166534, 0x6ee7b7);
/// 逾期行的警示底色：浅色淡红 #fef2f2、深色暗红 #2a1315，不压过行内文字
pub const OVERDUE_WASH: AdaptiveColor = AdaptiveColor::rgb(0xfef2f2, 0x2a1315);
/// 逾期警示文字与图标色：浅色深红 #b91c1c（在淡红底上实测 5.91:1），深色高亮红 #f87171（在暗红底上实测 6.31:1）
pub const OVERDUE_FG: AdaptiveColor = AdaptiveColor::rgb(0xb91c1c, 0xf87171);
/// 通用危险/错误文字与图标色（用于热键冲突、自启失败、表单校验错误、Danger 徽章）
/// 语义复用逾期警示色，在白底上实测 6.47:1、在深色暗底上实测 5.47~7.16:1，单点定义始终一致
pub const ACCENT_DANGER_FG: AdaptiveColor = OVERDUE_FG;
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

    /// 根据 Hsla 色值计算 WCAG 2.1 相对亮度 (Relative Luminance)
    fn hsla_luminance(hsla: nsh_core::color::Hsla) -> f64 {
        let rgba = hsla.to_rgb();
        let channel = |c: f32| {
            let c = c as f64;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };

        0.2126 * channel(rgba.r) + 0.7152 * channel(rgba.g) + 0.0722 * channel(rgba.b)
    }

    /// 根据两个实际 Hsla 颜色计算对比度比率：(L1 + 0.05) / (L2 + 0.05)
    fn token_contrast(c1: nsh_core::color::Hsla, c2: nsh_core::color::Hsla) -> f64 {
        let l1 = hsla_luminance(c1);
        let l2 = hsla_luminance(c2);
        let (lighter, darker) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
        (lighter + 0.05) / (darker + 0.05)
    }

    #[test]
    fn test_wcag_contrast_ratios() {
        // --- 浅色模式检验 (全面校验实际 Token，不硬编码 16 进制颜色) ---

        // 1. 浅色主按钮：ACCENT_GOLD_FG.light on ACCENT_GOLD.light (白字 on 琥珀金)
        let light_btn = token_contrast(ACCENT_GOLD_FG.light, ACCENT_GOLD.light);
        assert!(
            light_btn >= 4.5,
            "浅色主按钮文字对比度需 >= 4.5:1，实测 {:.2}:1",
            light_btn
        );

        // 2. 浅色浅金底：ACCENT_GOLD_STRONG.light on ACCENT_GOLD_BG.light (深金褐 on 浅金底)
        let light_chip = token_contrast(ACCENT_GOLD_STRONG.light, ACCENT_GOLD_BG.light);
        assert!(
            light_chip >= 4.5,
            "浅金底文字对比度需 >= 4.5:1，实测 {:.2}:1",
            light_chip
        );

        // 3. 浅色逾期行：OVERDUE_FG.light on OVERDUE_WASH.light (深红 on 淡红底)
        let light_overdue = token_contrast(OVERDUE_FG.light, OVERDUE_WASH.light);
        assert!(
            light_overdue >= 4.5,
            "浅色逾期警示在淡红底上的对比度需 >= 4.5:1，实测 {:.2}:1",
            light_overdue
        );

        // 4. 浅色通用错误：ACCENT_DANGER_FG.light on 常规白色背景
        let white_hsla = AdaptiveColor::rgb(0xffffff, 0xffffff).light;
        let light_danger = token_contrast(ACCENT_DANGER_FG.light, white_hsla);
        assert!(
            light_danger >= 4.5,
            "浅色通用错误在白底上的对比度需 >= 4.5:1，实测 {:.2}:1",
            light_danger
        );

        // --- 深色模式全覆盖检验 ---

        // 5. 深色主按钮：ACCENT_GOLD_FG.dark on ACCENT_GOLD.dark (深灰黑 on 亮金)
        let dark_btn = token_contrast(ACCENT_GOLD_FG.dark, ACCENT_GOLD.dark);
        assert!(
            dark_btn >= 7.0,
            "深色主按钮文字对比度需 >= 7.0:1，实测 {:.2}:1",
            dark_btn
        );

        // 6. 深色浅金底：ACCENT_GOLD_STRONG.dark on ACCENT_GOLD_BG.dark (亮金 on 暗褐底)
        let dark_chip = token_contrast(ACCENT_GOLD_STRONG.dark, ACCENT_GOLD_BG.dark);
        assert!(
            dark_chip >= 7.0,
            "深色浅金底文字对比度需 >= 7.0:1，实测 {:.2}:1",
            dark_chip
        );

        // 7. 深色逾期行：OVERDUE_FG.dark on OVERDUE_WASH.dark (高亮红 on 暗红底)
        let dark_overdue = token_contrast(OVERDUE_FG.dark, OVERDUE_WASH.dark);
        assert!(
            dark_overdue >= 4.5,
            "深色逾期警示在暗红底上的对比度需达到 WCAG AA (>= 4.5:1)，实测 {:.2}:1",
            dark_overdue
        );

        // 8. 深色通用错误文案：ACCENT_DANGER_FG.dark on 深色界面背景 (如抽屉/卡片底色 #18181b)
        let dark_surface = AdaptiveColor::rgb(0x18181b, 0x18181b).dark;
        let dark_danger = token_contrast(ACCENT_DANGER_FG.dark, dark_surface);
        assert!(
            dark_danger >= 4.5,
            "深色通用错误文案在界面底色上的对比度需达到 WCAG AA (>= 4.5:1)，实测 {:.2}:1",
            dark_danger
        );
    }
}
