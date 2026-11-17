use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use serde::{Deserialize, Serialize};

use crate::localtime::LocalNow;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// 一天的秒数
const SECS_PER_DAY: u32 = 86_400;

fn default_item_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

/// 初始化下一个分配的 ID，确保反序列化后新建项目的 ID 不与已有项目冲突
pub fn init_next_id(start_id: u64) {
    let mut current = NEXT_ID.load(Ordering::Relaxed);
    while current < start_id {
        match NEXT_ID.compare_exchange_weak(current, start_id, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(actual) => current = actual,
        }
    }
}

/// 提醒的计时方式：固定间隔循环，或每天固定时刻。
///
/// 持久化时写成 `{"kind":"interval"}` / `{"kind":"daily","hour":12,"minute":0}`；
/// 旧版本数据没有该字段，反序列化后按间隔循环处理。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Schedule {
    /// 每隔 `interval_secs` 提醒一次
    Interval,
    /// 每天 `hour:minute` 提醒一次（本地时间）
    Daily { hour: u8, minute: u8 },
}

impl Default for Schedule {
    fn default() -> Self {
        Self::Interval
    }
}

impl Schedule {
    /// 构造一个合法的每日定点时刻（越界值收敛到当天范围内）
    pub fn daily(hour: u32, minute: u32) -> Self {
        Self::Daily {
            hour: hour.min(23) as u8,
            minute: minute.min(59) as u8,
        }
    }

    pub fn is_daily(self) -> bool {
        matches!(self, Self::Daily { .. })
    }

    /// 每日定点的（时, 分）；间隔循环返回 `None`
    pub fn daily_time(self) -> Option<(u8, u8)> {
        match self {
            Self::Daily { hour, minute } => Some((hour, minute)),
            Self::Interval => None,
        }
    }
}

/// 提醒项目模型（纯数据结构，与 UI 生命周期 Owner 解耦，安全可靠）
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReminderItem {
    #[serde(default = "default_item_id")]
    pub id: u64,
    pub name: String,
    pub icon: String,
    pub interval_secs: u32,
    #[serde(default)]
    pub remaining_secs: u32,
    #[serde(default)]
    pub completed_count: u32,
    /// 按本地日期序号记录每天完成打卡的次数（见 [`LocalNow::day`]）。
    ///
    /// 界面只读取当天的次数；更早的日期继续保留，供以后做历史统计。
    /// 日期序号自 1970-01-01 起算，序列化为 `{"20000": 2}` 这样的键值对。
    #[serde(default)]
    pub completed_by_day: BTreeMap<i64, u32>,
    #[serde(default)]
    pub is_paused: bool,
    /// 倒计时归零后的超时持续秒数（用于超时动态统计与持续提醒，避免静止在0）
    #[serde(default)]
    pub overdue_secs: u32,
    /// 计时方式（间隔循环 / 每天定点）
    #[serde(default)]
    pub schedule: Schedule,
    /// 定点提醒：正在等待打卡的那一次提醒的**计划日**（本地日期序号，见
    /// [`LocalNow::day`]）。23:00 推迟到次日 00:03 才响时，它仍是前一天。
    #[serde(default)]
    pub fired_day: Option<i64>,
    /// 定点提醒：这一次实际响铃的本地日期序号（与 [`Self::fired_day`] 不同，
    /// 用于判断要不要按「今天的那一次」展示超时、以及打卡归属哪一次）。
    #[serde(default)]
    pub fired_on: Option<i64>,
    /// 定点提醒：这一次响铃的时刻（Unix 秒），超时从它开始累计
    #[serde(default)]
    pub fired_at: Option<u64>,
    /// 定点提醒：已打卡的那一次提醒的**计划日**（本地日期序号）
    #[serde(default)]
    pub done_day: Option<i64>,
    /// 定点提醒：本次被推迟到的时刻（Unix 秒）
    #[serde(default)]
    pub snooze_until: Option<u64>,
    /// 定点提醒：被推迟的是哪一次提醒的**计划日**
    #[serde(default)]
    pub snooze_day: Option<i64>,
}

impl ReminderItem {
    /// 固定间隔循环的提醒
    pub fn new(name: impl Into<String>, icon: impl Into<String>, interval_secs: u32) -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        Self {
            id,
            name: name.into(),
            icon: icon.into(),
            interval_secs,
            remaining_secs: interval_secs,
            completed_count: 0,
            completed_by_day: BTreeMap::new(),
            is_paused: false,
            overdue_secs: 0,
            schedule: Schedule::Interval,
            fired_day: None,
            fired_on: None,
            fired_at: None,
            done_day: None,
            snooze_until: None,
            snooze_day: None,
        }
    }

    /// 每天固定时刻的提醒（HH:MM，本地时间）。
    ///
    /// 创建时若今天的时刻已经过去，就从明天开始，不会立刻补响一次。
    pub fn new_daily(
        name: impl Into<String>,
        icon: impl Into<String>,
        hour: u32,
        minute: u32,
        now: &LocalNow,
    ) -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let mut item = Self {
            id,
            name: name.into(),
            icon: icon.into(),
            interval_secs: SECS_PER_DAY,
            remaining_secs: 0,
            completed_count: 0,
            completed_by_day: BTreeMap::new(),
            is_paused: false,
            overdue_secs: 0,
            schedule: Schedule::daily(hour, minute),
            fired_day: None,
            fired_on: None,
            fired_at: None,
            done_day: None,
            snooze_until: None,
            snooze_day: None,
        };
        let today_due = item.today_due(now).unwrap_or(now.unix);
        if now.unix < today_due {
            item.remaining_secs = clamp_secs(today_due - now.unix);
        } else {
            // 今天的时刻已过：跳过今天这一次，从明天开始
            item.done_day = Some(now.day);
            item.remaining_secs = item
                .tomorrow_due(now)
                .map_or(0, |due| clamp_secs(due.saturating_sub(now.unix)));
        }
        item
    }

    pub fn is_daily(&self) -> bool {
        self.schedule.is_daily()
    }

    /// 定点提醒今天这一次是否已完成（此时 +5 不会生效）
    pub fn is_done_today(&self, now: &LocalNow) -> bool {
        self.is_daily() && self.done_day == Some(now.day)
    }

    /// `now` 所在本地日期已打卡的次数；只统计当天动作，不包含历史累计
    pub fn today_completed(&self, now: &LocalNow) -> u32 {
        self.completed_by_day.get(&now.day).copied().unwrap_or(0)
    }

    /// 列表右侧的当天统计文本（如「今日 3 次」）
    pub fn today_label(&self, now: &LocalNow) -> String {
        format!("今日 {} 次", self.today_completed(now))
    }

    /// 每日定点的（时, 分）
    pub fn daily_time(&self) -> Option<(u8, u8)> {
        self.schedule.daily_time()
    }

    /// 今天的定时时刻（Unix 秒）；可能已经过去
    pub fn today_due(&self, now: &LocalNow) -> Option<u64> {
        let (hour, minute) = self.daily_time()?;
        Some(local_target_unix(now, hour, minute, 0))
    }

    /// 明天的定时时刻（Unix 秒）
    pub fn tomorrow_due(&self, now: &LocalNow) -> Option<u64> {
        let (hour, minute) = self.daily_time()?;
        Some(local_target_unix(now, hour, minute, 1))
    }

    /// 完成打卡：计数 +1；间隔提醒重置回设定周期，定点提醒跳过对应的一次。
    ///
    /// 打卡归属哪一次提醒：今天刚响过（包括昨晚推迟到凌晨才响的那一次）就归属它；
    /// 推迟还没响就打卡则归属被推迟的那次；否则归属今天这一次（提前打卡会跳过今天）。
    pub fn complete(&mut self, now: &LocalNow) {
        self.completed_count = self.completed_count.saturating_add(1);
        let today = self.completed_by_day.entry(now.day).or_insert(0);
        *today = today.saturating_add(1);
        self.overdue_secs = 0;
        if !self.is_daily() {
            self.remaining_secs = self.interval_secs;
            return;
        }

        let occurrence = if self.fired_on == Some(now.day) {
            self.fired_day.unwrap_or(now.day)
        } else if self.snooze_until.is_some() {
            self.snooze_day.unwrap_or(now.day)
        } else {
            now.day
        };
        self.done_day = Some(occurrence);
        self.fired_day = None;
        self.fired_on = None;
        self.fired_at = None;
        self.snooze_until = None;
        self.snooze_day = None;

        // 下一次提醒：打卡的是更早的那次就还有今天这一次（若还没过），否则顺延到明天
        let next = if occurrence < now.day {
            self.today_due(now).filter(|due| *due > now.unix)
        } else {
            None
        }
        .or_else(|| self.tomorrow_due(now));
        self.remaining_secs = next.map_or(0, |due| clamp_secs(due.saturating_sub(now.unix)));
    }

    /// 推迟提醒（例如加 5 分钟 = 300 秒），清空超时状态。
    ///
    /// 定点提醒推迟的是「当前这一次」：连续点击会不断累加；已经完成今天这一次时
    /// 不做任何事（避免把已完成的提醒复活）。返回是否真的改动了状态。
    pub fn snooze(&mut self, extra_secs: u32, now: &LocalNow) -> bool {
        self.overdue_secs = 0;
        if !self.is_daily() {
            self.remaining_secs += extra_secs;
            return true;
        }

        // 今天这一次已经完成：+5 不生效
        if self.done_day == Some(now.day) {
            return false;
        }

        // 已经在推迟中：逐次叠加（连续点 +5 每次都往后加 5 分钟）
        if let Some(until) = self.snooze_until {
            let until = until + u64::from(extra_secs);
            self.snooze_until = Some(until);
            self.remaining_secs = clamp_secs(until.saturating_sub(now.unix));
            return true;
        }

        // 刚刚响过、还在等打卡：从现在起往后推，仍然属于同一次提醒
        if let Some(occurrence) = self.fired_day.filter(|_| self.fired_on == Some(now.day)) {
            let base = now.unix.max(self.fired_at.unwrap_or(now.unix));
            let until = base + u64::from(extra_secs);
            self.snooze_day = Some(occurrence);
            self.snooze_until = Some(until);
            self.fired_day = None;
            self.fired_on = None;
            self.fired_at = None;
            self.remaining_secs = clamp_secs(until.saturating_sub(now.unix));
            return true;
        }

        // 还没到点：把今天的定时时刻往后推
        let due = self.today_due(now).unwrap_or(now.unix);
        let until = due.max(now.unix) + u64::from(extra_secs);
        self.snooze_day = Some(now.day);
        self.snooze_until = Some(until);
        self.fired_day = None;
        self.fired_on = None;
        self.fired_at = None;
        self.remaining_secs = clamp_secs(until.saturating_sub(now.unix));
        true
    }

    /// 切换暂停/恢复
    pub fn toggle_pause(&mut self) {
        self.is_paused = !self.is_paused;
    }

    /// 定时器每秒 Tick：间隔提醒递减倒计时，定点提醒按本地墙钟重算。
    /// 返回 true 表示这一秒恰好到点（用于触发提示音、系统通知与窗口唤起）
    pub fn tick(&mut self, now: &LocalNow) -> bool {
        if self.is_paused {
            return false;
        }
        if self.is_daily() {
            return self.tick_daily(now);
        }
        if self.remaining_secs > 0 {
            self.remaining_secs -= 1;
            if self.remaining_secs == 0 {
                return true;
            }
        } else {
            self.overdue_secs = self.overdue_secs.saturating_add(1);
        }
        false
    }

    /// 定点提醒每秒的推进：只按墙上时间判断，不依赖 tick 的调用间隔
    fn tick_daily(&mut self, now: &LocalNow) -> bool {
        let today = now.day;
        // 清理时钟回拨留下的“未来”记录
        if self.fired_day.is_some_and(|day| day > today)
            || self.fired_on.is_some_and(|day| day > today)
        {
            self.fired_day = None;
            self.fired_on = None;
            self.fired_at = None;
        }
        if self.done_day.is_some_and(|day| day > today) {
            self.done_day = None;
        }
        if self.snooze_day.is_some_and(|day| day > today) {
            self.snooze_day = None;
            self.snooze_until = None;
        }

        // 1. 已推迟：等推迟时刻到点。响铃时记下“计划日”，推迟跨天也不会认错是哪一次
        if let Some(until) = self.snooze_until {
            if now.unix < until {
                self.remaining_secs = clamp_secs(until - now.unix);
                self.overdue_secs = 0;
                return false;
            }
            let occurrence = self.snooze_day.unwrap_or(today);
            self.snooze_until = None;
            self.snooze_day = None;
            self.fired_day = Some(occurrence);
            self.fired_on = Some(today);
            self.fired_at = Some(until);
            self.remaining_secs = 0;
            self.overdue_secs = 0;
            return true;
        }

        // 2. 今天已打卡：倒计时到明天
        if self.done_day == Some(today) {
            self.remaining_secs = self
                .tomorrow_due(now)
                .map_or(0, |due| clamp_secs(due.saturating_sub(now.unix)));
            self.overdue_secs = 0;
            return false;
        }

        // 3. 今天这一次已响铃：等待打卡，超时持续累计
        if self.fired_day == Some(today) {
            let anchor = self.fired_at.unwrap_or(now.unix);
            self.remaining_secs = 0;
            self.overdue_secs = clamp_secs(now.unix.saturating_sub(anchor));
            return false;
        }

        // 4. 还没到今天的定时时刻。若上一次响铃是昨晚推迟到凌晨的那一次，
        //    不再把它的超时带进新的一天，只安静倒计时到今天的时刻
        let due = self.today_due(now).unwrap_or(now.unix);
        if now.unix < due {
            self.remaining_secs = clamp_secs(due - now.unix);
            self.overdue_secs = 0;
            return false;
        }

        // 5. 到点
        self.fired_day = Some(today);
        self.fired_on = Some(today);
        self.fired_at = Some(due);
        self.remaining_secs = 0;
        self.overdue_secs = 0;
        true
    }

    /// 启动 / 修改系统时间后用当前本地时间校正状态，不触发响铃。
    ///
    /// 定时时刻已过且从未响铃（应用当时没运行）按「超时」展示，
    /// 标记为已响铃以免启动时补响一次。
    pub fn refresh(&mut self, now: &LocalNow) {
        if !self.is_daily() {
            return;
        }
        let today = now.day;
        // 清理时钟回拨留下的“未来”记录
        if self.fired_day.is_some_and(|day| day > today)
            || self.fired_on.is_some_and(|day| day > today)
        {
            self.fired_day = None;
            self.fired_on = None;
            self.fired_at = None;
        }
        if self.done_day.is_some_and(|day| day > today) {
            self.done_day = None;
        }
        if self.snooze_day.is_some_and(|day| day > today) {
            self.snooze_day = None;
            self.snooze_until = None;
        }

        // 推迟还没到点：继续倒计时
        if let Some(until) = self.snooze_until {
            if now.unix < until {
                self.remaining_secs = clamp_secs(until - now.unix);
                self.overdue_secs = 0;
                return;
            }
            let occurrence = self.snooze_day.unwrap_or(today);
            self.snooze_until = None;
            self.snooze_day = None;
            if occurrence == today {
                // 今天这一次的推迟已经过点：按超时展示，不补响
                self.fired_day = Some(today);
                self.fired_on = Some(today);
                self.fired_at = Some(until);
            }
            // occurrence 是更早的日子：应用不在时错过的那次不再追溯，按新的一天倒计时
        }

        if self.done_day == Some(today) {
            self.remaining_secs = self
                .tomorrow_due(now)
                .map_or(0, |due| clamp_secs(due.saturating_sub(now.unix)));
            self.overdue_secs = 0;
            return;
        }

        if self.fired_day == Some(today) {
            let anchor = self.fired_at.unwrap_or(now.unix);
            self.remaining_secs = 0;
            self.overdue_secs = clamp_secs(now.unix.saturating_sub(anchor));
            return;
        }

        let due = self.today_due(now).unwrap_or(now.unix);
        if now.unix < due {
            self.remaining_secs = clamp_secs(due - now.unix);
            self.overdue_secs = 0;
        } else {
            self.fired_day = Some(today);
            self.fired_on = Some(today);
            self.fired_at = Some(due);
            self.remaining_secs = 0;
            self.overdue_secs = clamp_secs(now.unix - due);
        }
    }

    /// 当前是否处于已到期状态
    pub fn is_expired(&self) -> bool {
        self.remaining_secs == 0
    }

    /// 格式化当前剩余时间或超时时间（纯文本，绝无 Emoji，符合专业高质感设计目标）
    pub fn format_remaining(&self) -> String {
        if self.remaining_secs == 0 {
            if self.overdue_secs == 0 {
                "到时间啦!".to_string()
            } else {
                format!("超时 {}", format_clock(self.overdue_secs))
            }
        } else {
            format_clock(self.remaining_secs)
        }
    }

    /// 进度比例（0.0 ~ 1.0）
    pub fn progress_ratio(&self) -> f32 {
        let total = self.interval_secs.max(1) as f32;
        (self.remaining_secs as f32 / total).clamp(0.0, 1.0)
    }

    /// 格式化循环周期（如 "30m"、"1h"、"1h30m"；定点提醒返回 "12:00"）
    pub fn format_interval(&self) -> String {
        if let Some((hour, minute)) = self.daily_time() {
            return format!("{hour:02}:{minute:02}");
        }
        let mins = self.interval_secs / 60;
        if mins >= 60 {
            let h = mins / 60;
            let m = mins % 60;
            if m == 0 {
                format!("{h}h")
            } else {
                format!("{h}h{m}m")
            }
        } else {
            format!("{mins}m")
        }
    }

    /// 列表里的计时方式徽章："30m" / "每天 12:00"
    pub fn format_badge(&self) -> String {
        match self.daily_time() {
            Some((hour, minute)) => format!("每天 {hour:02}:{minute:02}"),
            None => self.format_interval(),
        }
    }

    /// 就地更新循环周期时长（分钟），并将超出新周期的倒计时拉齐
    pub fn update_interval_mins(&mut self, mins: u32) {
        if self.is_daily() {
            return;
        }
        let new_secs = mins.max(1) * 60;
        self.interval_secs = new_secs;
        if self.remaining_secs > new_secs {
            self.remaining_secs = new_secs;
        }
    }
}

/// 本地定时时刻换算成 Unix 秒：`day_offset` 为 0 是今天，1 是明天。
///
/// 按当前时刻的时区偏移换算（跨夏令时切换的日子会差一小时，留给下一次 tick 校正）。
fn local_target_unix(now: &LocalNow, hour: u8, minute: u8, day_offset: i64) -> u64 {
    let local_epoch = now.unix as i64 + now.offset;
    let day_start = local_epoch - local_epoch.rem_euclid(i64::from(SECS_PER_DAY));
    let target = i64::from(hour) * 3600 + i64::from(minute) * 60;
    let local_target = day_start + day_offset * i64::from(SECS_PER_DAY) + target;
    (local_target - now.offset).max(0) as u64
}

fn clamp_secs(secs: u64) -> u32 {
    secs.min(u64::from(u32::MAX)) as u32
}

/// 时间格式：不足一小时用 "MM:SS"，超过一小时用 "HH:MM:SS"
fn format_clock(secs: u32) -> String {
    if secs >= 3600 {
        format!("{:02}:{:02}:{:02}", secs / 3600, (secs % 3600) / 60, secs % 60)
    } else {
        format!("{:02}:{:02}", secs / 60, secs % 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造一个确定性的本地时刻（偏移 0，便于断言）
    fn local(day: i64, secs_of_day: u32) -> LocalNow {
        LocalNow {
            unix: day as u64 * u64::from(SECS_PER_DAY) + u64::from(secs_of_day),
            day,
            secs_of_day,
            hour: secs_of_day / 3600,
            minute: (secs_of_day / 60) % 60,
            second: secs_of_day % 60,
            weekday: (day + 4).rem_euclid(7) as u32,
            offset: 0,
        }
    }

    const TEN_AM: u32 = 10 * 3600;
    const NOON: u32 = 12 * 3600;

    #[test]
    fn test_reminder_item_lifecycle() {
        let now = local(20_000, 0);
        let mut item = ReminderItem::new("测试喝水", "droplet", 60);
        assert_eq!(item.remaining_secs, 60);
        assert_eq!(item.completed_count, 0);
        assert_eq!(item.format_remaining(), "01:00");
        assert!(!item.is_expired());

        // 模拟 1 秒倒计时
        let expired = item.tick(&now);
        assert!(!expired);
        assert_eq!(item.remaining_secs, 59);
        assert_eq!(item.format_remaining(), "00:59");

        // 推迟 5 分钟 (300秒)
        item.snooze(300, &now);
        assert_eq!(item.remaining_secs, 359);
        assert_eq!(item.overdue_secs, 0);

        // 完成打卡
        item.complete(&now);
        assert_eq!(item.completed_count, 1);
        assert_eq!(item.remaining_secs, 60);

        // 暂停测试
        item.toggle_pause();
        assert!(item.is_paused);
        let expired = item.tick(&now);
        assert!(!expired);
        assert_eq!(item.remaining_secs, 60); // 暂停时不递减

        // 恢复并递减至归零
        item.toggle_pause();
        item.remaining_secs = 1;
        let expired = item.tick(&now);
        assert!(expired); // 刚刚归零触发提醒
        assert_eq!(item.remaining_secs, 0);
        assert!(item.is_expired());
        assert_eq!(item.format_remaining(), "到时间啦!");

        // 归零后继续 tick：累加超时时间，避免永远停在0
        let expired = item.tick(&now);
        assert!(!expired);
        assert_eq!(item.overdue_secs, 1);
        assert_eq!(item.format_remaining(), "超时 00:01");

        // 周期格式化测试
        assert_eq!(item.format_interval(), "1m");
        item.update_interval_mins(90);
        assert_eq!(item.interval_secs, 90 * 60);
        assert_eq!(item.format_interval(), "1h30m");
        item.update_interval_mins(120);
        assert_eq!(item.format_interval(), "2h");
    }

    #[test]
    fn completions_are_recorded_per_local_day() {
        let mut item = ReminderItem::new("喝水", "droplet", 30 * 60);
        item.complete(&local(20_000, NOON));
        item.complete(&local(20_000, NOON + 60));
        item.complete(&local(20_001, TEN_AM));
        item.complete(&local(20_001, NOON));

        // 累计次数保留，同时按天留档，供以后做统计时使用
        assert_eq!(item.completed_count, 4);
        assert_eq!(item.completed_by_day.get(&20_000), Some(&2));
        assert_eq!(item.completed_by_day.get(&20_001), Some(&2));
        // 界面只取当天：第一天显示 2 次，第二天显示 2 次，其他日子 0 次
        assert_eq!(item.today_completed(&local(20_000, NOON)), 2);
        assert_eq!(item.today_label(&local(20_000, NOON)), "今日 2 次");
        assert_eq!(item.today_completed(&local(20_001, NOON)), 2);
        assert_eq!(item.today_label(&local(20_002, NOON)), "今日 0 次");
        // 没有打卡的日子不产生记录
        assert_eq!(item.completed_by_day.get(&19_999), None);
    }

    #[test]
    fn daily_item_counts_down_rings_and_schedules_tomorrow_after_check_in() {
        let created = local(20_000, TEN_AM);
        let mut item = ReminderItem::new_daily("午餐", "apple", 12, 0, &created);
        assert!(item.is_daily());
        assert_eq!(item.format_badge(), "每天 12:00");
        assert_eq!(item.remaining_secs, 2 * 3600);
        assert_eq!(item.format_remaining(), "02:00:00");

        // 11:59:58 还差 2 秒，不响
        assert!(!item.tick(&local(20_000, NOON - 2)));
        assert_eq!(item.remaining_secs, 2);
        assert_eq!(item.format_remaining(), "00:02");

        // 12:00:00 到点响铃
        assert!(item.tick(&local(20_000, NOON)));
        assert!(item.is_expired());
        assert_eq!(item.format_remaining(), "到时间啦!");

        // 没打卡就持续累计超时
        assert!(!item.tick(&local(20_000, NOON + 75)));
        assert_eq!(item.overdue_secs, 75);
        assert_eq!(item.format_remaining(), "超时 01:15");

        // 打卡后进入明天的倒计时，今天不再响
        item.complete(&local(20_000, NOON + 120));
        assert_eq!(item.completed_count, 1);
        assert_eq!(item.remaining_secs, u32::from(SECS_PER_DAY) - 120);
        assert!(!item.tick(&local(20_000, NOON + 130)));
        assert!(!item.is_expired());

        // 第二天 12:00 再响
        assert!(!item.tick(&local(20_001, NOON - 1)));
        assert!(item.tick(&local(20_001, NOON)));
    }

    #[test]
    fn daily_item_created_after_its_time_starts_tomorrow() {
        let created = local(20_000, 13 * 3600);
        let mut item = ReminderItem::new_daily("午餐", "apple", 12, 0, &created);
        assert_eq!(item.remaining_secs, 23 * 3600);
        // 今天 12:00 早已过去，不会补响
        assert!(!item.tick(&local(20_000, 13 * 3600 + 1)));
        assert!(!item.is_expired());
    }

    #[test]
    fn daily_item_early_check_in_skips_today() {
        let created = local(20_000, TEN_AM);
        let mut item = ReminderItem::new_daily("午餐", "apple", 12, 0, &created);
        item.complete(&local(20_000, TEN_AM + 60));
        // 明天 12:00 减去现在 10:01
        assert_eq!(item.remaining_secs, u32::from(SECS_PER_DAY) + 2 * 3600 - 60);
        // 今天 12:00 不再响
        assert!(!item.tick(&local(20_000, NOON)));
        assert!(!item.is_expired());
        // 第二天照常响
        assert!(item.tick(&local(20_001, NOON)));
    }

    #[test]
    fn daily_item_snooze_pushes_this_occurrence() {
        let created = local(20_000, 11 * 3600 + 50 * 60);
        let mut item = ReminderItem::new_daily("午餐", "apple", 12, 0, &created);
        item.snooze(300, &local(20_000, 11 * 3600 + 50 * 60));
        assert_eq!(item.remaining_secs, 15 * 60);
        assert!(!item.is_expired());

        // 12:04 还差 1 分钟
        assert!(!item.tick(&local(20_000, NOON + 240)));
        assert_eq!(item.remaining_secs, 60);
        // 12:05 响
        assert!(item.tick(&local(20_000, NOON + 300)));
        assert!(item.is_expired());
        assert_eq!(item.overdue_secs, 0);
        // 之后超时从推迟时刻算起
        assert!(!item.tick(&local(20_000, NOON + 360)));
        assert_eq!(item.overdue_secs, 60);
    }

    #[test]
    fn daily_item_snooze_accumulates_across_repeated_presses() {
        let created = local(20_000, 11 * 3600 + 50 * 60);
        let mut item = ReminderItem::new_daily("午餐", "apple", 12, 0, &created);
        // 连续点 3 次 +5：12:00 → 12:15，而不是每次都落在 12:05
        assert!(item.snooze(300, &local(20_000, 11 * 3600 + 50 * 60)));
        assert!(item.snooze(300, &local(20_000, 11 * 3600 + 51 * 60)));
        assert!(item.snooze(300, &local(20_000, 11 * 3600 + 52 * 60)));
        assert_eq!(item.remaining_secs, 23 * 60);

        assert!(!item.tick(&local(20_000, NOON + 14 * 60 + 59)));
        assert_eq!(item.remaining_secs, 1);
        assert!(item.tick(&local(20_000, NOON + 15 * 60)));
    }

    #[test]
    fn daily_item_snooze_after_check_in_does_not_revive_today() {
        // 提前打卡：今天这一次直接完成，+5 不能把它复活
        let created = local(20_000, TEN_AM);
        let mut item = ReminderItem::new_daily("午餐", "apple", 12, 0, &created);
        item.complete(&local(20_000, TEN_AM));
        let remaining = item.remaining_secs;
        assert!(!item.snooze(300, &local(20_000, TEN_AM + 60)));
        assert!(item.snooze_until.is_none());
        assert_eq!(item.done_day, Some(20_000));
        assert_eq!(item.remaining_secs, remaining);
        // 到点不会误响
        assert!(!item.tick(&local(20_000, NOON)));

        // 响过之后打卡，同样不能再复活
        let mut fired = ReminderItem::new_daily("午餐", "apple", 12, 0, &created);
        assert!(fired.tick(&local(20_000, NOON)));
        fired.complete(&local(20_000, NOON + 300));
        let remaining = fired.remaining_secs;
        assert!(!fired.snooze(300, &local(20_000, NOON + 360)));
        assert_eq!(fired.remaining_secs, remaining);
        assert!(fired.snooze_until.is_none());
    }

    #[test]
    fn daily_snooze_across_midnight_keeps_the_occurrence_and_next_day() {
        let created = local(20_000, 22 * 3600);
        let mut item = ReminderItem::new_daily("睡觉", "moon", 23, 0, &created);
        assert!(item.tick(&local(20_000, 23 * 3600)));

        // 23:58 推迟 5 分钟 → 次日 00:03 才响
        assert!(item.snooze(300, &local(20_000, 23 * 3600 + 58 * 60)));
        assert!(!item.tick(&local(20_001, 2 * 60)));
        assert!(item.tick(&local(20_001, 3 * 60)));
        // 响的是「昨晚 23:00」那一次，而不是新的一天
        assert_eq!(item.fired_day, Some(20_000));
        assert_eq!(item.fired_on, Some(20_001));

        // 紧接着的下一分钟就不再把超时带进新的一天
        assert!(!item.tick(&local(20_001, 4 * 60)));
        assert!(!item.is_expired());
        assert_eq!(item.overdue_secs, 0);
        assert_eq!(item.remaining_secs, 23 * 3600 - 4 * 60);

        // 次日白天不再显示巨额超时，只倒计时到今晚 23:00
        let morning = local(20_001, 8 * 3600);
        assert!(!item.tick(&morning));
        assert!(!item.is_expired());
        assert_eq!(item.overdue_secs, 0);
        assert_eq!(item.remaining_secs, 15 * 3600);

        // 白天打卡只结束昨晚那一次，今晚 23:00 仍然照常提醒
        item.complete(&morning);
        assert_eq!(item.done_day, Some(20_000));
        assert_eq!(item.remaining_secs, 15 * 3600);
        // 补打动作也计入当天的历史留档（20_001），不影响今晚这一次
        assert_eq!(item.completed_by_day.get(&20_001), Some(&1));
        assert!(!item.tick(&local(20_001, 22 * 3600)));
        assert!(item.tick(&local(20_001, 23 * 3600)));
    }

    #[test]
    fn daily_item_missed_while_off_shows_overdue_without_ringing() {
        let created = local(20_000, TEN_AM);
        let mut item = ReminderItem::new_daily("午餐", "apple", 12, 0, &created);
        // 应用在 13:00 才启动
        item.refresh(&local(20_000, 13 * 3600));
        assert!(item.is_expired());
        assert_eq!(item.overdue_secs, 3600);
        assert_eq!(item.format_remaining(), "超时 01:00:00");
        // 启动后的 tick 不会补响
        assert!(!item.tick(&local(20_000, 13 * 3600 + 1)));
        assert_eq!(item.overdue_secs, 3601);
    }

    #[test]
    fn daily_item_keeps_ringing_next_day_without_check_in() {
        let created = local(20_000, TEN_AM);
        let mut item = ReminderItem::new_daily("午餐", "apple", 12, 0, &created);
        assert!(item.tick(&local(20_000, NOON)));
        // 一直不打卡，第二天 12:00 重新提醒
        assert!(!item.tick(&local(20_001, NOON - 1)));
        assert!(item.tick(&local(20_001, NOON)));
    }

    #[test]
    fn legacy_interval_json_without_schedule_still_loads() {
        let legacy = r#"{"id":1,"name":"喝水","icon":"droplet","interval_secs":1800}"#;
        let item: ReminderItem = serde_json::from_str(legacy).unwrap();
        assert_eq!(item.schedule, Schedule::Interval);
        assert!(!item.is_daily());
        assert_eq!(item.remaining_secs, 0);
        assert!(item.completed_by_day.is_empty());
    }

    #[test]
    fn daily_item_json_round_trips() {
        let created = local(20_000, TEN_AM);
        let mut item = ReminderItem::new_daily("下班打卡", "laptop", 18, 30, &created);
        assert_eq!(item.format_badge(), "每天 18:30");
        item.fired_day = Some(20_000);
        item.fired_on = Some(20_000);
        item.fired_at = Some(created.unix + 8 * 3600 + 30 * 60);
        item.completed_by_day.insert(20_000, 2);
        let json = serde_json::to_string(&item).unwrap();
        let restored: ReminderItem = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, item);
        assert_eq!(restored.completed_by_day.get(&20_000), Some(&2));
        assert_eq!(
            restored.schedule,
            Schedule::Daily {
                hour: 18,
                minute: 30
            }
        );
    }

    #[test]
    fn clock_format_adds_hours_only_when_needed() {
        assert_eq!(format_clock(59), "00:59");
        assert_eq!(format_clock(600), "10:00");
        assert_eq!(format_clock(3600), "01:00:00");
        assert_eq!(format_clock(4 * 3600 + 5 * 60 + 9), "04:05:09");
        assert_eq!(format_clock(10 * 3600), "10:00:00");
    }
}
