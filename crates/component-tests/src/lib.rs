//! ビルド済みの jpholiday Component（`wasm32-wasip2`）を wasmtime 上で呼び出す結合テスト。
//!
//! テスト本体は `tests/component.rs` にある。先に Component をビルドしてから実行する:
//!
//! ```sh
//! cargo build --release --target wasm32-wasip2 -p jpholiday-component
//! cargo test -p component-tests
//! ```
//!
//! 別の場所の .wasm を検証する場合は `JPHOLIDAY_COMPONENT_WASM` にパスを指定する。
