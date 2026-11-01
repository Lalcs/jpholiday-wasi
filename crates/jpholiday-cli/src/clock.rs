//! 「今日（日本時間）」の算出。
//!
//! WASI からはタイムゾーン情報を取得できないため、UTC+9 固定で計算する。日本には夏時間がないため、
//! 固定オフセットで日本標準時（JST）と一致する。

use jpholiday::Date;
use std::time::{SystemTime, UNIX_EPOCH};

/// JST の UTC からのオフセット（秒）。
const JST_OFFSET_SECS: i64 = 9 * 60 * 60;

/// 1 日の秒数。
const SECS_PER_DAY: i64 = 24 * 60 * 60;

/// 1970-01-01 の序数（[`Date::to_ordinal`] は 0001-01-01 を 1 とする）。
const UNIX_EPOCH_ORDINAL: i64 = 719_163;

/// 現在時刻から今日の日付（JST）を返す。
pub fn today_jst() -> Option<Date> {
    let secs = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(after) => i64::try_from(after.as_secs()).ok()?,
        Err(before) => i64::try_from(before.duration().as_secs())
            .ok()?
            .checked_neg()?,
    };
    date_from_unix_secs_jst(secs)
}

/// UNIX 時刻（秒）を JST の日付に変換する。
pub fn date_from_unix_secs_jst(secs: i64) -> Option<Date> {
    let days = secs.checked_add(JST_OFFSET_SECS)?.div_euclid(SECS_PER_DAY);
    Date::checked_from_ordinal(UNIX_EPOCH_ORDINAL.checked_add(days)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ymd(y: i32, m: u32, d: u32) -> Option<Date> {
        Some(Date::new(y, m, d).unwrap())
    }

    #[test]
    fn epoch_ordinal_matches() {
        assert_eq!(
            Date::new(1970, 1, 1).unwrap().to_ordinal(),
            UNIX_EPOCH_ORDINAL
        );
    }

    #[test]
    fn converts_with_jst_offset() {
        // 1970-01-01T00:00:00Z は JST で 09:00。
        assert_eq!(date_from_unix_secs_jst(0), ymd(1970, 1, 1));
        // 2025-12-31T14:59:59Z = 2025-12-31T23:59:59+09:00
        assert_eq!(date_from_unix_secs_jst(1_767_193_199), ymd(2025, 12, 31));
        // 2025-12-31T15:00:00Z = 2026-01-01T00:00:00+09:00
        assert_eq!(date_from_unix_secs_jst(1_767_193_200), ymd(2026, 1, 1));
    }

    #[test]
    fn handles_times_before_epoch() {
        // 1969-12-31T14:59:59Z = 1969-12-31T23:59:59+09:00
        assert_eq!(date_from_unix_secs_jst(-32_401), ymd(1969, 12, 31));
        // 1969-12-31T15:00:00Z = 1970-01-01T00:00:00+09:00
        assert_eq!(date_from_unix_secs_jst(-32_400), ymd(1970, 1, 1));
    }

    #[test]
    fn rejects_overflow() {
        assert_eq!(date_from_unix_secs_jst(i64::MAX), None);
    }
}
