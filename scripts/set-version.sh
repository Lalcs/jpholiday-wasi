#!/usr/bin/env bash
# ワークスペースのバージョン（Cargo.toml の [workspace.package] version）を書き換える。
# リリース時にタグ（v1.2.3 → 1.2.3）の値を注入するために使う。
#
#   scripts/set-version.sh 1.2.3
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

awk -v version="$version" '
  /^\[/ { section = $0 }
  section == "[workspace.package]" && !done && /^version = "/ {
    print "version = \"" version "\""
    done = 1
    next
  }
  { print }
  END { if (!done) exit 1 }
' Cargo.toml > Cargo.toml.tmp || {
  rm -f Cargo.toml.tmp
  echo "error: [workspace.package] version not found in Cargo.toml" >&2
  exit 1
}
mv Cargo.toml.tmp Cargo.toml

# Cargo.lock のワークスペースメンバーのバージョンだけを更新する（依存クレートは固定のまま）。
cargo update --workspace

actual=$(cargo metadata --format-version 1 --no-deps |
  grep -o '"name":"jpholiday-cli","version":"[^"]*"' |
  sed 's/.*"version":"\([^"]*\)"/\1/')
if [ "$actual" != "$version" ]; then
  echo "error: failed to set version (expected $version, got '$actual')" >&2
  exit 1
fi
echo "version set to $version"
