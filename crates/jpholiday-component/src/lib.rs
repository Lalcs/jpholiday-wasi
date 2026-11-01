//! 日本の祝日を判定する WebAssembly Component。
//!
//! `wit/jpholiday.wit` の `lalcs:jpholiday/holidays` インターフェースを、
//! [`jpholiday`] クレートの関数 API に委譲して実装する。
//!
//! `cargo build --release --target wasm32-wasip2 -p jpholiday-component` で
//! `target/wasm32-wasip2/release/jpholiday_component.wasm` が生成される。

wit_bindgen::generate!({
    world: "jpholiday",
    path: "wit",
    additional_derives: [PartialEq, Eq],
});

use exports::lalcs::jpholiday::holidays::{Date, DateError, Guest, Holiday};

/// `lalcs:jpholiday/holidays` の実装。
struct Component;

impl Guest for Component {
    fn holidays(date: Date) -> Result<Vec<Holiday>, DateError> {
        Ok(jpholiday::holidays(to_date(date)?)
            .into_iter()
            .map(|h| to_holiday((h.date, h.name)))
            .collect())
    }

    fn is_holiday(date: Date) -> Result<bool, DateError> {
        Ok(jpholiday::is_holiday(to_date(date)?))
    }

    fn holiday_name(date: Date) -> Result<Option<String>, DateError> {
        Ok(jpholiday::is_holiday_name(to_date(date)?))
    }

    fn year_holidays(year: i32) -> Vec<Holiday> {
        to_holidays(jpholiday::year_holidays(year))
    }

    fn month_holidays(year: i32, month: u8) -> Vec<Holiday> {
        to_holidays(jpholiday::month_holidays(year, month.into()))
    }

    fn between(start: Date, end: Date) -> Result<Vec<Holiday>, DateError> {
        Ok(to_holidays(jpholiday::between(
            to_date(start)?,
            to_date(end)?,
        )))
    }
}

// エクスポート関数のシンボル名（`lalcs:jpholiday/holidays@0.1.0#is-holiday` など）は
// ネイティブのリンカでは扱えないため、エクスポートは wasm32 のときだけ生成する。
// ネイティブでは型と `Guest` 実装だけをビルドし、ユニットテストで検証する。
#[cfg(target_arch = "wasm32")]
export!(Component);

/// WIT の `date` を [`jpholiday::Date`] に変換する。実在しない日付はエラー。
fn to_date(date: Date) -> Result<jpholiday::Date, DateError> {
    jpholiday::Date::new(date.year, date.month.into(), date.day.into())
        .map_err(|_| DateError::InvalidDate(date))
}

/// [`jpholiday::Date`] を WIT の `date` に変換する。
fn from_date(date: jpholiday::Date) -> Date {
    // 実在する日付の月（1〜12）・日（1〜31）は必ず u8 に収まる。
    Date {
        year: date.year(),
        month: date.month() as u8,
        day: date.day() as u8,
    }
}

fn to_holiday((date, name): (jpholiday::Date, String)) -> Holiday {
    Holiday {
        date: from_date(date),
        name,
    }
}

fn to_holidays(holidays: Vec<(jpholiday::Date, String)>) -> Vec<Holiday> {
    holidays.into_iter().map(to_holiday).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u8, day: u8) -> Date {
        Date { year, month, day }
    }

    fn holiday(year: i32, month: u8, day: u8, name: &str) -> Holiday {
        Holiday {
            date: date(year, month, day),
            name: name.to_string(),
        }
    }

    #[test]
    fn converts_dates_both_ways() {
        let d = date(2024, 2, 29);
        assert_eq!(from_date(to_date(d).unwrap()), d);
    }

    #[test]
    fn rejects_invalid_dates() {
        for d in [
            date(2023, 2, 29),
            date(2024, 13, 1),
            date(2024, 0, 1),
            date(2024, 4, 31),
        ] {
            assert_eq!(to_date(d), Err(DateError::InvalidDate(d)));
            assert_eq!(Component::is_holiday(d), Err(DateError::InvalidDate(d)));
        }
        let ok = date(2024, 1, 1);
        let ng = date(2024, 2, 30);
        assert_eq!(Component::between(ok, ng), Err(DateError::InvalidDate(ng)));
        assert_eq!(Component::between(ng, ok), Err(DateError::InvalidDate(ng)));
    }

    #[test]
    fn delegates_to_jpholiday() {
        assert_eq!(Component::is_holiday(date(2017, 1, 2)), Ok(true));
        assert_eq!(Component::is_holiday(date(2017, 1, 3)), Ok(false));
        assert_eq!(
            Component::holiday_name(date(2017, 1, 2)),
            Ok(Some("元日 振替休日".to_string()))
        );
        assert_eq!(Component::holiday_name(date(2017, 1, 3)), Ok(None));
        assert_eq!(
            Component::holidays(date(2020, 1, 1)),
            Ok(vec![holiday(2020, 1, 1, "元日")])
        );
        assert_eq!(Component::holidays(date(2020, 1, 2)), Ok(vec![]));
        assert_eq!(Component::year_holidays(2017).len(), 17);
        assert_eq!(
            Component::month_holidays(2017, 5),
            vec![
                holiday(2017, 5, 3, "憲法記念日"),
                holiday(2017, 5, 4, "みどりの日"),
                holiday(2017, 5, 5, "こどもの日"),
            ]
        );
        assert_eq!(Component::month_holidays(2017, 13), vec![]);
        assert_eq!(
            Component::between(date(2017, 5, 4), date(2017, 5, 5)),
            Ok(vec![
                holiday(2017, 5, 4, "みどりの日"),
                holiday(2017, 5, 5, "こどもの日"),
            ])
        );
        assert_eq!(
            Component::between(date(2017, 5, 5), date(2017, 5, 4)),
            Ok(vec![])
        );
    }
}
