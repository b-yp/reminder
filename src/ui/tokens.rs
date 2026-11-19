//! 共享 UI 设计令牌 (Design Tokens)
//!
//! 只收敛跨 styled 控件层真正高频共享的核心基准尺度（字阶、控件圆角、控件尺寸与基础动效），
//! 杜绝把组件内部独立的局部边距与尺寸机械重命名的过度设计。

/// 基础排版字阶（桌面挂件紧凑密度）
pub mod font {
    /// 辅助字号：徽章、状态角标、小胶囊、说明文本 (10.5pt)
    pub const CAPTION: f32 = 10.5;
    /// 标准正文：按钮文字、菜单项、常规标签 (11.0pt)
    pub const BODY: f32 = 11.0;
}

/// 控件几何圆角尺度
pub mod radius {
    /// 微控件与内嵌胶囊：数字步进、行内周期胶囊、快捷填入项 (3.0pt)
    pub const SM: f32 = 3.0;
    /// 紧凑控件与菜单项：常规文字按钮、撤销按钮、状态徽章、菜单项 (4.0pt)
    pub const MD: f32 = 4.0;
    /// 标准控件与面板浮层：新交互控件默认使用该档；包含动作按钮、图标按钮、分段选择、卡片与浮层 (6.0pt)
    pub const LG: f32 = 6.0;
}

/// 统一控件尺寸基线
pub mod size {
    /// 紧凑控件边长/高度 (20.0pt)
    pub const CONTROL_SM: f32 = 20.0;
    /// 标准交互控件高度：图标按钮、操作按钮、分段切换 (24.0pt)
    pub const CONTROL_MD: f32 = 24.0;
    /// 键盘焦点环粗细 (2.0pt)
    pub const FOCUS_RING: f32 = 2.0;
}

/// 动效过渡基准
pub mod motion {
    /// 悬浮、高亮与状态颜色微动效时长 (120ms)
    pub const FAST: f32 = 120.0;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_token_scales() {
        assert!(font::CAPTION < font::BODY);
        assert!(radius::SM < radius::MD && radius::MD < radius::LG);
        assert!(size::CONTROL_SM < size::CONTROL_MD);
    }
}
