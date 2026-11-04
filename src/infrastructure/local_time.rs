//! Local wall-clock time for schedules and history timestamps.
//!
//! Windows reads the system local time. Other platforms fall back to UTC,
//! which keeps development builds working without a time-zone database.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalDateTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl LocalDateTime {
    pub fn minute_of_day(self) -> u32 {
        u32::from(self.hour) * 60 + u32::from(self.minute)
    }

    /// `YYYY-MM-DD HH:MM`, sortable and locale neutral.
    pub fn display(self) -> String {
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute
        )
    }
}

#[cfg(windows)]
pub fn now() -> LocalDateTime {
    use windows_sys::Win32::Foundation::SYSTEMTIME;
    use windows_sys::Win32::System::SystemInformation::GetLocalTime;

    let mut time = SYSTEMTIME {
        wYear: 0,
        wMonth: 0,
        wDayOfWeek: 0,
        wDay: 0,
        wHour: 0,
        wMinute: 0,
        wSecond: 0,
        wMilliseconds: 0,
    };
    // SAFETY: GetLocalTime only writes the provided SYSTEMTIME.
    unsafe { GetLocalTime(&mut time) };
    LocalDateTime {
        year: time.wYear,
        month: time.wMonth as u8,
        day: time.wDay as u8,
        hour: time.wHour as u8,
        minute: time.wMinute as u8,
        second: time.wSecond as u8,
    }
}

#[cfg(not(windows))]
pub fn now() -> LocalDateTime {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    from_unix_utc(seconds)
}

/// Civil UTC date-time for Unix seconds (Howard Hinnant's algorithm).
pub fn from_unix_utc(seconds: u64) -> LocalDateTime {
    let days = (seconds / 86_400) as i64;
    let second_of_day = seconds % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    LocalDateTime {
        year: year.clamp(0, i64::from(u16::MAX)) as u16,
        month: month as u8,
        day: day as u8,
        hour: (second_of_day / 3_600) as u8,
        minute: ((second_of_day % 3_600) / 60) as u8,
        second: (second_of_day % 60) as u8,
    }
}

/// Parse `H:MM` / `HH:MM` (24-hour) into minutes since midnight.
pub fn parse_hhmm(value: &str) -> Option<u32> {
    let (hour, minute) = value.trim().split_once(':')?;
    if hour.is_empty() || hour.len() > 2 || minute.len() != 2 {
        return None;
    }
    let hour: u32 = hour.parse().ok()?;
    let minute: u32 = minute.parse().ok()?;
    (hour < 24 && minute < 60).then_some(hour * 60 + minute)
}

pub fn normalize_hhmm(value: &str) -> Option<String> {
    parse_hhmm(value).map(format_hhmm)
}

pub fn format_hhmm(minute_of_day: u32) -> String {
    let minute_of_day = minute_of_day % (24 * 60);
    format!("{:02}:{:02}", minute_of_day / 60, minute_of_day % 60)
}

/// Whether `minute` lies in `[start, end)`. A window whose end is not after
/// its start spans midnight; an equal start and end covers the whole day.
pub fn minute_in_window(minute: u32, start: u32, end: u32) -> bool {
    if start == end {
        return true;
    }
    if start < end {
        (start..end).contains(&minute)
    } else {
        minute >= start || minute < end
    }
}

/// Seconds from `now` until the next `target` minute of day (1..=86400).
pub fn seconds_until(now: LocalDateTime, target_minute: u32) -> u64 {
    let now_seconds = u64::from(now.minute_of_day()) * 60 + u64::from(now.second);
    let target_seconds = u64::from(target_minute % (24 * 60)) * 60;
    let day = 24 * 60 * 60;
    let delta = (target_seconds + day - now_seconds % day) % day;
    if delta == 0 { day } else { delta }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_formats_clock_times() {
        assert_eq!(parse_hhmm("09:30"), Some(570));
        assert_eq!(parse_hhmm("9:05"), Some(545));
        assert_eq!(parse_hhmm(" 23:59 "), Some(1439));
        assert_eq!(parse_hhmm("24:00"), None);
        assert_eq!(parse_hhmm("12:5"), None);
        assert_eq!(parse_hhmm("noon"), None);
        assert_eq!(normalize_hhmm("7:00").as_deref(), Some("07:00"));
        assert_eq!(format_hhmm(1439), "23:59");
    }

    #[test]
    fn windows_handle_midnight() {
        assert!(minute_in_window(600, 540, 1080));
        assert!(!minute_in_window(1080, 540, 1080));
        assert!(minute_in_window(30, 1320, 360));
        assert!(minute_in_window(1400, 1320, 360));
        assert!(!minute_in_window(720, 1320, 360));
        assert!(minute_in_window(5, 100, 100));
    }

    #[test]
    fn current_local_time_is_plausible() {
        let now = now();
        assert!(now.year >= 2024, "{now:?}");
        assert!((1..=12).contains(&now.month) && (1..=31).contains(&now.day));
        assert!(now.hour < 24 && now.minute < 60 && now.second < 61);
        assert_eq!(now.display().len(), "YYYY-MM-DD HH:MM".len());
    }

    #[test]
    fn seconds_until_counts_from_current_second() {
        let now = from_unix_utc(9 * 3_600 + 59 * 60 + 30);
        assert_eq!(seconds_until(now, 600), 30);
        assert_eq!(seconds_until(now, 599), 86_370);
    }

    #[test]
    fn unix_utc_conversion() {
        assert_eq!(from_unix_utc(0).display(), "1970-01-01 00:00");
        assert_eq!(from_unix_utc(951_782_400).display(), "2000-02-29 00:00");
        assert_eq!(
            from_unix_utc(1_793_750_400 + 3_660).display(),
            "2026-11-04 01:01"
        );
    }
}
