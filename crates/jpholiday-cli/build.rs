//! 依存している `jpholiday` クレートのバージョンを `--version` で表示するため、
//! ワークスペースの Cargo.lock から読み取って `JPHOLIDAY_CRATE_VERSION` として埋め込む。

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest_dir =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let lock = manifest_dir.join("../../Cargo.lock");
    println!("cargo::rerun-if-changed={}", lock.display());

    let version = fs::read_to_string(&lock)
        .ok()
        .and_then(|content| locked_version(&content, "jpholiday"))
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo::rustc-env=JPHOLIDAY_CRATE_VERSION={version}");
}

/// Cargo.lock の `[[package]]` から `name` のバージョンを探す。
///
/// Cargo.lock は `name = "..."` の直後に `version = "..."` が来る形式で生成される。
fn locked_version(lock: &str, name: &str) -> Option<String> {
    let name_line = format!("name = \"{name}\"");
    let mut lines = lock.lines();
    while let Some(line) = lines.next() {
        if line.trim() == name_line {
            return lines
                .next()?
                .trim()
                .strip_prefix("version = \"")?
                .strip_suffix('"')
                .map(str::to_owned);
        }
    }
    None
}
