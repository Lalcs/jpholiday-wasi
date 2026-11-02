//! 外部クレートに頼らない最小限の JSON 出力。

use jpholiday::Date;
use std::fmt::Write;

/// 文字列を JSON の文字列リテラル（前後の `"` を含む）として `out` に書き込む。
pub fn write_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if u32::from(c) < 0x20 => {
                // 書き込み先が String なので失敗しない。
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// `(日付, 名称)` の一覧を `[{"date":"YYYY-MM-DD","name":"..."}]` 形式で書き込む。
pub fn write_holidays(out: &mut String, holidays: &[(Date, String)]) {
    out.push('[');
    for (i, (date, name)) in holidays.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str("{\"date\":");
        write_string(out, &date.to_string());
        out.push_str(",\"name\":");
        write_string(out, name);
        out.push('}');
    }
    out.push(']');
}

#[cfg(test)]
mod tests {
    use super::*;

    fn string(s: &str) -> String {
        let mut out = String::new();
        write_string(&mut out, s);
        out
    }

    #[test]
    fn escapes_special_characters() {
        assert_eq!(string("元日"), "\"元日\"");
        assert_eq!(string("a\"b\\c"), "\"a\\\"b\\\\c\"");
        assert_eq!(string("\n\r\t"), "\"\\n\\r\\t\"");
        assert_eq!(string("\u{0}\u{1f}"), "\"\\u0000\\u001f\"");
    }

    #[test]
    fn writes_holiday_list() {
        let mut out = String::new();
        write_holidays(&mut out, &[]);
        assert_eq!(out, "[]");

        let mut out = String::new();
        write_holidays(
            &mut out,
            &[
                (Date::new(2026, 1, 1).unwrap(), "元日".to_string()),
                (Date::new(2026, 1, 12).unwrap(), "成人の日".to_string()),
            ],
        );
        assert_eq!(
            out,
            r#"[{"date":"2026-01-01","name":"元日"},{"date":"2026-01-12","name":"成人の日"}]"#
        );
    }
}
