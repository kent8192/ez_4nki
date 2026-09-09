use chrono::{DateTime, Timelike};

pub fn study_day(timestamp: i64, timezone: &str, start_hour: u32) -> Result<String> {
    if start_hour > 23 {
        return Err(invalid("学習日の開始時刻が不正です。"));
    }
    let timezone: chrono_tz::Tz = timezone
        .parse()
        .map_err(|_| invalid("タイムゾーンが不正です。"))?;
    let local = DateTime::from_timestamp(timestamp, 0)
        .ok_or_else(|| invalid("日時が不正です。"))?
        .with_timezone(&timezone);
    let date = if local.hour() < start_hour {
        local
            .date_naive()
            .pred_opt()
            .ok_or_else(|| invalid("日時が不正です。"))?
    } else {
        local.date_naive()
    };
    Ok(date.to_string())
}

mod model;
pub use model::*;
mod library;
pub use library::*;
mod learning;
mod storage;
pub use storage::*;
mod backup;
pub use backup::*;
mod analytics;
pub use analytics::*;
