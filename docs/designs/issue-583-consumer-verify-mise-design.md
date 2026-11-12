---
id: "DESIGN-034Wv6hcYDLZycrIcEQ7Ec"
type: "design"
status: "approved"
title: "Issue #583 設計 — mr_boxington 回帰の原因確定と検証方針(設定変更不要の判断)"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/583"
    - "docs/specs/issue-583-consumer-verify-mise.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-583-consumer-verify-mise-design.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/designs/README.md"
  implements:
    - "docs/specs/issue-583-consumer-verify-mise.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-583-consumer-verify-mise.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-583-consumer-verify-mise.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-583-consumer-verify-mise.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/pull/580"
    - "https://github.com/scarecrow173/ruviform/pull/584"
  defers: []
  deferred-by: []
---

# Issue #583 設計 — mr_boxington 回帰の原因確定と検証方針(設定変更不要の判断)

## Overview

consumer-verify preflight の失敗は、`mise.toml` の rust tool option `mr_boxington = true` に対して companion tool `mr-boxington` が `[tools]` に未宣言だったことが直接原因。該当設定は後続の PR #584 で除去済みのため、本 Issue では **設定変更を行わず**、原因の確定・再現・現行 main での回帰検証・workflow_dispatch による実 CI 確認を行い、doc-chain に記録する。

## mr_boxington が有効化される経路の切り分け

| 候補経路 | 判定 | 根拠 |
| --- | --- | --- |
| リポジトリ `mise.toml` | **該当(原因)** | `4f31460e` の diff が `rust = { ..., mr_boxington = true }` を追加。`mr-boxington` tool は `[tools]` に無し |
| `mise.ci.toml` / `MISE_ENV=ci` | 非該当 | 当該 run(2026-11-12T02:16Z、sha `4f31460e`)時点で `mise.ci.toml` は未存在。`MISE_ENV` は #584 で初めて workflow env に追加 |
| mise-action@v5.2.0 の既定 | 非該当 | action は mise 本体の install と `mise install --locked` の自動実行のみ。tool option を付与する機能はない |
| `MISE_*` env / workflow env | 非該当 | run の env は `RUVIFORM_REV` のみ。`MISE_*` 系の option 注入なし |
| global mise settings | 非該当 | GitHub hosted runner に global config は存在しない。ローカルでは `~/.config/mise/config.toml` が `mr-boxington = "latest"` を持ち error をマスクするが、CI には無関係 |
| mise バージョン回帰 | 部分該当(顕在化要因) | option 必須 tool の検証が shim 再構築で hard error になる挙動は mise 2026.11.6 で観測。古い mise で同じ宣言が通っていたかは未検証だが、宣言自体が不完全であることに変わりはない |

## 失敗メカニズム

1. `jdx/mise-action@v5.2.0` が `mise.lock` を検出し `mise install --locked` を自動実行(ログ: "Detected a mise lock file, running `mise install --locked`")。
2. lockfile 内の全 7 tool(uv/prek/git-lfs/apm/node/actionlint/rust)を install。rust@1.99.0 は rustup 経由で正常 install。
3. install 完了後の "rebuild user shims" で、rust tool spec の `mr_boxington = true` を解決する際に companion tool `mr-boxington` が `[tools]` に無いことを検出し exit 1。
4. preflight job 失敗 → verify-audiolens / verify-synvelis が `needs.preflight.result != success` でゲート skip → summary が lane 失敗として fail。

lane 差の説明: `daily-ci.yml` は mise-action に `install: false` を指定し `mise install --locked node` のみ実行するため rust を install せず同 sha で成功(実測: run 43140224101 success)。`ci.yml` / `consumer-verify.yml` は `install: false` を付けていないため全 tool install → 同じく失敗(ci run 43140254535 も同 sha で failure)。

## 現行 main の状態(修正済みの根拠)

- `246e364e`(PR #584)で `mise.toml` の `rust` 行(`mr_boxington` 含む)が削除され、`mise.ci.toml` に `rust = { version = "1.99.0", components = ["rustfmt", "clippy"] }` が新設。`mr_boxington` は CI の sccache(`RUSTC_WRAPPER`)と競合するため意図的に CI pin から除外された(commit message 明記)。
- `ci.yml` / `consumer-verify.yml` に `MISE_ENV: ci` が追加され、CI では `mise.toml` + `mise.ci.toml` がマージされ `mise.ci.lock` が lock 正本となる。
- `mise.lock` に rust 項目は残っていない。`mise.ci.lock` は rust のみを保持し、`MISE_ENV=ci` 下の `mise install --locked` は `mise.lock` 側の tool とマージして全 7 tool を解決する(実測済み)。
- ci.yml run 43270011255(sha `246e364e`)で verify lane(windows/ubuntu/macos)は全て success — mise 関連の preflight/セットアップは実 CI で正常動作を確認済み。同 run の失敗は `ui-gpui-verify` / `ui-gpui-offscreen-verify` / `icc-verify` であり本 Issue の範囲外。

## 判断: 設定変更なし

- offending な `mr_boxington` option は main に存在しない。再追加する理由もない(CI では sccache 競合、ローカルは global mise 設定に委ねる #584 の方針)。
- mise 自体の pin は導入しない。失敗原因はリポジトリ設定の不整合であり、mise のバグと確定していないため。将来 mise 側の破壊的変更で再発した場合は別 Issue で pin を検討する。
- consumer-verify / ci の mise-action `install` 挙動は変更しない。現行の全 tool install は `MISE_ENV=ci` で正しく解決されることを確認済み。

## Verification

- 再現(旧設定): sha `4f31460e` の `mise.toml`/`mise.lock` を clean global config(`MISE_CONFIG_DIR` 差し替え)で mise 2026.11.7 windows-x64 にかけ `mise install --locked` → exit 1、CI と同一の `mr_boxington` エラーを確認。
- 現行 main: `mise install --locked node`、`MISE_ENV=ci mise install --locked`(全 7 tool)、`MISE_ENV=ci mise install --locked rust git-lfs node` が全て exit 0。`MISE_DATA_DIR`/`MISE_CACHE_DIR` を空にした fresh install でも `MISE_ENV=ci mise install --locked node` が node@26.12.1 を正しく解決・install。
- 実 CI: 対象ブランチで `gh workflow run consumer-verify.yml` を実行し preflight job の success を確認(結果 URL は PR 本文に記載)。
