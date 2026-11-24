//! 偏好设置里的「应用更新」区：当前版本、检查新版本、下载，退出应用时替换。
//!
//! 一条主线只给一个动作：未检查 / 已是最新 / 失败 → 「检查更新」，有新版本 →
//! 「下载更新」，下载并校验完 → 「退出并更新」。同一时刻只会有一个动作能推进，
//! 下载中与检查中不显示按钮，只显示进度与说明。

use nsh_base::theme;
use nsh_core::element::AdaptiveColor;
use nsh_core::prelude::*;

use crate::app::AppCtx;
use crate::icons;
use crate::theme_mode::{ACCENT_DANGER_FG, ACCENT_GOLD_STRONG};
use crate::ui;
use crate::ui::tokens::font;
use crate::updater::{self, Status};

/// 状态徽章的语气：能动手时用品牌金黄，出错用警示色，其余保持中性
fn tone(status: &Status) -> ui::BadgeTone {
    match status {
        Status::Available(_) | Status::Ready { .. } | Status::Installing => ui::BadgeTone::Accent,
        Status::Failed(_) => ui::BadgeTone::Danger,
        _ => ui::BadgeTone::Muted,
    }
}

/// 状态徽章的短标签
fn badge(status: &Status) -> &'static str {
    match status {
        Status::Unavailable => "不可用",
        Status::Idle => "未检查",
        Status::Checking => "检查中",
        Status::UpToDate => "已是最新",
        Status::Available(_) => "有新版本",
        Status::Downloading { .. } => "下载中",
        Status::Ready { .. } => "待更新",
        Status::Installing => "更新中",
        Status::Failed(_) => "失败",
    }
}

/// 说明行：一句话解释这一档在做什么、下一步会怎样
fn hint(status: &Status) -> String {
    match status {
        Status::Unavailable => "开发运行，没有可替换的安装".to_owned(),
        Status::Idle => "退出 reminder 时自动安装新版本".to_owned(),
        Status::Checking => "正在检查新版本…".to_owned(),
        Status::UpToDate => "已经是最新版本".to_owned(),
        // 装不进去的目录不画下载按钮，直接给下载页
        Status::Available(found) if !found.can_install => "应用目录不可写，请下载后手动替换".to_owned(),
        Status::Available(found) => match found.size_label() {
            Some(size) => format!("新版本 {} · 约 {size}", found.version),
            None => format!("新版本 {}", found.version),
        },
        Status::Downloading {
            received, total, ..
        } => match total {
            Some(total) => format!(
                "下载中 {:.0}% · {} / {}",
                status.progress() * 100.0,
                updater::bytes_label(*received),
                updater::bytes_label(*total)
            ),
            None => format!("已下载 {}", updater::bytes_label(*received)),
        },
        Status::Ready { version } => format!("已下载 {version} · 重启生效，或下次退出时自动安装"),
        Status::Installing => "正在替换应用，马上重启…".to_owned(),
        Status::Failed(message) => message.clone(),
    }
}

/// 有新版本时，更新说明里第一句有用的话（没有说明就不占这一行）
fn notes(status: &Status) -> String {
    match status {
        Status::Available(found) => found.summary(),
        _ => String::new(),
    }
}

/// 装不进应用目录时，用户该去的地方：清单里那个包的地址
fn manual_url(status: &Status) -> Option<String> {
    match status {
        Status::Available(found) if !found.can_install => Some(found.url.clone()),
        _ => None,
    }
}

/// 用系统浏览器打开下载页
fn open_download_page(url: String) {
    spawn(async move {
        let _ = nsh_core::element::open_url(url).await;
    });
}

fn hint_color(status: &Status) -> AdaptiveColor {
    match status {
        Status::Failed(_) => ACCENT_DANGER_FG,
        _ => theme().muted_foreground,
    }
}

/// 应用更新区（设置抽屉第 5 区块）
pub fn render(app: AppCtx) -> impl IntoElement {
    let AppCtx {
        update,
        update_version,
        ..
    } = app;
    let status = computed(move || update.get());
    let version = computed(move || format!("v{}", update_version.get()));
    let hint = computed(move || hint(&status.get()));
    let notes = computed(move || notes(&status.get()));
    let page = computed(move || manual_url(&status.get()));

    view()
        .test_id("update-setting")
        .flex_col()
        .gap(px(4.0))
        // 1. 标题行：刷新图标 + 名称 + 当前版本，右侧是这一档唯一的动作
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .gap(px(6.0))
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        .child(icons::icon_svg(
                            icons::ICON_REFRESH,
                            12.0,
                            ACCENT_GOLD_STRONG,
                        ))
                        .child(
                            text("应用更新")
                                .text_size(px(font::TITLE))
                                .text_color(move || theme().foreground),
                        )
                        .child(
                            text(move || version.get())
                                .text_size(px(font::CAPTION))
                                .text_color(move || theme().muted_foreground),
                        ),
                )
                .child(when(
                    move || {
                        matches!(
                            status.get(),
                            Status::Idle | Status::UpToDate | Status::Failed(_)
                        )
                    },
                    move || ui::text_button("检查更新", false, move || app.check_update()),
                ))
                .child(when(
                    move || matches!(status.get(), Status::Available(found) if found.can_install),
                    move || ui::text_button("下载更新", true, move || app.download_update()),
                ))
                .child(when(
                    move || page.get().is_some(),
                    move || {
                        ui::text_button("打开下载页", true, move || {
                            if let Some(url) = page.get() {
                                open_download_page(url);
                            }
                        })
                    },
                ))
                .child(when(
                    move || matches!(status.get(), Status::Ready { .. }),
                    move || ui::text_button("重启并更新", true, move || app.restart_and_update()),
                )),
        )
        // 2. 状态行：徽章 + 一句话说明（失败的说明用警示色）
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(ui::status_badge(
                    move || tone(&status.get()),
                    move || badge(&status.get()),
                ))
                .child(
                    view().flex_1().child(
                        text(move || hint.get())
                            .text_size(px(font::LABEL))
                            .text_color(move || hint_color(&status.get())),
                    ),
                ),
        )
        // 3. 下载进度（只在下载中占一行）
        .child(when(
            move || matches!(status.get(), Status::Downloading { .. }),
            move || ui::progress_bar(move || status.get().progress()),
        ))
        // 4. 更新说明摘要（有新版本且说明非空时）
        .child(when(
            move || !notes.get().is_empty(),
            move || {
                view().test_id("update-notes").child(
                    text(move || format!("「{}」", notes.get()))
                        .text_size(px(font::CAPTION))
                        .text_color(move || theme().muted_foreground),
                )
            },
        ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::updater::Found;

    fn found() -> Found {
        Found {
            version: "0.3.0".to_owned(),
            notes: "# 更新内容\n\n- 支持退出时自动更新\n- 修复托盘图标".to_owned(),
            url: "https://example.com/reminder.zip".to_owned(),
            size: Some(19_400_000),
            sha256: "0".repeat(64),
            can_install: true,
        }
    }

    /// 装在只读目录里的那一种：不能自己装，只能给下载页
    fn locked() -> Found {
        Found {
            can_install: false,
            ..found()
        }
    }

    #[test]
    fn every_state_has_a_badge_and_a_hint() {
        for status in [
            Status::Unavailable,
            Status::Idle,
            Status::Checking,
            Status::UpToDate,
            Status::Available(found()),
            Status::Available(locked()),
            Status::Downloading {
                version: "0.3.0".to_owned(),
                received: 1,
                total: Some(2),
            },
            Status::Ready {
                version: "0.3.0".to_owned(),
            },
            Status::Installing,
            Status::Failed("连接失败：超时".to_owned()),
        ] {
            assert!(!badge(&status).is_empty(), "{status:?} 没有徽章");
            assert!(!hint(&status).is_empty(), "{status:?} 没有说明");
        }
    }

    #[test]
    fn only_actionable_states_are_accent() {
        assert_eq!(tone(&Status::UpToDate), ui::BadgeTone::Muted);
        assert_eq!(tone(&Status::Available(found())), ui::BadgeTone::Accent);
        assert_eq!(
            tone(&Status::Ready {
                version: "0.3.0".to_owned()
            }),
            ui::BadgeTone::Accent
        );
        assert_eq!(tone(&Status::Installing), ui::BadgeTone::Accent);
        assert_eq!(
            tone(&Status::Failed("x".to_owned())),
            ui::BadgeTone::Danger
        );
    }

    #[test]
    fn hints_name_the_version_and_the_size() {
        assert!(hint(&Status::Available(found())).contains("0.3.0"));
        assert!(hint(&Status::Available(found())).contains("18.5 MB"));
        assert!(hint(&Status::Ready {
            version: "0.3.0".to_owned()
        })
        .contains("重启生效"));
    }

    /// 装在只读目录：说明改成「请手动替换」，并给出下载页
    #[test]
    fn a_locked_install_offers_the_download_page_instead() {
        let read_only = Status::Available(locked());
        assert!(hint(&read_only).contains("不可写"));
        assert_eq!(
            manual_url(&read_only).as_deref(),
            Some("https://example.com/reminder.zip")
        );
        // 能自己装的时候没有下载页按钮，只有「下载更新」
        assert!(manual_url(&Status::Available(found())).is_none());
        assert!(manual_url(&Status::Ready {
            version: "0.3.0".to_owned()
        })
        .is_none());
    }

    #[test]
    fn notes_appear_only_with_a_new_version() {
        assert_eq!(notes(&Status::Idle), "");
        assert_eq!(notes(&Status::Available(found())), "支持退出时自动更新");
    }
}
