//! ビルド済みの jpholiday Component を wasmtime 上でインスタンス化し、WIT の各関数を呼び出す。
//!
//! 期待値は同じ版の `jpholiday` クレートをネイティブで呼んだ結果と突き合わせる。

use std::path::PathBuf;
use std::sync::OnceLock;

use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::{Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

wasmtime::component::bindgen!({
    world: "jpholiday",
    path: "../jpholiday-component/wit",
    additional_derives: [PartialEq, Eq],
});

use exports::lalcs::jpholiday::holidays::{Date, DateError, Guest, Holiday};

struct State {
    ctx: WasiCtx,
    table: ResourceTable,
}

impl WasiView for State {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.ctx,
            table: &mut self.table,
        }
    }
}

/// 検証対象の .wasm のパス。`JPHOLIDAY_COMPONENT_WASM` で上書きできる。
fn component_path() -> PathBuf {
    match std::env::var_os("JPHOLIDAY_COMPONENT_WASM") {
        Some(path) => PathBuf::from(path),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/wasm32-wasip2/release/jpholiday_component.wasm"),
    }
}

/// コンパイル済みの Component（テスト間で共有する）。
fn compiled() -> &'static (Engine, Component) {
    static COMPILED: OnceLock<(Engine, Component)> = OnceLock::new();
    COMPILED.get_or_init(|| {
        let path = component_path();
        assert!(
            path.exists(),
            "Component が見つかりません: {}\n\
             先に `cargo build --release --target wasm32-wasip2 -p jpholiday-component` を実行するか、\
             JPHOLIDAY_COMPONENT_WASM にパスを指定してください。",
            path.display()
        );
        let engine = Engine::default();
        let component = Component::from_file(&engine, &path).expect("Component を読み込めません");
        (engine, component)
    })
}

/// Component をインスタンス化し、`f` にエクスポートとストアを渡す。
fn with_instance<R>(f: impl FnOnce(&Guest, &mut Store<State>) -> R) -> R {
    let (engine, component) = compiled();
    let mut linker = Linker::<State>::new(engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker).expect("WASI をリンクできません");
    let state = State {
        ctx: WasiCtx::builder().build(),
        table: ResourceTable::new(),
    };
    let mut store = Store::new(engine, state);
    let bindings =
        Jpholiday::instantiate(&mut store, component, &linker).expect("インスタンス化できません");
    f(bindings.lalcs_jpholiday_holidays(), &mut store)
}

fn date(year: i32, month: u8, day: u8) -> Date {
    Date { year, month, day }
}

fn holiday(year: i32, month: u8, day: u8, name: &str) -> Holiday {
    Holiday {
        date: date(year, month, day),
        name: name.to_string(),
    }
}

/// ネイティブの jpholiday の結果を WIT の型に変換する（期待値用）。
fn expected(holidays: Vec<(jpholiday::Date, String)>) -> Vec<Holiday> {
    holidays
        .into_iter()
        .map(|(d, name)| Holiday {
            date: date(d.year(), d.month() as u8, d.day() as u8),
            name,
        })
        .collect()
}

#[test]
fn is_holiday() {
    with_instance(|h, store| {
        assert_eq!(
            h.call_is_holiday(&mut *store, date(2017, 1, 1)).unwrap(),
            Ok(true)
        );
        assert_eq!(
            h.call_is_holiday(&mut *store, date(2017, 1, 2)).unwrap(),
            Ok(true)
        );
        assert_eq!(
            h.call_is_holiday(&mut *store, date(2017, 1, 3)).unwrap(),
            Ok(false)
        );
    });
}

#[test]
fn holiday_name() {
    with_instance(|h, store| {
        assert_eq!(
            h.call_holiday_name(&mut *store, date(2017, 1, 2)).unwrap(),
            Ok(Some("元日 振替休日".to_string()))
        );
        assert_eq!(
            h.call_holiday_name(&mut *store, date(2017, 1, 3)).unwrap(),
            Ok(None)
        );
    });
}

#[test]
fn holidays() {
    with_instance(|h, store| {
        assert_eq!(
            h.call_holidays(&mut *store, date(2019, 5, 1)).unwrap(),
            Ok(vec![holiday(2019, 5, 1, "天皇の即位の日")])
        );
        assert_eq!(
            h.call_holidays(&mut *store, date(2019, 5, 7)).unwrap(),
            Ok(vec![])
        );
    });
}

#[test]
fn invalid_dates_are_errors() {
    with_instance(|h, store| {
        let ng = date(2023, 2, 29);
        let ok = date(2023, 1, 1);
        let err = Err(DateError::InvalidDate(ng));
        assert_eq!(h.call_is_holiday(&mut *store, ng).unwrap(), err);
        assert_eq!(
            h.call_holiday_name(&mut *store, ng).unwrap(),
            Err(DateError::InvalidDate(ng))
        );
        assert_eq!(
            h.call_holidays(&mut *store, ng).unwrap(),
            Err(DateError::InvalidDate(ng))
        );
        assert_eq!(
            h.call_between(&mut *store, ok, ng).unwrap(),
            Err(DateError::InvalidDate(ng))
        );
        assert_eq!(
            h.call_is_holiday(&mut *store, date(2023, 13, 1)).unwrap(),
            Err(DateError::InvalidDate(date(2023, 13, 1)))
        );
    });
}

#[test]
fn month_holidays() {
    with_instance(|h, store| {
        assert_eq!(
            h.call_month_holidays(&mut *store, 2017, 5).unwrap(),
            vec![
                holiday(2017, 5, 3, "憲法記念日"),
                holiday(2017, 5, 4, "みどりの日"),
                holiday(2017, 5, 5, "こどもの日"),
            ]
        );
        assert_eq!(h.call_month_holidays(&mut *store, 2017, 0).unwrap(), vec![]);
        assert_eq!(
            h.call_month_holidays(&mut *store, 2017, 13).unwrap(),
            vec![]
        );
    });
}

#[test]
fn between() {
    with_instance(|h, store| {
        assert_eq!(
            h.call_between(&mut *store, date(2017, 1, 1), date(2017, 1, 9))
                .unwrap(),
            Ok(vec![
                holiday(2017, 1, 1, "元日"),
                holiday(2017, 1, 2, "元日 振替休日"),
                holiday(2017, 1, 9, "成人の日"),
            ])
        );
        // start > end は空。
        assert_eq!(
            h.call_between(&mut *store, date(2017, 1, 9), date(2017, 1, 1))
                .unwrap(),
            Ok(vec![])
        );
    });
}

#[test]
fn year_holidays_match_native_crate() {
    with_instance(|h, store| {
        for year in 1948..=2060 {
            assert_eq!(
                h.call_year_holidays(&mut *store, year).unwrap(),
                expected(jpholiday::year_holidays(year)),
                "year {year}"
            );
        }
    });
}
