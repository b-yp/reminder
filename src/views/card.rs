//! 完整卡片形态（320×350）：顶栏 + 可滚动内容区（新建抽屉 / 设置抽屉 / 提醒列表）
//! + 底栏状态行 + 底部撤销 Toast。

use nsh_core::prelude::*;

use crate::app::AppCtx;
use crate::ui::tokens::space;
use crate::views::{add_drawer, footer, header, list, settings, toast};

pub fn render(app: AppCtx) -> impl IntoElement {
    let AppCtx {
        pending_delete,
        toast_leaving,
        ..
    } = app;

    view()
        .relative()
        .w_full()
        .h_full()
        .p(px(space::INSET))
        .flex_col()
        .justify_between()
        // 1. 顶栏：通用「reminder 提醒」金黄微标与标题，右侧挂件操作按钮组（新建、暂停、设置、收起胶囊、收进托盘）
        .child(header::render(app))
        // 2. 核心内容滚动区：抽屉展开与列表统一容纳，高了自行平滑滚动，大窗口全程稳定
        .child(
            scroll()
                .window_drag(true)
                .w_full()
                .flex_1()
                .my(px(space::TIGHT))
                .scrollbar(false)
                .child(
                    view()
                        .w_full()
                        .flex_col()
                        .gap(px(space::SNUG))
                        // 独立新建提醒抽屉（集成常用推荐预设与自定义新提醒配置）
                        .child(add_drawer::render(app))
                        // 独立偏好设置抽屉（外观主题选择、透明度调节、全局热键、开机自启）
                        .child(settings::render(app))
                        // 核心提醒列表
                        .child(list::render(app)),
                ),
        )
        // 3. 底部状态行（「进行中 X 项 · 今日打卡 Y 次」，若处于半透明状态则额外指示不透明度；
        //    常态下右侧显示全局热键，提示一键隐藏 / 呼出）
        .child(footer::render(app))
        // 4. 底部浮层撤销 Toast（当存在 pending_delete 时在底栏上方优雅浮现，
        //    退场时先播 240ms 的 ending 再卸载）
        .child(when(
            move || pending_delete.get().is_some(),
            move || {
                pending_delete
                    .get()
                    .map(|pending| toast::render(app, pending, toast_leaving.get()))
            },
        ))
}
