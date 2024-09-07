use std::time::{SystemTime, SystemTimeError};

use chrono::prelude::{DateTime, Local, TimeZone};

/// 时间格式化类型
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeFormat {
    /// 年-月-日, YYYY-MM-DD
    YYYYMMDD,
    HHMM,
    /// 时-分-秒, HH:MM:SS
    HHMMSS,
    /// 年-月-日 时:分, YYYY-MM-DD HH:MM
    YYYYMMDD_HHMM,
    /// 年-月-日 时:分:秒, YYYY-MM-DD HH:MM:SS
    YYYYMMDD_HHMMSS,
}
pub enum DurationType {
    Year(usize),
    Month(usize),
    Day(usize),
}
/// 时间格式化
#[derive(Debug, Default, Clone)]
pub struct TIME {
    naos: u128,
    year: usize,
    month: usize,
    day: usize,
    hour: usize,
    minute: usize,
    second: usize,
}
impl From<DateTime<Local>> for TIME {
    fn from(value: DateTime<Local>) -> Self {
        let naos = value.timestamp_nanos_opt().unwrap() as u128;
        let (year, month, day, hour, minute, second) = parse_time(naos as i64);
        Self {
            naos,
            year,
            month,
            day,
            hour,
            minute,
            second,
        }
    }
}
impl TIME {
    /// 获取当前的时间
    pub fn now() -> Result<Self, SystemTimeError> {
        let time = SystemTime::now();
        let naos = time.duration_since(SystemTime::UNIX_EPOCH)?.as_nanos();
        // let local = chrono::Local.timestamp_nanos(naos as i64);

        let (year, month, day, hour, minute, second) = parse_time(naos as i64);
        Ok(Self {
            naos,
            year,
            month,
            day,
            hour,
            minute,
            second,
        })
    }
    pub fn format(&self, fmt: TimeFormat) -> String {
        match fmt {
            TimeFormat::HHMMSS => {
                format!("{:0>2}:{:0>2}:{:0>2}", self.hour, self.minute, self.second)
            }
            TimeFormat::YYYYMMDD => {
                format!("{:0>4}-{:0>2}-{:0>2}", self.year, self.month, self.day)
            }
            TimeFormat::YYYYMMDD_HHMM => format!(
                "{:0>4}-{:0>2}-{:0>2} {:0>2}:{:0>2}",
                self.year, self.month, self.day, self.hour, self.minute
            ),
            TimeFormat::YYYYMMDD_HHMMSS => format!(
                "{:0>4}-{:0>2}-{:0>2} {:0>2}:{:0>2}:{:0>2}",
                self.year, self.month, self.day, self.hour, self.minute, self.second
            ),
            TimeFormat::HHMM => {
                format!("{:0>2}:{:0>2}", self.hour, self.minute)
            }
        }
    }
    pub fn naos(&self) -> u128 {
        self.naos
    }
    pub fn year(&self) -> usize {
        self.year
    }
    pub fn month(&self) -> usize {
        self.month
    }
    pub fn day(&self) -> usize {
        self.day
    }
    pub fn hour(&self) -> usize {
        self.hour
    }
    pub fn minute(&self) -> usize {
        self.minute
    }
    pub fn second(&self) -> usize {
        self.second
    }
    
}
fn parse_time(naos: i64) -> (usize, usize, usize, usize, usize, usize) {
    let local = chrono::Local.timestamp_nanos(naos);
    let time = local.to_rfc3339();
    let time_str = time.as_str();
    (
        time_str[0..4].parse().unwrap(),
        time_str[5..7].parse().unwrap(),
        time_str[8..10].parse().unwrap(),
        time_str[11..13].parse().unwrap(),
        time_str[14..16].parse().unwrap(),
        time_str[17..19].parse().unwrap(),
    )
}
// 1996-12-19T16:39:57
