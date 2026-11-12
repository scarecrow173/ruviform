---
id: "SPEC-034Wv5sDkKQHAHjQDgLKG3"
type: "spec"
status: "approved"
title: "Issue #583 consumer-verify preflight の mise mr_boxington 失敗を調査・検証する"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/583"
    - "https://github.com/scarecrow173/ruviform/actions/runs/43140257916"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-583-consumer-verify-mise.md"
      - type: "file"
        path: "docs/designs/issue-583-consumer-verify-mise-design.md"
      - type: "file"
        path: "docs/plans/issue-583-consumer-verify-mise-plan.md"
      - type: "file"
        path: "docs/tasks/issue-583-consumer-verify-mise.md"
      - type: "file"
        path: "docs/test-specs/issue-583-consumer-verify-mise.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
  implements: []
  implemented-by:
    - "docs/plans/issue-583-consumer-verify-mise-plan.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-583-consumer-verify-mise-design.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-583-consumer-verify-mise.md"
  references:
    - "https://github.com/scarecrow173/ruviform/pull/580"
    - "https://github.com/scarecrow173/ruviform/pull/584"
  defers: []
  deferred-by: []
---

# Issue #583 consumer-verify preflight の mise mr_boxington 失敗を調査・検証する

## Problem

2026-11-12T02:16Z の `consumer-verify` workflow_dispatch run([run 43140257916](https://github.com/scarecrow173/ruviform/actions/runs/43140257916)、sha `4f31460e`)で preflight job が失敗した。`jdx/mise-action@v5.2.0` が検出した `mise.lock` に対して自動実行した `mise install --locked` が、全 7 tool の install 完了後の user shims 再構築で次のエラーを出して exit 1 となった。

```
mise ERROR failed to rebuild user shims
mise ERROR rust's mr_boxington option requires mr-boxington in [tools]; add `mr-boxington = "latest"`
mise ERROR Version: 2026.11.6 linux-x64 (2026-11-10)
```

verify-audiolens / verify-synvelis / summary は preflight 失敗のゲート連鎖であり、独立した失敗ではない。直前 run(11-11、sha `9865cd57`)は成功している。

## Diagnosis(変更前に診断する)

- 直接原因は sha `4f31460e`(PR #580)が `mise.toml` の `rust` tool に `mr_boxington = true` を追加した一方、要求される companion tool `mr-boxington` を `[tools]` に宣言しなかったこと。`mr_boxington` は rust tool の option で、cargo を `mr-boxington`(packslip backend の build cache wrapper)経由にする有効化フラグであり、mise は shim 再構築時に「option 有効 ⇒ 対応 tool が `[tools]` に存在すること」を検証する。
- `mr_boxington` が有効化される経路は **リポジトリの `mise.toml` のみ** である。mise-action@v5.2.0 の既定・`MISE_*` env・global settings がこの option を立てる経路はない(当該 run に `MISE_ENV` は未設定で、`#584` で `MISE_ENV=ci` が入るのは後続 commit)。
- mise 側の新バージョン回帰というより「option を有効化したのに必須 tool を宣言していない」設定不整合が、mise 2026.11.6 で shim 再構築時の検証として顕在化した形。実際に同一エラーを mise 2026.11.7 windows-x64・clean global config で `mise install --locked` により再現済み(後述)。
- consumer-verify / ci workflow は mise-action に `install: false` を付けていないため、action が `mise.lock` を検出して全 tool の `mise install --locked` を自動実行する。daily-ci.yml は `install: false` + `mise install --locked node` のみで rust を install しないため同 sha で成功しており、これが lane 差の説明になる。
- その後の PR #584(`246e364e`)で `rust` tool は `mise.toml` から除去され、CI 専用 pin(`rust = 1.99.0` + rustfmt/clippy、`mr_boxington` なし — CI の sccache `RUSTC_WRAPPER` と競合するため意図的に除外)が `mise.ci.toml` に分離された。現行 main には offending な設定は残っていない。

## Requirements

1. 失敗の直接原因を、mise-action 既定・env・settings・mise バージョン回帰の別を含めて根拠つきで特定し記録する。
2. 現行 main の設定で `mise install --locked node` が mise 2026.11.6 以降で exit 0 になることを検証する。
3. `MISE_ENV=ci mise install --locked rust git-lfs node` が同バージョンで exit 0 になることを検証する。
4. `consumer-verify.yml` を workflow_dispatch で実行し、preflight job が `success` になることを確認する。
5. 既存の検証意図(`rust = "1.99.0"` の CI pin、`--locked` 運用、mise.ci.toml / mise.ci.lock の分離構造)を弱めない。設定変更が不要と判断した場合はその根拠を記録する。

## Acceptance Criteria

- AC1: 直接原因(mise.toml の rust `mr_boxington` option に対する `mr-boxington` tool 未宣言)が PR 本文に根拠つきで記載されている。
- AC2: `mise install --locked node` が mise 2026.11.6 以降で exit 0。
- AC3: `MISE_ENV=ci mise install --locked rust git-lfs node` が同バージョンで exit 0。
- AC4: 対象ブランチへの `gh workflow run consumer-verify.yml` で preflight job が `success`(実行 URL を PR 本文に記載)。
- AC5: `rust = "1.99.0"` CI pin・`--locked`・mise.ci.toml/mise.ci.lock 分離が維持されている。

## Non-goals

- `mr_boxington` / mr-boxington を CI またはリポジトリ mise 設定へ再導入すること(#584 が sccache 競合を理由に CI pin から意図的に除外済み。ローカル開発は各開発者の global mise 設定に委ねる現行方針を維持)。
- mise 自体のバージョン pin 導入(失敗原因はリポジトリ設定であり、mise 側の問題とは確定していないため。再発時に再検討)。
- consumer 側レーン(audiolens / synvelis)固有の検証内容や、preflight 以外の job の挙動変更。
- ci-issue-tracker 系ワークフローの管理ロジック変更、製品コード変更。
