#!/usr/bin/env bash
# jpholiday CLI のスモークテスト。ネイティブ版・WASI 版のどちらにも使える。
#
# 引数には CLI を起動するコマンドを渡す（その後ろに各テストの引数が付け足される）:
#   scripts/smoke-test.sh target/release/jpholiday
#   scripts/smoke-test.sh wasmtime run target/wasm32-wasip1/release/jpholiday.wasm
set -euo pipefail

if [ "$#" -eq 0 ]; then
  echo "usage: $0 <command to run jpholiday...>" >&2
  exit 2
fi
runner=("$@")
failures=0

# check <期待する終了コード> <期待する標準出力> <jpholiday の引数...>
check() {
  local expected_code=$1 expected_out=$2
  shift 2
  local out code=0
  out=$("${runner[@]}" "$@" 2>/dev/null) || code=$?
  if [ "$code" = "$expected_code" ] && [ "$out" = "$expected_out" ]; then
    echo "ok: jpholiday $*"
  else
    echo "FAIL: jpholiday $*" >&2
    echo "  expected (exit $expected_code): $expected_out" >&2
    echo "  actual   (exit $code): $out" >&2
    failures=$((failures + 1))
  fi
}

check 0 "true" is-holiday 2026-01-01
check 1 "false" is-holiday 2026-01-02
check 0 "憲法記念日 振替休日" name 2026-05-06
check 1 "" name 2026-05-07
check 0 '"国民の休日"' name 2026-09-22 --json
check 1 "null" --json name 2026-09-24
check 0 $'2026-09-21\t敬老の日\n2026-09-22\t国民の休日\n2026-09-23\t秋分の日' month 2026 9
check 0 '[{"date":"2026-01-01","name":"元日"},{"date":"2026-01-12","name":"成人の日"}]' --json month 2026 1
check 0 $'2026-05-04\tみどりの日\n2026-05-05\tこどもの日\n2026-05-06\t憲法記念日 振替休日' between 2026-05-04 2026-05-06
check 2 "" is-holiday 2026-02-30
check 2 "" month 2026 13
check 2 "" unknown-command

year_count=$("${runner[@]}" year 2026 | wc -l | tr -d ' ')
if [ "$year_count" = 18 ]; then
  echo "ok: jpholiday year 2026 (18 holidays)"
else
  echo "FAIL: jpholiday year 2026: expected 18 lines, got $year_count" >&2
  failures=$((failures + 1))
fi

# 引数を省略すると今日（JST）を使う。WASI ランタイムから時計を読めることを確認する。
out=$("${runner[@]}" is-holiday 2>/dev/null) && code=0 || code=$?
if { [ "$code" = 0 ] && [ "$out" = "true" ]; } || { [ "$code" = 1 ] && [ "$out" = "false" ]; }; then
  echo "ok: jpholiday is-holiday (today)"
else
  echo "FAIL: jpholiday is-holiday (today): exit $code, output: $out" >&2
  failures=$((failures + 1))
fi

version=$("${runner[@]}" --version)
case "$version" in
  "jpholiday "*" (jpholiday crate "*")") echo "ok: jpholiday --version ($version)" ;;
  *)
    echo "FAIL: jpholiday --version: $version" >&2
    failures=$((failures + 1))
    ;;
esac

if [ "$failures" -gt 0 ]; then
  echo "$failures check(s) failed" >&2
  exit 1
fi
echo "all checks passed"
