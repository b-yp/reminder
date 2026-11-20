//! 本地墙钟时间：把 Unix 时间戳换算成用户所在时区的日期与时刻。
//!
//! 「每天 12:00 午餐」这类定点提醒必须按本地墙上时间到点，而
//! `nsh_core::time` 只给出 UTC 时刻，所以这里用各平台系统 API 补一次换算：
//! Windows 走 `SystemTimeToTzSpecificLocalTime`，Linux / macOS / iOS / Android
//! 走 C 库的 `localtime_r`，Web（wasm）等没有本地时区能力的平台退回 UTC。

use nsh_core::time::unix_ms;

/// 某个时刻在本地时区下的日期与时刻
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalNow {
    /// Unix 时间戳（UTC 秒）
    pub unix: u64,
    /// 本地日期序号：自 1970-01-01 起的天数（按本地日期算）
    pub day: i64,
    /// 本地当天第几秒（0..86400）
    pub secs_of_day: u32,
    /// 本地小时（0..24）
    pub hour: u32,
    /// 本地分钟（0..60）
    pub minute: u32,
    /// 本地秒（0..60）
    pub second: u32,
    /// 星期几，0 = 周日（与 `tm_wday` 一致）
    pub weekday: u32,
    /// 该时刻的时区偏移（秒；东八区 = 8 * 3600）
    pub offset: i64,
}

/// 当前时刻（读引擎时钟，因此手动时钟的测试里同样可复现）
pub fn now() -> LocalNow {
    at(unix_now())
}

/// 当前 Unix 时间戳（秒）
pub fn unix_now() -> u64 {
    let ms = unix_ms();
    if ms <= 0.0 { 0 } else { (ms / 1000.0) as u64 }
}

/// 把 Unix 时间戳换算成本地日期与时刻
pub fn at(unix: u64) -> LocalNow {
    let f = sys::local_fields(unix);
    let day = days_from_civil(f.year, f.month, f.day);
    let secs_of_day = f.hour * 3600 + f.minute * 60 + f.second;
    LocalNow {
        unix,
        day,
        secs_of_day,
        hour: f.hour,
        minute: f.minute,
        second: f.second,
        weekday: f.weekday,
        offset: day * 86_400 + i64::from(secs_of_day) - unix as i64,
    }
}

/// 本地日期与时刻的原始字段
struct LocalFields {
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    weekday: u32,
}

/// 没有本地时区信息的平台：按 UTC 处理
#[allow(dead_code)]
fn utc_fields(unix: u64) -> LocalFields {
    let days = (unix / 86_400) as i64;
    let secs = (unix % 86_400) as u32;
    let (year, month, day) = civil_from_days(days);
    LocalFields {
        year,
        month,
        day,
        hour: secs / 3600,
        minute: (secs % 3600) / 60,
        second: secs % 60,
        weekday: weekday_from_days(days),
    }
}

#[cfg(target_os = "windows")]
mod sys {
    use nsh_platform_windows::windows::Win32::Foundation::SYSTEMTIME;
    use nsh_platform_windows::windows::Win32::System::Time::SystemTimeToTzSpecificLocalTime;

    use super::LocalFields;

    /// UTC 时刻 → 系统当前时区下的本地时刻（含夏令时）
    pub fn local_fields(unix: u64) -> LocalFields {
        let utc = systemtime_from_unix(unix);
        let mut local = SYSTEMTIME::default();
        // SAFETY: `utc` 与 `local` 都是本函数里的有效栈对象；`None` 表示用系统当前时区。
        let converted =
            unsafe { SystemTimeToTzSpecificLocalTime(None, &utc, &mut local) }.is_ok();
        // 换算失败时退回 UTC，宁可差一个时区也不 panic
        let time = if converted { local } else { utc };
        LocalFields {
            year: i32::from(time.wYear),
            month: u32::from(time.wMonth),
            day: u32::from(time.wDay),
            hour: u32::from(time.wHour),
            minute: u32::from(time.wMinute),
            second: u32::from(time.wSecond),
            weekday: u32::from(time.wDayOfWeek),
        }
    }

    /// Unix 秒 → UTC 的 `SYSTEMTIME`
    fn systemtime_from_unix(unix: u64) -> SYSTEMTIME {
        let days = (unix / 86_400) as i64;
        let secs = (unix % 86_400) as u32;
        let (year, month, day) = super::civil_from_days(days);
        SYSTEMTIME {
            wYear: year as u16,
            wMonth: month as u16,
            wDayOfWeek: super::weekday_from_days(days) as u16,
            wDay: day as u16,
            wHour: (secs / 3600) as u16,
            wMinute: ((secs % 3600) / 60) as u16,
            wSecond: (secs % 60) as u16,
            wMilliseconds: 0,
        }
    }
}

#[cfg(unix)]
mod sys {
    use super::LocalFields;

    /// `localtime_r` 给出的本地时刻
    pub fn local_fields(unix: u64) -> LocalFields {
        // SAFETY: `localtime_r` 只写我们提供的 `tm`，不保留指针；失败时退回 UTC。
        let tm = unsafe {
            let time = unix as libc::time_t;
            let mut tm: libc::tm = std::mem::zeroed();
            if libc::localtime_r(&time, &mut tm).is_null() {
                None
            } else {
                Some(tm)
            }
        };
        tm.map_or_else(
            || super::utc_fields(unix),
            |tm| LocalFields {
                year: tm.tm_year + 1900,
                month: (tm.tm_mon + 1) as u32,
                day: tm.tm_mday as u32,
                hour: tm.tm_hour as u32,
                minute: tm.tm_min as u32,
                second: tm.tm_sec as u32,
                weekday: tm.tm_wday as u32,
            },
        )
    }
}

#[cfg(not(any(target_os = "windows", unix)))]
mod sys {
    use super::LocalFields;

    /// Web（wasm）等平台没有本地时区 API，按 UTC 处理
    pub fn local_fields(unix: u64) -> LocalFields {
        super::utc_fields(unix)
    }
}

/// 公历日期 → 自 1970-01-01 起的天数（Howard Hinnant 的 `days_from_civil`）
pub(crate) fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = i64::from(year) - i64::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400; // [0, 399]
    let month = i64::from(month);
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// 自 1970-01-01 起的天数 → 公历日期（Howard Hinnant 的 `civil_from_days`）
#[allow(dead_code)] // 仅在 Windows / UTC 回退路径里用到
pub(crate) fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let day = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let month = mp + if mp < 10 { 3 } else { -9 }; // [1, 12]
    (
        (year + i64::from(month <= 2)) as i32,
        month as u32,
        day as u32,
    )
}

/// 自 1970-01-01 起的天数 → 星期几（0 = 周日）
#[allow(dead_code)] // 仅在 Windows / UTC 回退路径里用到
fn weekday_from_days(days: i64) -> u32 {
    (days + 4).rem_euclid(7) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_dates_round_trip() {
        // 1970-01-01、闰年 2 月 29 日、2000-03-01、2100-12-31
        for (days, date) in [
            (0, (1970, 1, 1)),
            (19_782, (2024, 2, 29)),
            (11_017, (2000, 3, 1)),
            (47_846, (2100, 12, 31)),
        ] {
            assert_eq!(civil_from_days(days), date);
            assert_eq!(days_from_civil(date.0, date.1, date.2), days);
        }
    }

    #[test]
    fn weekday_matches_known_days() {
        assert_eq!(weekday_from_days(0), 4); // 1970-01-01 周四
        assert_eq!(weekday_from_days(19_782), 4); // 2024-02-29 周四
    }

    #[test]
    fn local_time_is_internally_consistent() {
        let now = at(1_700_000_000);
        assert!(now.hour < 24 && now.minute < 60 && now.second < 60);
        // 用本地日期与时刻重建的「本地时间戳」减去偏移就是原始 Unix 秒
        assert_eq!(
            now.day * 86_400 + i64::from(now.secs_of_day) - now.offset,
            now.unix as i64
        );
    }
}
