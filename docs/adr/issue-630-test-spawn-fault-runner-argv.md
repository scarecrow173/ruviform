---
id: "ADR-7Qk2mXvLp9RwTd4YsN8bF"
type: "adr"
status: "accepted"
title: "issue-630-test-spawn-fault-runner-argv"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
  changes:
    added:
      - type: "file"
        path: "docs/adr/issue-630-test-spawn-fault-runner-argv.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-630-preset-spawn-fault.md"
    - "docs/designs/issue-630-preset-spawn-fault.md"
    - "docs/designs/issue-630-l1-runner.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-630-preset-spawn-fault.md"
    - "docs/designs/issue-630-preset-spawn-fault.md"
  derived-by:
    - "docs/plans/issue-630-preset-spawn-fault.md"
    - "docs/impl/ir/issue-630-preset-spawn-fault.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata: {}
---

# test spawn/pre-Hello fault を専用 bin ではなく l1-runner argv で注入する

## Context and Problem Statement

Issue #630 の spawn-fault slice は、fixture child process が
spawn 後・`Hello` 前に exit/abort/hang/stderr/bad-fixture を
起こす経路を必要とする。`ruviform_plugin_export::run` は wire v2
で argv を受理しない（plugin 選択と shared-memory bootstrap は
`CreateInstance` payload へ移動済み）ため、fault 注入は
`run()` の外側に置く必要がある。注入形態として (a) 専用 fault
bin の新設、(b) 既存 l1-runner bin への test-only argv flag の
2 案があった。

## Decision Drivers

- `run()` の no-argv 契約を変更しない（production grammar の
  fail-closed 性を維持）。
- 未認識の fault 指定が黙って無視されないこと。
- pre-Hello 到達性が専用 bin 案と同等であること。
- fixture 解決（`RUVIFORM_L1_REAL_PLUGIN_DIR`）経路を単一化し、
  runner 初期化コードの 2 系統化を避けること。
- Edition 2024 の並列 test 下で env mutation による race を
  持ち込まないこと。

## Considered Options

- 専用 fault bin（`ruviform-plugin-test-*-fault-runner` 等）を
  新設し、`run()` 呼出前に fault を実行。
- 既存 l1-runner bin が `--ruviform-test-fault=MODE` argv を
  `OutOfProcessSpec::args` 経由で受理し、`run()` 前に消費。

## Decision Outcome

既存 l1-runner bin の argv 方式を採用する。`SpawnFault` parse は
bin 層に閉じ、`run()` 本体は無変更。未認識 argv（未知の fault
値を含む）は `run()` の usage rejection に落ちる fail-closed
動作となる。Coordinator により受理済み。

### Consequences

- Good: `run()` の契約・grammar が不変。fixture 解決経路が
  単一。per-spawn argv で並列 race なし。bin 数が増えない。
- Bad: l1-runner bin が test-only argv を理解する分、bin の
  責務が「wire runner + test fault harness」に広がる。
  doc comment で test-only 性を明記して緩和。
