//! 应用内 shared styled 控件层：把 NSH 的行为组件（`nsh/base` 的 `button`、
//! `toggle`、`switch`）包成挂件统一的外观。业务区块只组合这里的控件，不直接
//! 写颜色、圆角、字号与按压反馈。
//!
//! 分工：`nsh/base` 给行为（角色、键盘、焦点、选中态）；这一层给外观；
//! 页面只给布局（宽高、间距、对齐）。

mod badge;
mod button;
mod choice;
mod disclosure;
mod menu;
mod preset;
mod switch;
pub mod tokens;

pub use badge::{BadgeTone, status_badge};
pub use button::{
    ActionTone, action_button, dynamic_tooltip_bubble, icon_button, micro_button, step_button,
    text_button, tooltip_bubble, undo_button,
};
pub use choice::{choice_chip, choice_grid, choice_group, icon_choice, segment_toggle};
pub use disclosure::disclosure_row;
pub use menu::row_menu;
pub use preset::preset_card;
pub use switch::setting_switch;
