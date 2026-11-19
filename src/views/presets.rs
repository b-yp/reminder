//! 常用推荐预设的渲染：单张预设卡片 + 两列网格 + 分组小标题。
//!
//! 预设数据在 `crate::presets`；这里只负责外观与「点击加入列表」的行为。

use nsh_base::theme;
use nsh_core::prelude::*;

use crate::app::AppCtx;
use crate::localtime;
use crate::model::{ReminderItem, insert_item_at_top};
use crate::presets::{PresetKind, PresetReminder};
use crate::ui;

/// 渲染单个常用推荐预设卡片（精致卡片、无勾选、清晰时间标识、单例防重）
pub(crate) fn render_card(preset: &'static PresetReminder, app: AppCtx) -> impl IntoElement {
    let AppCtx { items, .. } = app;
    let on_save = move || app.save();
    let name = preset.name;
    let icon = preset.icon_key;
    let badge = match preset.kind {
        PresetKind::Interval(mins) => format!("{mins}m"),
        PresetKind::Daily(hour, minute) => format!("{hour:02}:{minute:02}"),
    };

    let is_running = computed(move || items.with(|list| list.iter().any(|x| x.name == name)));

    // 卡片本身是 ui::preset_card（底座 nsh/base button）：已在列表里时语义上禁用，
    // 不再重复添加，读屏会读出不可用。
    ui::preset_card(
        name,
        badge,
        icon,
        move || is_running.get(),
        move || {
            if !is_running.get() {
                let now = localtime::now();
                let item = match preset.kind {
                    PresetKind::Interval(mins) => ReminderItem::new(name, icon, mins * 60),
                    PresetKind::Daily(hour, minute) => {
                        ReminderItem::new_daily(name, icon, hour, minute, &now)
                    }
                };
                items.update(|list| {
                    insert_item_at_top(list, item);
                });
                on_save();
            }
        },
    )
}

/// 预设卡片的两列网格
pub(crate) fn render_grid(presets: &'static [PresetReminder], app: AppCtx) -> impl IntoElement {
    view()
        .w_full()
        .flex_col()
        .gap(px(4.5))
        .children(presets.chunks(2).map(|pair| {
            view()
                .w_full()
                .flex_row()
                .gap(px(4.5))
                .children(pair.iter().map(|preset| render_card(preset, app)))
        }))
}

/// 预设分组的小标题（「循环打卡」「定点时刻」）
pub(crate) fn section_label(title: &'static str, hint: &'static str) -> impl IntoElement {
    view()
        .w_full()
        .flex_row()
        .items_center()
        .justify_between()
        .child(
            text(title)
                .text_size(px(11.0))
                .text_color(move || theme().muted_foreground),
        )
        .child(
            text(hint)
                .text_size(px(10.5))
                .text_color(move || theme().muted_foreground),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presets::INTERVAL_PRESETS;
    use crate::storage::AppState;
    use crate::views::list;

    #[component]
    fn PresetAddBehaviorHost() -> impl IntoElement {
        let items = signal(vec![ReminderItem::new("旧任务", "droplet", 1800)]);
        let app = AppCtx::new(AppState {
            items: items.get(),
            ..Default::default()
        });
        view()
            .w(px(320.0))
            .h(px(500.0))
            .flex_col()
            .child(render_grid(INTERVAL_PRESETS, app))
            .child(list::render(app))
    }
    #[test]
    fn test_preset_add_inserts_at_top_headless() {
        use nsh_engine::{EngineConfig, Points, Root, Size};
        use nsh_testing::Headless;

        let config = EngineConfig {
            viewport: Size::new(Points(320.0), Points(500.0)),
            ..EngineConfig::new(Root::of::<PresetAddBehaviorHost>())
        };
        let mut app = Headless::start(config).expect("Headless started");
        app.wait_for("旧任务").unwrap();
        assert!(app.screen().shows("旧任务"));

        // 点击预设卡片「番茄专注」
        app.press("番茄专注").unwrap();
        app.wait_for("25m").unwrap();

        // 验证生产代码实际将新行渲染在「旧任务」之前
        let texts = app.screen().texts();
        let new_item_idx = texts
            .iter()
            .rposition(|&t| t == "番茄专注")
            .expect("番茄专注已渲染到提醒列表中");
        let old_item_idx = texts
            .iter()
            .position(|&t| t == "旧任务")
            .expect("旧任务存在");
        assert!(
            new_item_idx < old_item_idx,
            "新增提醒 (番茄专注 index={new_item_idx}) 必须渲染在旧任务 (index={old_item_idx}) 之前"
        );
    }
}
