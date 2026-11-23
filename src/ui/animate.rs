//! 列表级动效：增删一行时，其余行滑到新位置（而不是瞬间跳一格）。
//!
//! 只在用户自己的动作里调用（新建、打卡删除后撤销、删除），不在每次数据刷新时调用；
//! 减弱动态效果时什么都不做。

use nsh_core::prelude::*;
use nsh_core::{LayoutAnimation, LayoutCurve, LayoutTransition};

use super::tokens::motion;

/// 让下一次提交里行的增删、位置的移动走 240ms 的 ease-in-out。
pub fn animate_rows() {
    if reduce_motion() {
        return;
    }
    animate_layout(LayoutAnimation::all(LayoutTransition::new(
        LayoutCurve::EaseInEaseOut,
        motion::BASE,
    )));
}
