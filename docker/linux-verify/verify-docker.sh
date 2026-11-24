#!/usr/bin/env bash
# `mise run verify-docker` の実体（Issue #953）。worktree を read-only
# mount し、named volume 上の /work でコンテナ内 lane を実行する。
# 使い方・役割分担は evidence/rf953/ を参照。
set -euo pipefail

if [ "$#" -lt 1 ]; then
    echo "usage: mise run verify-docker <mise-task> [args]" >&2
    echo "例: mise run verify-docker fmt-check / mise run verify-docker test ruviform-io-file" >&2
    exit 2
fi

# worktree ルートを docker bind source に使う。Git Bash では
# G:/... 形式、Linux/macOS では /... 形式が返る。
repo="$(git rev-parse --show-toplevel)"

# Git Bash (MSYS) は -v "G:/repo:/src:ro" のような引数を POSIX パスへ
# 変換して docker CLI の -v 形式を壊すため、パス変換を抑止する。
export MSYS_NO_PATHCONV=1

# - /src        : worktree 本体（read-only — コンテナから host を書き換えない）
# - work volume : rsync 先。target/ と .git はコンテナ側で独立管理
#   （host の target は mbx managed target への symlink で共有不能、
#     worktree の .git は Windows 絶対パスを指す gitfile で使用不能）
# - mise volume : `mise install --locked` で解決した rust/prek 等の永続化
# - cargo volume: crates.io registry cache の永続化
exec docker run --rm \
    -v "$repo:/src:ro" \
    -v ruviform-linux-verify-work:/work \
    -v ruviform-linux-verify-mise:/root/.local/share/mise \
    -v ruviform-linux-verify-cargo:/root/.cargo \
    ruviform-linux-verify "$@"
