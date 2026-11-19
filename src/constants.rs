//! 应用级常量与静态文案：窗口尺寸、新建表单默认值、快捷填入选项、图标候选与时长常量。
//!
//! 这些值同时被视图层、窗口层和测试读取，集中在一处，避免「改一处漏一处」。

use nsh_core::prelude::*;

/// App Logo：顶栏微标与托盘图标共用同一份资源。
pub(crate) const LOGO: ImageAsset = asset!("logo.png");

/// 统一窗口尺寸规范（逻辑像素，自动由 NSH 处理 High-DPI 缩放，杜绝模糊与尺寸割裂）
pub const WINDOW_WIDTH: f32 = 320.0;
pub const WINDOW_HEIGHT: f32 = 350.0;
/// 迷你胶囊形态尺寸（极简悬浮小药丸，仅占用极少屏幕空间）
pub const MINI_WIDTH: f32 = 196.0;
pub const MINI_HEIGHT: f32 = 44.0;

/// 自定义新提醒表单的初始值：新建抽屉每次打开都回到这里，与首次打开完全一致
pub(crate) const DRAFT_MINS: &str = "30";
pub(crate) const DRAFT_HOUR: &str = "12";
pub(crate) const DRAFT_MINUTE: &str = "00";
pub(crate) const DRAFT_ICON: &str = "bell";

/// 循环周期的快捷填入（标签, 分钟）
pub(crate) const MINUTE_CHIPS: &[(&str, u32)] = &[
    ("15m", 15),
    ("25m", 25),
    ("30m", 30),
    ("45m", 45),
    ("60m", 60),
    ("90m", 90),
];

/// 常用定点时刻（标签, 时, 分）
pub(crate) const TIME_CHIPS: &[(&str, u32, u32)] = &[
    ("08:00", 8, 0),
    ("12:00", 12, 0),
    ("18:30", 18, 30),
    ("23:00", 23, 0),
];

/// 图标主题第一排：生活与健康（图标 key, 读屏用的名字）
pub(crate) const ICON_CHOICES_LIFE: &[(&str, &str)] = &[
    ("droplet", "喝水"),
    ("coffee", "咖啡"),
    ("walk", "走动"),
    ("activity", "活动"),
    ("eye", "护眼"),
    ("pill", "服药"),
    ("heart", "静心"),
    ("sun", "晒太阳"),
];

/// 删除后留给「撤销」的秒数
pub(crate) const UNDO_SECONDS: u64 = 5;

/// 图标主题第二排：专注与日程
pub(crate) const ICON_CHOICES_FOCUS: &[(&str, &str)] = &[
    ("clock", "专注"),
    ("book", "阅读"),
    ("laptop", "办公"),
    ("target", "目标"),
    ("moon", "小憩"),
    ("apple", "轻食"),
    ("toilet", "如厕"),
    ("bell", "闹钟"),
];

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
}
