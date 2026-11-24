#!/usr/bin/env bash
# ruviform Linux 検証イメージ entrypoint（Issue #953）
#
# /src に read-only mount されたリポジトリを /work（named volume）へ
# rsync し、コンテナ内で `mise run <task>` を実行する。
#
#   docker run --rm \
#     -v <repo>:/src:ro \
#     -v ruviform-linux-verify-work:/work \
#     -v ruviform-linux-verify-mise:/root/.local/share/mise \
#     -v ruviform-linux-verify-cargo:/root/.cargo \
#     ruviform-linux-verify <mise-task> [args]
#
# target/ と .git は同期対象外: worktree の target は mbx managed
# target への symlink でコンテナ内では dangling となり、worktree
# の .git は Windows 絶対パスを指す gitfile でそのまま使えないため。
# cargo は /work 配下に独立した target を作成する（volume 永続化で
# 2 回目以降は増分ビルド）。prek 等の git 依存処理向けに、/work 側へ
# 使い捨ての snapshot commit を作る。
set -euo pipefail

SRC="${RVF_SRC:-/src}"
WORK="${RVF_WORK:-/work}"

if [ ! -f "$SRC/mise.toml" ]; then
    echo "linux-verify: ruviform リポジトリが $SRC に mount されていません" >&2
    echo "usage: docker run --rm -v <repo>:/src:ro -v <vol>:/work ruviform-linux-verify <mise-task> [args]" >&2
    exit 2
fi
if [ "$#" -lt 1 ]; then
    echo "usage: docker run ... ruviform-linux-verify <mise-task> [args]" >&2
    echo "例: ruviform-linux-verify fmt-check / ruviform-linux-verify test ruviform-io-file" >&2
    exit 2
fi

# mount 元は read-only。/work へミラーする（--delete で前回同期後の削除も反映。
# --exclude 指定した /work/.git・/work/target は削除対象から保護される）。
rsync -a --delete --exclude='/.git' --exclude='/target' "$SRC/" "$WORK/"

cd "$WORK"
# /work 配下の mise.toml/mise.ci.toml を trusted 扱いにする
# （コンテナ内の一時コピーなので trust store への個別登録は不要）
export MISE_TRUSTED_CONFIG_PATHS="$WORK"
# 初回のみ rust+prek+actionlint を解決（CI の `mise install --locked` と同一。
# MISE_DATA_DIR は volume 永続化を推奨 — 未 mount 時は毎回 toolchain install が走る）。
mise install --locked rust prek actionlint

# prek run --all-files 等のための使い捨て snapshot。commit は /work 側のみで
# host の履歴・config には触れない。
if [ ! -d .git ]; then
    git init -q
    git config user.name "ruviform linux verify"
    git config user.email "linux-verify@localhost"
fi
git add -A >/dev/null
git commit -qm "linux-verify snapshot" --no-gpg-sign >/dev/null 2>&1 || true

exec mise run "$@"
