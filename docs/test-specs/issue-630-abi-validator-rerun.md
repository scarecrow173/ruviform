---
id: "TSPEC-630abiValidatorRerun06"
type: "test-spec"
status: "approved"
title: "Issue #630 外部 ABI validator 再実走 slice — test specification"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-abi-validator-rerun.md"
    - "docs/designs/issue-630-abi-validator-rerun.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-630-abi-validator-rerun.md"
  implements:
    - "docs/specs/issue-630-abi-validator-rerun.md"
    - "docs/designs/issue-630-abi-validator-rerun.md"
  implemented-by: []
  depends-on:
    - "docs/test-specs/issue-630-abi-validator.md"
    - "docs/test-specs/issue-630-preset-spawn-fault.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #630 外部 ABI validator 再実走 slice — test specification

## Feature: declared-expectation 付き外部 ABI validator lane

### Rule: create-decline expectation は観測された decline のみを Pass にする

`expectCreateDecline` を持つ plugin leg は、failed/crashed case が
全て宣言 pattern（create_plugin-null signature）に一致し、crash
0・decline 1 件以上のときのみ Passed でなければならない。

- Example: `test_spawn_fail` leg で 35 件全てが
  `Could not create the plugin instance ... returned a null pointer`
  → Passed、`leg.expectation.declinedCases == 35`、
  `unexpectedCases == []`、`crashedCases == 0`。
- Example: 同 leg に pattern 不一致の failed case が 1 件でも
  あれば Failed。
- Example: JSON report が無い（validator 異常終了）場合は
  expectation を適用せず generic verdict（Unsupported/Failed）
  に落ちる。

### Rule: library-level exclusion は理由つきで leg に記録されなければならない

fixture target の `preset-discovery-(crawl|load)` 除外は全 CLAP
leg の `leg.exclusions` に pattern・reason を保持し、plugin 固有
除外（test_fault の `param-(set|fuzz)`）と合成される。

- Example: test_fault canonical leg の `exclusions` は 2 件
  （library preset-discovery + plugin param）。
- Example: informational unfiltered fault leg は plugin 固有除外を
  解除するが library 除外は維持する（`includePlugin: false`）。

### Rule: VST3 declared-uncreatable は decline 証拠を要求する

`VST3_DECLARED_UNCREATABLE` に登録された class は raw output 中の
`Failed to create component instance of <class>!` marker 出現回数を
`leg.declaredUncreatable[].occurrences` に記録し、Passed leg で
occurrences が 0 の class があれば Failed に倒さなければならない。

- Example: `test_spawn_fail` の occurrences が 47 → 記録のみで
  leg.status は validator の `Result:`（235 passed, 0 failed）に
  従い Passed。
- Example: bundle から class が消え marker が 0 件 → leg Failed。
- Example: `--vst3-package` が宣言 table に無い package →
  `applyVst3DeclaredUncreatable` は no-op（production adapter に
  fixture 宣言を適用しない）。

### Rule: schema /2 の新フィールドは run.json に残らなければならない

`run.json` は `schema: "rf630-abi-validator-run/2"` を名乗り、
canonical 7 leg 全ての verdict・exclusion/expectation/
declaredUncreatable・tool/artifact SHA-256・git HEAD・build 結果を
保持する。rawOutput は `clap-*.txt`/`vst3-validator.txt` に分離し
run.json には残さない（/1 と同じ規約）。

- Example: `test_spawn_fail` leg の `exitCode` は 1（validator が
  failed case を報告）でも `status: "Passed"` +
  `expectation.kind: "create-declined"` で記録される — exit code
  と verdict の乖離は declared expectation によるものであり、
  説明なしに握り潰さない。

## 検証コマンド（canonical）

- `mise run test-plugin-abi-validator` — 本 slice の主検証
  （opt-in・validator binary 不在は fail-closed NotRun）
- `node --test ci/plugin-abi-validator/tests/` — runner contract
  test（canonical mise task 未定義のため補助証拠）
- `mise run fmt-check` / `mise run check|lint|test
  ruviform-plugin-test-{vst3,clap}` — 非回帰（本 slice は
  `crates/` 非編集）
- `mise run check-l1-real-plugin` / `lint-l1-real-plugin` /
  `test-l1-real-plugin` — 非回帰
- `mise run test-rt` / `mise exec -- prek run --all-files`
