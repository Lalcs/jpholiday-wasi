#!/usr/bin/env bash
# ワークスペースのバージョンを書き換える。リリース時にタグ（v1.2.3 → 1.2.3）の値を注入するために使う。
#
#   scripts/set-version.sh 1.2.3
#
# 書き換える対象:
#   - Cargo.toml の [workspace.package] version（CLI / Component のバージョン）
#   - WIT の package 宣言（lalcs:jpholiday@1.2.3）
#   - Cargo.lock のワークスペースメンバーのバージョン
#
# macOS（BSD 系ツール）でも動くよう sed -i ではなく awk を使う。
set -euo pipefail

if [ "$#" -ne 1 ]; then
  echo "usage: $0 <version>" >&2
  exit 2
fi
version=$1

# SemVer 以外は拒否する（awk に特殊文字が渡らないことの保証も兼ねる）。
semver='^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$'
if ! printf '%s\n' "$version" | grep -Eq "$semver"; then
  echo "error: '$version' is not a valid SemVer version (expected e.g. 1.2.3)" >&2
  exit 1
fi

cd "$(dirname "$0")/.."

wit=crates/jpholiday-component/wit/jpholiday.wit

# rewrite <file> <説明> <awk プログラム>: awk の結果でファイルを置き換える。
# awk プログラムは対象を書き換えたら done = 1 にし、見つからなければ exit 1 する。
rewrite() {
  local file=$1 what=$2 program=$3
  awk -v version="$version" "$program" "$file" > "$file.tmp" || {
    rm -f "$file.tmp"
    echo "error: $what not found in $file" >&2
    exit 1
  }
  mv "$file.tmp" "$file"
}

rewrite Cargo.toml "[workspace.package] version" '
  /^\[/ { section = $0 }
  section == "[workspace.package]" && !done && /^version = "/ {
    print "version = \"" version "\""
    done = 1
    next
  }
  { print }
  END { if (!done) exit 1 }
'

rewrite "$wit" "package declaration (package lalcs:jpholiday@...;)" '
  !done && /^package lalcs:jpholiday@[^;]*;$/ {
    print "package lalcs:jpholiday@" version ";"
    done = 1
    next
  }
  { print }
  END { if (!done) exit 1 }
'

# Cargo.lock のワークスペースメンバーのバージョンだけを更新する（依存クレートは固定のまま）。
cargo update --workspace

actual=$(cargo metadata --format-version 1 --no-deps |
  grep -o '"name":"jpholiday-cli","version":"[^"]*"' |
  sed 's/.*"version":"\([^"]*\)"/\1/')
if [ "$actual" != "$version" ]; then
  echo "error: failed to set Cargo version (expected $version, got '$actual')" >&2
  exit 1
fi
if ! grep -qxF "package lalcs:jpholiday@$version;" "$wit"; then
  echo "error: failed to set WIT package version to $version" >&2
  exit 1
fi
echo "version set to $version"
