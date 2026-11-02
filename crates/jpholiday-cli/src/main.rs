//! `jpholiday` コマンド: 日本の祝日を判定する CLI。
//!
//! WASI（`wasm32-wasip1`）とネイティブの両方で、同じソースからビルドする。
//! 使い方は `jpholiday --help` を参照。

#![forbid(unsafe_code)]

mod app;
mod clock;
mod json;

use std::io::{self, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let outcome = app::run(&args, clock::today_jst);

    // パイプの読み手が先に終了した場合（`| head` など）も panic せず静かに終える。
    if write_all(&mut io::stdout().lock(), &outcome.stdout).is_err() {
        return ExitCode::from(app::EXIT_ERROR);
    }
    let _ = write_all(&mut io::stderr().lock(), &outcome.stderr);
    ExitCode::from(outcome.code)
}

fn write_all(out: &mut impl Write, s: &str) -> io::Result<()> {
    if s.is_empty() {
        return Ok(());
    }
    out.write_all(s.as_bytes())?;
    out.flush()
}
