---
id: "PLAN-fUxIG0DACT0SXtitvEBE3q"
type: "plan"
status: "approved"
title: "Issue #311 RF-GT-02 実装計画 — ui/guitest + Rust conformance lanes"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "docs/specs/issue-311-ipc-conformance-replay.md"
    - "docs/designs/issue-311-ipc-conformance-replay.md"
  changes:
    added:
      - type: "dir"
        path: "ui/guitest"
      - type: "file"
        path: "crates/ruviform-ui/tests/ipc_conformance.rs"
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
    - "docs/specs/issue-311-ipc-conformance-replay.md"
    - "docs/designs/issue-311-ipc-conformance-replay.md"
  implemented-by:
    - "docs/tasks/issue-311-t1-fixture-corpus.md"
    - "docs/tasks/issue-311-t2-ts-wire-validators.md"
    - "docs/tasks/issue-311-t3-recorder-replay-oracle.md"
    - "docs/tasks/issue-311-t4-fault-matrix-lanes.md"
  depends-on:
    - "docs/designs/issue-311-ipc-conformance-replay.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-311-ipc-conformance-replay.md"
  derived-by:
    - "docs/tasks/issue-311-t1-fixture-corpus.md"
    - "docs/tasks/issue-311-t2-ts-wire-validators.md"
    - "docs/tasks/issue-311-t3-recorder-replay-oracle.md"
    - "docs/tasks/issue-311-t4-fault-matrix-lanes.md"
    - "docs/test-specs/issue-311-ipc-conformance-replay.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-311-ipc-conformance-replay.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #311 RF-GT-02 実装計画

対象 spec `docs/specs/issue-311-ipc-conformance-replay.md`、設計 `docs/designs/issue-311-ipc-conformance-replay.md`。

## Scope split / file map

| スライス | 主な成果物 | 依存 |
| --- | --- | --- |
| T1 fixture corpus | `ui/guitest/fixtures/**` + package 骨格（package.json/tsconfig/README）+ `src/fixtureload.ts` | なし |
| T2 TS wire validators | `src/ipcwire.ts`（rvf.ipc.v1 + rvf://data）、`src/bindingwire.ts` + `test/conformance.test.ts` + `test/fixtures.test.ts` | T1 |
| T3 recorder/replay/oracle | `src/jsonl.ts`、`src/recorder.ts`、`src/replay.ts`、`src/oracle.ts`、`src/consumers.ts`、`src/seed.ts` + `test/record.test.ts` + `test/replay.test.ts` | T1 |
| T4 fault matrix + lanes | `test/faultmatrix.test.ts`、`crates/ruviform-ui/tests/ipc_conformance.rs`、`scheme.rs` テスト拡張、mise.toml / ci.yml / AGENTS.md | T1・T2・T3 |

## リスク登録

| リスク | 対応 |
| --- | --- |
| oracle が実装を再実装しすぎて維持負荷になる | 契約観測（dispatch 列・phase・分類）に限定し、内部構造（queue/tombstone/tracked map）を写さない |
| fixture 形式の過剰設計 | case 語彙は棚卸しで観測した契約要素のみ。新形式は `v` で追加 |
| `Destroyed` 等、Rust 側で runtime が産出しない語彙 | runtime の産出語彙と配信層の語彙を分けて scenario 化（`destroy` 入力は「以後の message を配信層が drop」として扱う） |
| webview feature lane がローカルで走らない環境 | NotRun/Unsupported と Fail を分離する報告を IR に残す |
| Windows パス区切り | 両側とも `path`/`Path` API で組み立て、絶対パス・区切り文字を fixture に書かない |

## Rollback

新規パッケージ追加型なので `ui/guitest` ディレクトリ削除 + `crates/ruviform-ui/tests/ipc_conformance.rs` 削除 + `scheme.rs` のテスト差分 revert + mise/ci/AGENTS の 3 ファイル差分 revert で完全に戻る。既存 product コードへの変更は scheme.rs のテストモジュール内に閉じる。

## 検証行列

| 項目 | 入口 | タイミング |
| --- | --- | --- |
| fmt | `mise run fmt-check` | 実装中・最終 |
| Rust 型/lint | `mise run check` / `mise run lint` | 実装中・最終 |
| workspace テスト（ruviform-ui conformance 含む） | `mise run test` | 実装中・最終 |
| TS 型検査 | `mise run check-ui-guitest` | 実装中・最終 |
| TS テスト | `mise run test-ui-guitest` | 実装中・最終 |
| 既存 UI レーン非回帰 | `test-ui-headless` / `test-ui-binding` / `test-ui-components` / `test-ui-viz` / `test-ui-showcase` | 最終 |
| webview feature lane | `check-ui-webview` / `lint-ui-webview` / `test-ui-webview` | 最終（環境依存: Windows は WebView2 必要 — 不可なら NotRun 記録） |
| CI 相当列 | `mise run verify-ci`（時間が許す限り、最低 check-ci/lint-ci/test-ci） | 最終 |

## タスク分解（task docs 参照）

1. `issue-311-t1-fixture-corpus.md` — fixture 形式 v1 + package 骨格 + loader
2. `issue-311-t2-ts-wire-validators.md` — ipcwire/bindingwire + conformance テスト
3. `issue-311-t3-recorder-replay-oracle.md` — jsonl/recorder/replay/oracle/consumers/seed + テスト
4. `issue-311-t4-fault-matrix-lanes.md` — fault matrix + Rust 側 conformance + mise/CI/AGENTS 同期 + IR
