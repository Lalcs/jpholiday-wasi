//! コマンドライン引数の解釈と各コマンドの実行。
//!
//! 入出力（標準出力・標準エラー・現在時刻）から切り離した純粋な処理にしてあり、
//! [`run`] の結果を `main` が実際の入出力へ書き出す。

use crate::json;
use jpholiday::Date;

/// 成功（`is-holiday` / `name` では祝日だった）。
pub const EXIT_SUCCESS: u8 = 0;
/// `is-holiday` / `name` で祝日ではなかった。
pub const EXIT_NOT_HOLIDAY: u8 = 1;
/// 引数の誤りなどのエラー。
pub const EXIT_ERROR: u8 = 2;

/// コマンドの実行結果。
#[derive(Debug, PartialEq, Eq)]
pub struct Outcome {
    /// 標準出力へ書く内容。
    pub stdout: String,
    /// 標準エラーへ書く内容。
    pub stderr: String,
    /// 終了コード。
    pub code: u8,
}

impl Outcome {
    fn ok(stdout: String) -> Self {
        Self::with_code(stdout, EXIT_SUCCESS)
    }

    fn with_code(stdout: String, code: u8) -> Self {
        Outcome {
            stdout,
            stderr: String::new(),
            code,
        }
    }

    fn error(message: &str) -> Self {
        Outcome {
            stdout: String::new(),
            stderr: format!(
                "jpholiday: エラー: {message}\n詳しくは 'jpholiday --help' を参照してください。\n"
            ),
            code: EXIT_ERROR,
        }
    }
}

/// 解釈済みのコマンド。`None` の引数は「今日（JST）」で補う。
#[derive(Debug, PartialEq, Eq)]
enum Command {
    IsHoliday(Option<Date>),
    Name(Option<Date>),
    Year(Option<i32>),
    Month(Option<(i32, u32)>),
    Between(Date, Date),
}

/// 解釈済みの呼び出し全体。
#[derive(Debug, PartialEq, Eq)]
enum Invocation {
    Help,
    Version,
    Run { command: Command, json: bool },
}

/// バージョン表記（依存している jpholiday クレートのバージョンも併記する）。
pub fn version_text() -> String {
    format!(
        "jpholiday {} (jpholiday crate {})\n",
        env!("CARGO_PKG_VERSION"),
        env!("JPHOLIDAY_CRATE_VERSION")
    )
}

/// `--help` の表示内容。
pub fn help_text() -> String {
    format!(
        "\
jpholiday {version} - 日本の祝日を判定する CLI（内閣府の公表データに準拠）

使い方:
  jpholiday [オプション] <コマンド> [引数...]

コマンド:
  is-holiday [DATE]     祝日なら true、祝日でなければ false を出力
  name [DATE]           祝日名を出力（祝日でなければ何も出力しない）
  year [YEAR]           その年の祝日一覧を出力
  month [YEAR MONTH]    その月の祝日一覧を出力
  between START END     START〜END（両端を含む）の祝日一覧を出力
  help                  このヘルプを表示

  DATE / START / END は YYYY-MM-DD 形式で指定します。
  DATE・YEAR・YEAR MONTH を省略すると今日（日本時間）を使います。
  一覧は 1 行に 1 件、「日付<TAB>祝日名」の形式で出力します。

オプション:
      --json            JSON で出力
  -h, --help            このヘルプを表示
  -V, --version         バージョンを表示

終了コード:
  0  成功（is-holiday / name では祝日だった）
  1  is-holiday / name で祝日ではなかった
  2  引数の誤りなどのエラー
",
        version = env!("CARGO_PKG_VERSION")
    )
}

/// 引数（プログラム名を除く）を解釈してコマンドを実行する。
///
/// `today` は日付・年・月が省略されたときにだけ呼ばれ、今日の日付（JST）を返す。
pub fn run(args: &[String], today: impl FnOnce() -> Option<Date>) -> Outcome {
    match parse(args) {
        Ok(Invocation::Help) => Outcome::ok(help_text()),
        Ok(Invocation::Version) => Outcome::ok(version_text()),
        Ok(Invocation::Run { command, json }) => execute(command, json, today),
        Err(message) => Outcome::error(&message),
    }
}

fn parse(args: &[String]) -> Result<Invocation, String> {
    let mut json = false;
    let mut help = false;
    let mut version = false;
    let mut positional: Vec<&str> = Vec::new();
    let mut options_ended = false;

    for arg in args {
        let arg = arg.as_str();
        if options_ended || arg == "-" || !arg.starts_with('-') {
            positional.push(arg);
            continue;
        }
        match arg {
            "--" => options_ended = true,
            "--json" => json = true,
            "-h" | "--help" => help = true,
            "-V" | "--version" => version = true,
            _ => return Err(format!("不明なオプションです: {arg}")),
        }
    }

    if help {
        return Ok(Invocation::Help);
    }
    if version {
        return Ok(Invocation::Version);
    }

    let Some((&name, rest)) = positional.split_first() else {
        return Err("コマンドを指定してください".to_string());
    };
    let command = match (name, rest) {
        ("help", []) => return Ok(Invocation::Help),
        ("is-holiday", []) => Command::IsHoliday(None),
        ("is-holiday", [date]) => Command::IsHoliday(Some(parse_date(date)?)),
        ("name", []) => Command::Name(None),
        ("name", [date]) => Command::Name(Some(parse_date(date)?)),
        ("year", []) => Command::Year(None),
        ("year", [year]) => Command::Year(Some(parse_year(year)?)),
        ("month", []) => Command::Month(None),
        ("month", [year, month]) => Command::Month(Some((parse_year(year)?, parse_month(month)?))),
        ("month", [_]) => {
            return Err(
                "month には YEAR と MONTH の両方を指定するか、両方とも省略してください".to_string(),
            );
        }
        ("between", [start, end]) => {
            let (start, end) = (parse_date(start)?, parse_date(end)?);
            if start > end {
                return Err(format!(
                    "START（{start}）には END（{end}）以前の日付を指定してください"
                ));
            }
            Command::Between(start, end)
        }
        ("between", _) => return Err("between には START と END を指定してください".to_string()),
        ("help" | "is-holiday" | "name" | "year" | "month", _) => {
            return Err(format!("{name} の引数が多すぎます"));
        }
        _ => return Err(format!("不明なコマンドです: {name}")),
    };
    Ok(Invocation::Run { command, json })
}

/// `YYYY-MM-DD` 形式の日付を解釈する。
fn parse_date(s: &str) -> Result<Date, String> {
    let invalid_format = || format!("日付は YYYY-MM-DD 形式で指定してください: {s}");
    let mut parts = s.split('-');
    let (Some(y), Some(m), Some(d), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(invalid_format());
    };
    let is_digits = |p: &str, max_len: usize| {
        !p.is_empty() && p.len() <= max_len && p.bytes().all(|b| b.is_ascii_digit())
    };
    if !(is_digits(y, 9) && is_digits(m, 2) && is_digits(d, 2)) {
        return Err(invalid_format());
    }
    let (Ok(year), Ok(month), Ok(day)) = (y.parse(), m.parse(), d.parse()) else {
        return Err(invalid_format());
    };
    Date::new(year, month, day).map_err(|_| format!("存在しない日付です: {s}"))
}

fn parse_year(s: &str) -> Result<i32, String> {
    s.parse()
        .map_err(|_| format!("年は整数で指定してください: {s}"))
}

fn parse_month(s: &str) -> Result<u32, String> {
    match s.parse() {
        Ok(month @ 1..=12) => Ok(month),
        _ => Err(format!("月は 1〜12 の整数で指定してください: {s}")),
    }
}

fn execute(command: Command, json: bool, today: impl FnOnce() -> Option<Date>) -> Outcome {
    // `today` は引数が省略されたときだけ呼ぶ（時計へのアクセスを必要最小限にする）。
    match command {
        Command::IsHoliday(date) => {
            let date = match date.map_or_else(|| today_or_error(today), Ok) {
                Ok(date) => date,
                Err(outcome) => return outcome,
            };
            let holiday = jpholiday::is_holiday(date);
            let code = if holiday {
                EXIT_SUCCESS
            } else {
                EXIT_NOT_HOLIDAY
            };
            // JSON でも true / false はそのまま有効な値になる。
            Outcome::with_code(format!("{holiday}\n"), code)
        }
        Command::Name(date) => {
            let date = match date.map_or_else(|| today_or_error(today), Ok) {
                Ok(date) => date,
                Err(outcome) => return outcome,
            };
            let name = jpholiday::is_holiday_name(date);
            let code = if name.is_some() {
                EXIT_SUCCESS
            } else {
                EXIT_NOT_HOLIDAY
            };
            let stdout = match (name, json) {
                (Some(name), false) => format!("{name}\n"),
                (None, false) => String::new(),
                (Some(name), true) => {
                    let mut out = String::new();
                    json::write_string(&mut out, &name);
                    out.push('\n');
                    out
                }
                (None, true) => "null\n".to_string(),
            };
            Outcome::with_code(stdout, code)
        }
        Command::Year(year) => {
            let year = match year.map_or_else(|| today_or_error(today).map(Date::year), Ok) {
                Ok(year) => year,
                Err(outcome) => return outcome,
            };
            Outcome::ok(render_list(&jpholiday::year_holidays(year), json))
        }
        Command::Month(year_month) => {
            let (year, month) = match year_month.map_or_else(
                || today_or_error(today).map(|date| (date.year(), date.month())),
                Ok,
            ) {
                Ok(year_month) => year_month,
                Err(outcome) => return outcome,
            };
            Outcome::ok(render_list(&jpholiday::month_holidays(year, month), json))
        }
        Command::Between(start, end) => {
            Outcome::ok(render_list(&jpholiday::between(start, end), json))
        }
    }
}

/// 今日の日付を求める。取得できなければエラーの実行結果を返す。
fn today_or_error(today: impl FnOnce() -> Option<Date>) -> Result<Date, Outcome> {
    today().ok_or_else(|| Outcome::error("現在の日付を取得できませんでした"))
}

/// 祝日一覧を出力形式に整形する。
fn render_list(holidays: &[(Date, String)], json: bool) -> String {
    let mut out = String::new();
    if json {
        json::write_holidays(&mut out, holidays);
        out.push('\n');
    } else {
        for (date, name) in holidays {
            out.push_str(&format!("{date}\t{name}\n"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ymd(y: i32, m: u32, d: u32) -> Date {
        Date::new(y, m, d).unwrap()
    }

    fn run_with(args: &[&str], today: Option<Date>) -> Outcome {
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        run(&args, || today)
    }

    /// 今日が不要なはずの呼び出し（呼ばれたら失敗させる）。
    fn run_args(args: &[&str]) -> Outcome {
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        run(&args, || panic!("today() must not be called"))
    }

    fn assert_error(outcome: &Outcome, needle: &str) {
        assert_eq!(outcome.code, EXIT_ERROR, "{outcome:?}");
        assert!(outcome.stdout.is_empty(), "{outcome:?}");
        assert!(outcome.stderr.contains(needle), "{outcome:?}");
    }

    #[test]
    fn is_holiday_reports_with_exit_code() {
        let out = run_args(&["is-holiday", "2017-01-02"]);
        assert_eq!(out, Outcome::with_code("true\n".into(), EXIT_SUCCESS));

        let out = run_args(&["is-holiday", "2017-01-03"]);
        assert_eq!(out, Outcome::with_code("false\n".into(), EXIT_NOT_HOLIDAY));

        let out = run_args(&["--json", "is-holiday", "2017-01-03"]);
        assert_eq!(out, Outcome::with_code("false\n".into(), EXIT_NOT_HOLIDAY));
    }

    #[test]
    fn name_prints_holiday_name() {
        let out = run_args(&["name", "2017-01-02"]);
        assert_eq!(out, Outcome::ok("元日 振替休日\n".into()));

        let out = run_args(&["name", "2017-01-03"]);
        assert_eq!(out, Outcome::with_code(String::new(), EXIT_NOT_HOLIDAY));

        let out = run_args(&["name", "2017-01-02", "--json"]);
        assert_eq!(out, Outcome::ok("\"元日 振替休日\"\n".into()));

        let out = run_args(&["name", "--json", "2017-01-03"]);
        assert_eq!(out, Outcome::with_code("null\n".into(), EXIT_NOT_HOLIDAY));
    }

    #[test]
    fn year_lists_holidays() {
        let out = run_args(&["year", "2017"]);
        assert_eq!(out.code, EXIT_SUCCESS);
        let lines: Vec<&str> = out.stdout.lines().collect();
        assert_eq!(lines.len(), 17);
        assert_eq!(lines[0], "2017-01-01\t元日");
        assert_eq!(lines[1], "2017-01-02\t元日 振替休日");
        assert_eq!(lines[16], "2017-12-23\t天皇誕生日");
    }

    #[test]
    fn month_lists_holidays() {
        let out = run_args(&["month", "2017", "5"]);
        assert_eq!(
            out,
            Outcome::ok(
                "2017-05-03\t憲法記念日\n2017-05-04\tみどりの日\n2017-05-05\tこどもの日\n".into()
            )
        );

        // 祝日のない月は空の出力で成功扱い。
        let out = run_args(&["month", "2017", "6"]);
        assert_eq!(out, Outcome::ok(String::new()));

        let out = run_args(&["month", "2017", "6", "--json"]);
        assert_eq!(out, Outcome::ok("[]\n".into()));
    }

    #[test]
    fn between_lists_holidays() {
        let out = run_args(&["--json", "between", "2017-05-01", "2017-05-04"]);
        assert_eq!(
            out,
            Outcome::ok(
                "[{\"date\":\"2017-05-03\",\"name\":\"憲法記念日\"},{\"date\":\"2017-05-04\",\"name\":\"みどりの日\"}]\n".into()
            )
        );

        let out = run_args(&["between", "2017-05-04", "2017-05-04"]);
        assert_eq!(out, Outcome::ok("2017-05-04\tみどりの日\n".into()));
    }

    #[test]
    fn omitted_arguments_use_today() {
        let today = Some(ymd(2026, 1, 1));
        assert_eq!(
            run_with(&["is-holiday"], today),
            Outcome::ok("true\n".into())
        );
        assert_eq!(run_with(&["name"], today), Outcome::ok("元日\n".into()));

        let year = run_with(&["year"], today);
        assert!(year.stdout.starts_with("2026-01-01\t元日\n"), "{year:?}");

        let month = run_with(&["month"], today);
        assert_eq!(
            month,
            Outcome::ok("2026-01-01\t元日\n2026-01-12\t成人の日\n".into())
        );
    }

    #[test]
    fn fails_when_today_is_unavailable() {
        assert_error(&run_with(&["is-holiday"], None), "現在の日付");
        assert_error(&run_with(&["year"], None), "現在の日付");
    }

    #[test]
    fn help_and_version() {
        for args in [
            &["--help"][..],
            &["-h"],
            &["help"],
            &["year", "2017", "--help"],
        ] {
            let out = run_args(args);
            assert_eq!(out.code, EXIT_SUCCESS);
            assert!(out.stdout.contains("使い方:"), "{args:?}");
        }
        for args in [&["--version"][..], &["-V"]] {
            let out = run_args(args);
            assert_eq!(out.code, EXIT_SUCCESS);
            assert!(out.stdout.starts_with("jpholiday "), "{out:?}");
            assert!(out.stdout.contains("(jpholiday crate "), "{out:?}");
        }
    }

    #[test]
    fn rejects_invalid_usage() {
        assert_error(&run_args(&[]), "コマンドを指定");
        assert_error(&run_args(&["--json"]), "コマンドを指定");
        assert_error(&run_args(&["holiday"]), "不明なコマンド");
        assert_error(&run_args(&["--yaml", "year"]), "不明なオプション");
        assert_error(
            &run_args(&["is-holiday", "2017-01-01", "2017-01-02"]),
            "多すぎ",
        );
        assert_error(&run_args(&["year", "2017", "2018"]), "多すぎ");
        assert_error(&run_args(&["help", "year"]), "多すぎ");
        assert_error(&run_args(&["month", "2017"]), "YEAR と MONTH");
        assert_error(&run_args(&["between", "2017-01-01"]), "START と END");
    }

    #[test]
    fn rejects_invalid_values() {
        for date in [
            "2017/01/01",
            "20170101",
            "2017-1",
            "2017-01-01-01",
            "2017-01-1x",
            "-2017-01-01x",
            "2017--01",
            "2017-001-01",
            "+2017-01-01",
        ] {
            assert_error(&run_args(&["is-holiday", "--", date]), "YYYY-MM-DD");
        }
        assert_error(&run_args(&["is-holiday", "2017-02-29"]), "存在しない日付");
        assert_error(&run_args(&["name", "2017-13-01"]), "存在しない日付");
        assert_error(&run_args(&["year", "twenty"]), "年は整数");
        assert_error(&run_args(&["month", "2017", "13"]), "月は 1〜12");
        assert_error(&run_args(&["month", "2017", "0"]), "月は 1〜12");
        assert_error(
            &run_args(&["between", "2017-05-05", "2017-05-01"]),
            "以前の日付",
        );
    }

    #[test]
    fn accepts_single_digit_month_and_day() {
        assert_eq!(
            run_args(&["name", "2017-1-2"]),
            Outcome::ok("元日 振替休日\n".into())
        );
    }

    #[test]
    fn double_dash_ends_options() {
        // `--` の後ろはオプションとして扱わない（負の年を渡せる）。
        let out = run_args(&["year", "--", "-1"]);
        assert_eq!(out, Outcome::ok(String::new()));
        assert_error(&run_args(&["--", "--json"]), "不明なコマンド");
    }
}
