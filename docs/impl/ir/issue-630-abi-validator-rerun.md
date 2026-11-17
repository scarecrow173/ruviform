---
id: "IMPL-630abiValidatorRerun07"
type: "impl"
status: "completed"
title: "Issue #630 外部 ABI validator 再実走 slice — 実装記録"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/tasks/issue-630-abi-validator-rerun.md"
    - "docs/plans/issue-630-abi-validator-rerun.md"
    - "docs/specs/issue-630-abi-validator-rerun.md"
    - "docs/designs/issue-630-abi-validator-rerun.md"
  changes:
    added:
      - type: "file"
        path: "evidence/rf630/abi-validator-rerun-verification.md"
      - type: "file"
        path: "evidence/rf630/abi-validator/clap-test_spawn_fail.txt"
      - type: "file"
        path: "docs/discovery/issue-630-abi-validator-rerun.md"
      - type: "file"
        path: "docs/specs/issue-630-abi-validator-rerun.md"
      - type: "file"
        path: "docs/designs/issue-630-abi-validator-rerun.md"
      - type: "file"
        path: "docs/plans/issue-630-abi-validator-rerun.md"
      - type: "file"
        path: "docs/tasks/issue-630-abi-validator-rerun.md"
      - type: "file"
        path: "docs/test-specs/issue-630-abi-validator-rerun.md"
      - type: "file"
        path: "docs/impl/ir/issue-630-abi-validator-rerun.md"
      - type: "file"
        path: "docs/impl/exp/issue-630-abi-validator-rerun.md"
      - type: "file"
        path: "docs/impl/exp/issue-630-abi-validator-rerun-exp.jsonl"
    modified:
      - type: "file"
        path: "ci/plugin-abi-validator/run.mjs"
      - type: "file"
        path: "ci/plugin-abi-validator/tests/run.test.mjs"
      - type: "file"
        path: "evidence/rf630/abi-validator/run.json"
      - type: "file"
        path: "evidence/rf630/abi-validator/clap-test_gain.txt"
      - type: "file"
        path: "evidence/rf630/abi-validator/clap-test_fault.txt"
      - type: "file"
        path: "evidence/rf630/abi-validator/clap-test_stateful.txt"
      - type: "file"
        path: "evidence/rf630/abi-validator/clap-test_instrument.txt"
      - type: "file"
        path: "evidence/rf630/abi-validator/clap-test_editor.txt"
      - type: "file"
        path: "evidence/rf630/abi-validator/clap-test_fault-unfiltered.txt"
      - type: "file"
        path: "evidence/rf630/abi-validator/vst3-validator.txt"
  implements:
    - "docs/tasks/issue-630-abi-validator-rerun.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-630-abi-validator-rerun.md"
  blocks: []
  supersedes: []
  superseded-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-630-abi-validator-rerun-exp.jsonl"
    rejected: []
---

# Issue #630 外部 ABI validator 再実走 slice — 実装記録

## 実装したもの

### runner（`ci/plugin-abi-validator/run.mjs`）

- `CLAP_PLUGINS` に `test_spawn_fail`
  （`org.ruviform-test.clap-test-spawn-fail`）を追加し、
  `expectCreateDecline: { pattern, reason }` を宣言。
- `normalizeClapLeg` に declared create-decline 評価を追加:
  JSON parse 成功時のみ、failed case を pattern 一致の
  `declined` / 不一致の `unexpected` に分類し、
  `crashed === 0 && declined > 0 && unexpected === 0` で
  Passed + `leg.expectation` 記録、それ以外は Failed。
  `proc.error`・JSON 無しは expectation 非適用。
- `CLAP_FIXTURE_LIBRARY_EXCLUDES`（`preset-discovery-(crawl|load)`、
  upstream free-audio/clap-validator issue #41 の MetadataReceiver
  self-deadlock — 0.4.1 pin 以降の master で修正）を新設し、
  fixture clap target の `excludes` として適用。production target
  は空。
- `clapLegExclusions(target, plugin, { includePlugin })` で
  target/plugin 両 level の除外を `{pattern, reason}` 配列に合成。
  leg 記録は旧 `exclusion` 単数形から `exclusions` 配列へ。
  informational unfiltered fault leg は `includePlugin: false`
  （plugin 固有 param 除外のみ解除、library 除外は維持）。
- `VST3_DECLARED_UNCREATABLE`（`ruviform-plugin-test-vst3` →
  `test_spawn_fail`）と `applyVst3DeclaredUncreatable(leg)` を追加。
  rawOutput 中の `Failed to create component instance of
  test_spawn_fail!` marker 出現回数を `leg.declaredUncreatable` に
  記録し、Passed leg で occurrences 0 の宣言 class があれば
  Failed に倒す。宣言 table に無い package では no-op。
- run.json schema を `rf630-abi-validator-run/2` に bump。

### contract test（`ci/plugin-abi-validator/tests/run.test.mjs`）

32 test（旧 18 + 新規 14 系）。追加分: CLAP_PLUGINS の
spawn_fail 宣言、declared create-decline met → Passed、
unexpected failed/crash → Failed、JSON 無し fallback、
`clapLegExclusions` の合成、`applyVst3DeclaredUncreatable` の
記録・Passed→Failed 反転・非 fixture package no-op、
`VST3_DECLARED_UNCREATABLE` の内容固定。既存の fail-closed・
digest・production leg 契約は維持。

### evidence

`mise run test-plugin-abi-validator` の実走で
`evidence/rf630/abi-validator/` を更新:

- `run.json` schema /2、repo.head `1d18e297`、tool/artifact
  SHA-256、build 記録、7 canonical leg + 1 informational leg。
- `clap-test_spawn_fail.txt` 新規（35 declined / 6 passed /
  1 skipped / 0 warning、validator exit 1）。
- `clap-test_fault-unfiltered.txt` は informational
  Unsupported（validator exit 101、JSON 無し — tool 側 panic）。
- `vst3-validator.txt` に `test_spawn_fail` の decline 行を含む
  全出力（Result: 235 passed, 0 failed）。

## 実行結果（canonical）

- `mise run test-plugin-abi-validator` → verdict **Passed**、
  exit 0。canonical 7 leg 全 Passed:
  - clap test_gain: 27 passed / 14 skipped / 1 warning / 0 failed
  - clap test_fault: 20 / 14 / 1 / 0（preset-discovery + param
    除外の 2 件を exclusions に記録）
  - clap test_stateful: 32 / 9 / 1 / 0
  - clap test_instrument: 31 / 11 / 0 / 0
  - clap test_editor: 18 / 23 / 1 / 0
  - clap test_spawn_fail: declared expectation met — 35 declined、
    6 passed、1 skipped、0 warning（validator exit 1）
  - vst3-sdk-validator: 235 passed / 0 failed、
    `test_spawn_fail` decline 証拠 47 occurrences
- informational leg: test_fault unfiltered → Unsupported
  （exit 101、canonical verdict 対象外）。
- `node --test ci/plugin-abi-validator/tests/` → 32 pass, exit 0。

## 非回帰検証（実 exit code）

| コマンド | 結果 |
| --- | --- |
| `mise run fmt-check` | exit 0 |
| `mise run check ruviform-plugin-test-vst3` / `-clap` | exit 0 |
| `mise run lint ruviform-plugin-test-vst3` / `-clap` | exit 0 |
| `mise run test ruviform-plugin-test-vst3` | exit 0（91 passed 他） |
| `mise run test ruviform-plugin-test-clap` | exit 0（66 passed 他） |
| `mise run check-l1-real-plugin` / `lint-l1-real-plugin` | exit 0 |
| `mise run test-l1-real-plugin` | exit 0（最終実行で全 suite 緑。
  途中 instrument 系 test の flaky fail を観測 — 下記参照） |
| `mise run test-rt` | exit 0 |
| `mise exec -- prek run --all-files` | exit 0（初回は
  `vst3-validator.txt` の trailing whitespace/EOF を hook が
  自動修正 → 再実行で全 Pass） |

`test-l1-real-plugin` では `l1_real_test_plugin_e2e` の
instrument 系 test（`note_off_releases_to_silence` /
`transport_clicks`）が高負荷時に frame-exact assertion で
断続的に fail することを観測した。失敗 test は実行ごとに変わり、
最終実行では両 crate の e2e suite が 10/10 で全緑。`crates/` は
本 slice で zero-diff のため本 slice 起因ではなく、既存 lane の
flaky として正直に記録する（詳細は検証 md）。

## evidence

`evidence/rf630/abi-validator/run.json`（schema /2）+
`clap-*.txt`（7 本）+ `vst3-validator.txt`、
`evidence/rf630/abi-validator-rerun-verification.md`。
