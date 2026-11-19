//! 常用推荐预设：循环打卡与每天定点两套模板。
//!
//! 纯数据、不含渲染：卡片外观在 `views/presets.rs`，插入行为在 `views/presets.rs` 的按钮回调里。

/// 预设提醒模板（通用丰富场景：间隔循环 + 每天定点）
pub(crate) struct PresetReminder {
    pub(crate) name: &'static str,
    pub(crate) icon_key: &'static str,
    pub(crate) kind: PresetKind,
}

/// 预设的计时方式
pub(crate) enum PresetKind {
    /// 每隔多少分钟
    Interval(u32),
    /// 每天 时:分
    Daily(u32, u32),
}

pub(crate) const INTERVAL_PRESETS: &[PresetReminder] = &[
    PresetReminder {
        name: "喝水",
        icon_key: "droplet",
        kind: PresetKind::Interval(30),
    },
    PresetReminder {
        name: "起身活动",
        icon_key: "walk",
        kind: PresetKind::Interval(45),
    },
    PresetReminder {
        name: "护眼远眺",
        icon_key: "eye",
        kind: PresetKind::Interval(20),
    },
    PresetReminder {
        name: "番茄专注",
        icon_key: "clock",
        kind: PresetKind::Interval(25),
    },
    PresetReminder {
        name: "带薪拉屎",
        icon_key: "toilet",
        kind: PresetKind::Interval(90),
    },
    PresetReminder {
        name: "站立拉伸",
        icon_key: "activity",
        kind: PresetKind::Interval(60),
    },
];

/// 定点时刻类预设：午餐、下班打卡、睡觉……这些按墙上时间提醒才有意义
pub(crate) const DAILY_PRESETS: &[PresetReminder] = &[
    PresetReminder {
        name: "早餐",
        icon_key: "coffee",
        kind: PresetKind::Daily(8, 0),
    },
    PresetReminder {
        name: "午餐",
        icon_key: "apple",
        kind: PresetKind::Daily(12, 0),
    },
    PresetReminder {
        name: "下班打卡",
        icon_key: "laptop",
        kind: PresetKind::Daily(18, 30),
    },
    PresetReminder {
        name: "睡觉",
        icon_key: "moon",
        kind: PresetKind::Daily(23, 0),
    },
];
