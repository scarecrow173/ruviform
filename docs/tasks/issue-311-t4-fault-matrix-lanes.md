---
id: "TASK-uyCbW614DqUYmATBzUrQxz"
type: "task"
status: "done"
title: "T4 fault matrix + Rust conformance lanes + mise/CI/AGENTS 同期 + IR（Issue #311）"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "ui/guitest/test/faultmatrix.test.ts"
      - type: "file"
        path: "crates/ruviform-ui/tests/ipc_conformance.rs"
      - type: "file"
        path: "docs/impl/ir/t4-rust-conformance-lanes-fault-matrix-ci-wiring-issue-311.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-webview/src/scheme.rs"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-311-ipc-conformance-replay.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-311-t1-fixture-corpus.md"
    - "docs/tasks/issue-311-t2-ts-wire-validators.md"
    - "docs/tasks/issue-311-t3-recorder-replay-oracle.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-311-ipc-conformance-replay.md"
  references: []
  defers: []
  deferred-by: []
---

# T4 fault matrix + Rust conformance lanes + mise/CI/AGENTS + IR

## Work

- `ui/guitest/test/faultmatrix.test.ts`: 設計 §5 の fault matrix を TS 側に網羅実装（terminal event 存在 assert 付き）。
- `crates/ruviform-ui/tests/ipc_conformance.rs`: fixture 読み込み → `parse_intent`/`Event::to_json`/`DataRegistry::read`/`UiRuntime::handle`/`handle_window` 照合。host scenario 再生 + job backpressure/panic/cancel。
- `crates/ruviform-ui-webview/src/scheme.rs`: 既存 `#[cfg(test)]` を fixture 駆動に拡張（`rvf://data` wire 契約）。product コードは変更しない。
- `mise.toml`: `check-ui-guitest`・`test-ui-guitest` 追加。
- `.github/workflows/ci.yml`: `ui-verify` ジョブに 2 ステップ追加。
- `AGENTS.md`: mise タスク表に 2 行追加（既存 UI lane 記述に整合）。
- `docs/impl/ir/t4-rust-conformance-lanes-fault-matrix-ci-wiring-issue-311.md`: 実施検証・未実施（理由付）・セキュリティレビュー・#294 証拠項目を記録。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/designs/issue-311-ipc-conformance-replay.md` §5–§7
- `docs/test-specs/issue-311-ipc-conformance-replay.md`

## Verification

- `mise run fmt-check` / `check` / `lint` / `test` / `check-ui-guitest` / `test-ui-guitest` / 既存 UI lanes / （環境が許せば）webview lanes。
