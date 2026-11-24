//! 共享 UI 设计令牌 (Design Tokens)
//!
//! 这里收的是「尺度」本身：字阶、圆角、间距、控件尺寸与基础动效。
//! 组件只从这里取值，页面只给布局；任何一次性的字面量都应该先回到这里补一档。

/// 字阶：四档，覆盖挂件里所有文字。
///
/// 10.5 只用于角标、键帽、计数这类「扫一眼」的辅助信息；11 是标签与说明；
/// 12 是正文与数值；13 是标题。桌面挂件是紧凑密度，所以整体比常规桌面应用小一档。
pub mod font {
    /// 角标、键帽、计数、秒数 (10.5pt)
    pub const CAPTION: f32 = 10.5;
    /// 字段标签、说明文字、错误提示、胶囊控件 (11.0pt)
    pub const LABEL: f32 = 11.0;
    /// 正文与数值：菜单项、按钮、气泡、倒计时、今日次数 (12.0pt)
    pub const BODY: f32 = 12.0;
    /// 标题：应用名、提醒名、设置分区标题、空状态标题 (13.0pt)
    pub const TITLE: f32 = 13.0;
}

/// 控件几何圆角尺度：小部件最紧，容器最松，嵌套时用「内圆角 ≈ 外圆角 − 间隙」。
pub mod radius {
    /// 微控件与内嵌胶囊：数字步进、行内周期胶囊、快捷填入项 (3.0pt)
    pub const SM: f32 = 3.0;
    /// 紧凑控件与菜单项：常规文字按钮、撤销按钮、状态徽章、菜单项 (4.0pt)
    pub const MD: f32 = 4.0;
    /// 标准控件与面板浮层：动作按钮、图标按钮、分段选择、输入框 (6.0pt)
    pub const LG: f32 = 6.0;
    /// 抽屉与 Toast 这类贴在卡片里的大浮层 (8.0pt)
    pub const XL: f32 = 8.0;
    /// 窗口卡片本身 (12.0pt)
    pub const MAX: f32 = 12.0;
}

/// 间距尺度：按「两个东西的关系有多近」取档。
pub mod space {
    /// 2pt：图标与文字、行内元素的视觉修正
    pub const HAIR: f32 = 2.0;
    /// 4pt：同一个控件内部的部件
    pub const TIGHT: f32 = 4.0;
    /// 6pt：相邻控件、列表行之间
    pub const SNUG: f32 = 6.0;
    /// 8pt：一组内容、抽屉内缩
    pub const GAP: f32 = 8.0;
    /// 12pt：区块之间
    pub const GROUP: f32 = 12.0;
    /// 14pt：窗口卡片的内容内缩
    pub const INSET: f32 = 14.0;
    /// 16pt：大区块与空状态的呼吸
    pub const SECTION: f32 = 16.0;
}

/// 统一控件尺寸基线
pub mod size {
    /// 紧凑控件边长/高度 (20.0pt)
    pub const CONTROL_SM: f32 = 20.0;
    /// 标准交互控件高度：图标按钮、操作按钮、分段切换、输入框 (24.0pt)
    pub const CONTROL_MD: f32 = 24.0;
    /// 键盘焦点环粗细 (2.0pt)
    pub const FOCUS_RING: f32 = 2.0;
}

/// 动效过渡基准
pub mod motion {
    /// 悬浮、高亮与状态颜色微动效时长 (120ms)
    pub const FAST: f32 = 120.0;
    /// 移到新位置、退场：列表行、撤销条 (240ms)
    pub const BASE: f32 = 240.0;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_token_scales() {
        assert!(font::CAPTION < font::LABEL);
        assert!(font::LABEL < font::BODY);
        assert!(font::BODY < font::TITLE);
        assert!(radius::SM < radius::MD && radius::MD < radius::LG);
        assert!(radius::LG < radius::XL && radius::XL < radius::MAX);
        assert!(space::HAIR < space::TIGHT && space::TIGHT < space::SNUG);
        assert!(space::SNUG < space::GAP && space::GAP < space::GROUP);
        assert!(space::GROUP < space::INSET && space::INSET < space::SECTION);
        assert!(size::CONTROL_SM < size::CONTROL_MD);
    }
}
