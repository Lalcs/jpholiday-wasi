# jpholiday-wasi

[![CI](https://github.com/Lalcs/jpholiday-wasi/actions/workflows/ci.yml/badge.svg)](https://github.com/Lalcs/jpholiday-wasi/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](./LICENSE)
[![Rust](https://img.shields.io/badge/Rust-jpholiday-DEA584?logo=rust&logoColor=white)](https://github.com/Lalcs/jpholiday-rust)
[![Python](https://img.shields.io/badge/Python-jpholiday-3776AB?logo=python&logoColor=white)](https://github.com/Lalcs/jpholiday)

日本の祝日を判定する Rust ライブラリ [jpholiday](https://github.com/Lalcs/jpholiday-rust) を、
WebAssembly（WASI）とネイティブの実行ファイルとして配布するプロジェクトです。

[内閣府](https://www8.cao.go.jp/chosei/shukujitsu/gaiyou.html)が公表しているデータを基に、
振替休日・国民の休日・春分/秋分の日まで判定します。
**2027年**までの祝日は公式発表された内容に基づいて動作確認済みです。
それ以降についても取得は可能ですが、内閣府からの正式な公表がないため、正確性は保証されません。

| 成果物 | 形式 | 用途 |
|---|---|---|
| CLI（WASI） | `wasm32-wasip1` のコアモジュール | wasmtime などの WASI ランタイムでコマンドとして実行 |
| Component | `wasm32-wasip2` の WebAssembly Component | JavaScript・Python・Rust などから関数として呼び出し |
| CLI（ネイティブ） | Linux / macOS / Windows の実行ファイル | ランタイムなしでコマンドとして実行 |

## ダウンロード

[Releases](https://github.com/Lalcs/jpholiday-wasi/releases) から取得してください。

| ファイル | 内容 |
|---|---|
| `jpholiday-vX.Y.Z-wasm32-wasip1.wasm` | CLI（WASI Preview 1） |
| `jpholiday-component-vX.Y.Z-wasm32-wasip2.wasm` | Component（WASI Preview 2） |
| `jpholiday-vX.Y.Z-x86_64-unknown-linux-musl.tar.gz` | CLI（Linux x86_64、静的リンク） |
| `jpholiday-vX.Y.Z-aarch64-unknown-linux-musl.tar.gz` | CLI（Linux arm64、静的リンク） |
| `jpholiday-vX.Y.Z-x86_64-apple-darwin.tar.gz` | CLI（macOS Intel） |
| `jpholiday-vX.Y.Z-aarch64-apple-darwin.tar.gz` | CLI（macOS Apple Silicon） |
| `jpholiday-vX.Y.Z-x86_64-pc-windows-msvc.zip` | CLI（Windows x86_64） |
| `SHA256SUMS` | 上記すべての SHA-256 チェックサム |

ダウンロードしたファイルは `SHA256SUMS` で検証できます。

```bash
sha256sum --check --ignore-missing SHA256SUMS
```

## CLI

### 実行方法

WASI 版は [wasmtime](https://wasmtime.dev/) で実行できます。ファイルシステムやネットワークへのアクセスは不要です。

```bash
wasmtime run jpholiday-v0.1.0-wasm32-wasip1.wasm year 2026
```

ネイティブ版は展開した `jpholiday`（Windows は `jpholiday.exe`）をそのまま実行します。

```bash
jpholiday year 2026
```

WASI 版を毎回 `wasmtime run …` と書くのが手間なら、エイリアスを作ると便利です。

```bash
alias jpholiday='wasmtime run /path/to/jpholiday-v0.1.0-wasm32-wasip1.wasm'
```

### コマンド

| コマンド | 説明 |
|---|---|
| `jpholiday is-holiday [DATE]` | 祝日なら `true`、祝日でなければ `false` を出力 |
| `jpholiday name [DATE]` | 祝日名を出力（祝日でなければ何も出力しない） |
| `jpholiday year [YEAR]` | その年の祝日一覧を出力 |
| `jpholiday month [YEAR MONTH]` | その月の祝日一覧を出力 |
| `jpholiday between START END` | START〜END（両端を含む）の祝日一覧を出力 |

- `DATE` / `START` / `END` は `YYYY-MM-DD` 形式で指定します。
- `DATE`・`YEAR`・`YEAR MONTH` を省略すると**今日（日本時間）**を使います。
  WASI ではタイムゾーンを取得できないため UTC+9 固定で計算します（日本に夏時間はないため正確です）。
- オプション: `--json`（JSON で出力）、`-h` / `--help`、`-V` / `--version`

### 出力例

```console
$ jpholiday is-holiday 2026-01-01
true

$ jpholiday name 2026-05-06
憲法記念日 振替休日

$ jpholiday month 2026 9
2026-09-21	敬老の日
2026-09-22	国民の休日
2026-09-23	秋分の日

$ jpholiday --json month 2026 1
[{"date":"2026-01-01","name":"元日"},{"date":"2026-01-12","name":"成人の日"}]

$ jpholiday --json name 2026-01-02
null
```

一覧は 1 行に 1 件、「日付<TAB>祝日名」の形式です。`--json` を付けると、一覧は
`[{"date": "YYYY-MM-DD", "name": "祝日名"}]` の配列、`is-holiday` は `true` / `false`、
`name` は文字列または `null` を出力します。

### 終了コード

| コード | 意味 |
|---|---|
| `0` | 成功（`is-holiday` / `name` では祝日だった） |
| `1` | `is-holiday` / `name` で祝日ではなかった |
| `2` | 引数の誤りなどのエラー |

`is-holiday` は終了コードでも結果を返すため、シェルの条件分岐にそのまま使えます。

```bash
if jpholiday is-holiday > /dev/null; then
  echo "今日は祝日です"
fi
```

> [!NOTE]
> `set -e` のスクリプトで `is-holiday` / `name` を条件分岐の外で実行すると、祝日でない日に終了コード 1 で
> スクリプトが止まります。`if` の条件にするか、`|| true` を付けてください。

## Component

Component は [WIT](./crates/jpholiday-component/wit/jpholiday.wit) で定義した
`lalcs:jpholiday/holidays` インターフェースを公開します。

```wit
interface holidays {
    record date { year: s32, month: u8, day: u8 }
    record holiday { date: date, name: string }
    variant date-error { invalid-date(date) }

    holidays: func(date: date) -> result<list<holiday>, date-error>;
    is-holiday: func(date: date) -> result<bool, date-error>;
    holiday-name: func(date: date) -> result<option<string>, date-error>;
    year-holidays: func(year: s32) -> list<holiday>;
    month-holidays: func(year: s32, month: u8) -> list<holiday>;
    between: func(start: date, end: date) -> result<list<holiday>, date-error>;
}
```

- 実在しない日付（例: 2月30日）は `date-error` の `invalid-date` を返します。
- `month-holidays` は `month` が 1〜12 の範囲外なら空、`between` は `start > end` なら空を返します（jpholiday クレートと同じ挙動）。
- 独自の休日を追加する機能（jpholiday クレートの `register` / `unregister`）は、Rust のトレイト実装が前提のため Component では提供していません。
- WIT のパッケージバージョンはリリースのバージョンと同じです（v0.1.0 なら `lalcs:jpholiday@0.1.0`）。
  WIT から生成したホストは SemVer 互換のバージョンの Component を読み込めます。
  たとえば 0.1.0 の WIT で生成した wasmtime のホストは 0.1.1 を読み込めますが、0.2.0 は読み込めません。

### JavaScript（jco）

[jco](https://github.com/bytecodealliance/jco) で JavaScript に変換して使います。

```bash
npm install @bytecodealliance/preview2-shim
npx @bytecodealliance/jco transpile jpholiday-component-v0.1.0-wasm32-wasip2.wasm -o jpholiday --name jpholiday
```

```js
import { holidays } from './jpholiday/jpholiday.js';

holidays.isHoliday({ year: 2026, month: 1, day: 1 });
// => true
holidays.holidayName({ year: 2026, month: 5, day: 7 });
// => undefined
holidays.monthHolidays(2026, 9);
// => [
//   { date: { year: 2026, month: 9, day: 21 }, name: '敬老の日' },
//   { date: { year: 2026, month: 9, day: 22 }, name: '国民の休日' },
//   { date: { year: 2026, month: 9, day: 23 }, name: '秋分の日' },
// ]

try {
  holidays.isHoliday({ year: 2026, month: 2, day: 30 });
} catch (e) {
  // ComponentError: e.payload => { tag: 'invalid-date', val: { year: 2026, month: 2, day: 30 } }
}
```

### Rust（wasmtime）

[wasmtime](https://docs.rs/wasmtime) と [wasmtime-wasi](https://docs.rs/wasmtime-wasi) でホストから呼び出せます。
動作する完全な例は [crates/component-tests/tests/component.rs](./crates/component-tests/tests/component.rs) を参照してください。

```rust
wasmtime::component::bindgen!({ world: "jpholiday", path: "wit" });
use exports::lalcs::jpholiday::holidays::Date;

// engine / linker（wasmtime_wasi::p2::add_to_linker_sync 済み）/ store を用意して:
let component = Component::from_file(&engine, "jpholiday-component-v0.1.0-wasm32-wasip2.wasm")?;
let bindings = Jpholiday::instantiate(&mut store, &component, &linker)?;
let holidays = bindings.lalcs_jpholiday_holidays();

holidays.call_is_holiday(&mut store, Date { year: 2026, month: 1, day: 1 })?;
// => Ok(true)
```

## ソースからビルド

Rust 1.96 以降が必要です。

```bash
# CLI（WASI）→ target/wasm32-wasip1/release/jpholiday.wasm
rustup target add wasm32-wasip1
cargo build --release -p jpholiday-cli --target wasm32-wasip1

# Component → target/wasm32-wasip2/release/jpholiday_component.wasm
rustup target add wasm32-wasip2
cargo build --release -p jpholiday-component --target wasm32-wasip2

# CLI（ネイティブ）→ target/release/jpholiday
cargo build --release -p jpholiday-cli
```

### テスト

```bash
# ユニットテスト（CLI・Component の型変換）
cargo test

# Component を wasmtime 上で呼び出す結合テスト（先に Component をビルドしておく）
cargo test -p component-tests

# CLI のスモークテスト（ネイティブ版・WASI 版のどちらにも使える）
scripts/smoke-test.sh target/release/jpholiday
scripts/smoke-test.sh wasmtime run target/wasm32-wasip1/release/jpholiday.wasm
```

## リリース手順

`v` で始まる SemVer のタグを push すると、[Release ワークフロー](./.github/workflows/release.yml)
が全ターゲットをビルド・テストし、GitHub Release を作成して成果物と `SHA256SUMS` を添付します。

```bash
git tag v0.1.0
git push origin v0.1.0
```

- Cargo.toml の `version` と WIT のパッケージバージョン（`lalcs:jpholiday@X.Y.Z`）は、
  どちらもタグの値で上書きされるため、手動で更新する必要はありません。
  リポジトリ内の両者が一致していることは `cargo test` で検証しています。
- `v1.0.0-rc.1` のようにプレリリース識別子を含むタグは、プレリリースとして公開されます。
- WIT のバージョンが変わると Component のインターフェース名（`lalcs:jpholiday/holidays@X.Y.Z`）も変わります。
  0.x の間はマイナーバージョンを上げると互換性のない変更として扱われるため、
  インターフェースを壊さない変更はパッチバージョン（例: v0.1.0 → v0.1.1）でリリースしてください。
- 祝日データは依存している [jpholiday クレート](https://crates.io/crates/jpholiday) に由来します。
  本体が更新されたら `cargo update -p jpholiday` で取り込んでからリリースしてください。
  `jpholiday --version` で、組み込まれている jpholiday クレートのバージョンを確認できます。

## License

MIT License. 詳細は [LICENSE](./LICENSE) を参照してください。
