---
id: "TSPEC-630abiValidator06"
type: "test-spec"
status: "approved"
title: "Issue #630 外部 ABI validator slice — test specification"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-abi-validator.md"
    - "docs/designs/issue-630-abi-validator.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-630-abi-validator.md"
  implements:
    - "docs/specs/issue-630-abi-validator.md"
    - "docs/designs/issue-630-abi-validator.md"
  implemented-by: []
  depends-on:
    - "docs/test-specs/issue-630-test-plugin-suite.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #630 外部 ABI validator slice — test specification

## Feature: 外部 ABI validator lane

### Rule: runner は fail-closed でなければならない

validator binary・artifact・build のいずれかが欠けた場合、runner は
Pass を報告してはならない。NotRun/Unsupported verdict と非 0 exit を
記録する。

- Example: `--clap-validator` 未指定かつ `CLAP_VALIDATOR` 未設定で
  binary 不在 → 全 CLAP leg が NotRun、verdict NotRun、exit 1。
- Example: validator が非 0 exit で JSON report を出さない
  （tool 側 panic）→ その leg は Unsupported（plugin defect ではない）。

### Rule: verdict 語彙は Passed/Failed/NotRun/Unsupported に限定

adapter 自己検証を外部適合に読み替えない。実走していない validator は
NotRun、tool が対象を扱えない場合のみ Unsupported。

- Example: clap-validator JSON の `status.code` が `failed`/`crashed`
  → leg Failed。`warning`/`skipped` は Pass に影響しないが counts に記録。
- Example: VST3 validator 出力に `Result: N tests passed, M tests failed`
  が無い → Failed（exit 0 でも summary が取れなければ Pass にしない）。

### Rule: test_fault の param-mutating case は宣言的に除外する

fatal fault mode が param 変更で武装される設計上、`param-set-*`/
`param-fuzz-*` は意図的 crash を誘発する。canonical run では
`--exclude "param-(set|fuzz)"` を付与し、unfiltered run は
informational leg として別記録する。

- Example: fault canonical leg は `--exclude` 付き command が
  run.json の `leg.command` に残る。`leg.exclusion.reason` が
  設計理由を保持する。

### Rule: evidence は再現可能な provenance を含む

run.json は tool/artifact の SHA-256・version・git SHA・OS・
command・exit code を保持し、生出力は別ファイルに分離する。

- Example: `tools.clapValidator.sha256` と `artifacts.clapDll.sha256`
  が 64 hex で記録される。`legs[].rawOutput` は run.json に残さない
  （`clap-*.log`/`vst3-validator.log` へ）。

## 検証コマンド（canonical）

- `node ci/plugin-abi-validator/run.mjs` — 実走（本 slice の新規 runner）
- `node --test ci/plugin-abi-validator/tests/` — contract test
- `mise run fmt-check` / `check|lint|test <pkg>`（両 fixture crate）
- `mise run check|lint|test-l1-real-plugin` — 非回帰
- `mise run test-rt` / `mise exec -- prek run --all-files`
