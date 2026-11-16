---
id: "SPEC-630abiValidator02"
type: "spec"
status: "approved"
title: "Issue #630 外部 ABI validator slice — clap-validator 実走 + VST3 SDK validator + opt-in runner"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/discovery/issue-630-abi-validator.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-630-abi-validator.md"
      - type: "file"
        path: "ci/plugin-abi-validator/run.mjs"
      - type: "file"
        path: "ci/plugin-abi-validator/tests/run.test.mjs"
      - type: "file"
        path: "evidence/rf630/abi-validator/run.json"
  implements:
    - "docs/discovery/issue-630-abi-validator.md"
  implemented-by:
    - "docs/designs/issue-630-abi-validator.md"
  depends-on:
    - "docs/specs/issue-630-test-plugin-suite.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #630 外部 ABI validator slice — spec

## 目的

Issue #630 AC3: Test Plugin の実 ABI 準拠を、Ruviform production
adapter/host 実装から独立した外部権威で検証する。本 slice は
`clap-validator`（CLAP）と Steinberg VST3 SDK `validator`（VST3）を
実バイナリに対して実走し、その結果と tool/artifact の provenance を
`evidence/rf630/` に記録する opt-in lane を提供する。

## 要求

1. **実走**: 両 fixture cdylib（`ruviform_plugin_test_clap.dll` /
   `ruviform_plugin_test_vst3.dll`）を外部 validator が Discover /
   Instantiate / Process / State 経路まで実際に駆動すること。
   adapter 自己検証を外部適合に読み替えない。
2. **fail-closed**: validator binary・SDK build 成果物・fixture
   artifact・必須 metadata のいずれかが欠けた場合、runner は
   NotRun/Unsupported verdict と非 0 exit を記録し、成功と偽らない。
3. **正直な verdict**: Passed / Failed / NotRun / Unsupported を
   区別する。validator 自体の異常終了（tool 側 bug）は plugin defect
   と混同せず Unsupported として記録する。
4. **provenance 記録**: validator/SDK の version・license・SHA-256、
   fixture artifact の SHA-256、git SHA、OS/toolchain、実行 command、
   exit code、validator 出力を `evidence/rf630/` に残す。
5. **opt-in**: CI 必須 lane にしない。`mise.toml` は本 slice では編集
   せず、意図する task 定義を design 文書と PR 本文に TOML スニペット
   として明記する（登録は Coordinator が統合時に実施）。
6. **workspace 依存を増やさない**: validator tool は外部 binary として
   runner が解決する。`Cargo.toml`/`Cargo.lock` は変更しない。

## 受入条件（本 slice）

- `node ci/plugin-abi-validator/run.mjs` が実環境で exit 0 を返し、
  `evidence/rf630/abi-validator/run.json` に全 leg の verdict・digest・
  出力参照を記録する。
- `node --test ci/plugin-abi-validator/tests/` の contract test が
  fail-closed 契約（missing tool/artifact → NotRun、crash → 非 Pass）
  を固定する。
- fixture 修正は validator 指摘の契約違反に限定し、production
  adapter crate は zero-diff を維持する。

## 非目標

- CI 必須 lane 化・`.github/**` 変更（Coordinator/後続 slice）。
- production adapter の修正（指摘があれば `ask` で分離判断）。
- Preset 面・pre-Hello fault・consumer verdict（残 AC・後続 slice）。
- clap-validator `param-fuzz-sample-accurate` の tool 側 panic 修正
  （upstream 依存 — informational leg で記録）。
