---
id: "IMPL-034Qe184TypedGraphIrPoC"
type: "impl"
status: "completed"
title: "Issue #184 M1 ruviform-graph crate: typed Graph IR + Validator + PoC"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/184"
  changes:
    added:
      - type: "crate"
        path: "crates/ruviform-graph/"
      - type: "file"
        path: "docs/impl/ir/issue-184-typed-graph-ir-poc.md"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
  implements:
    - "docs/tasks/t-1-ruviform-graph-crate-skeleton-ids-port-graph-types-builder.md"
    - "docs/tasks/t-2-validator-stablehandlemap-graphproject.md"
    - "docs/tasks/t-3-poc-ex-001-006-index-lockfile-full-verification-evidence.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-184-m1-ruviform-graph-crate-typed-ir-validator-poc.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-184-typed-graph-ir-ports-buses-connections-validation-revisions-m1.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-184-m1-typed-graph-ir-representation-multi-bus-topology-validation-failure.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #184 M1 ruviform-graph 実装記録

## 実装内容

`crates/ruviform-graph` を新設（構造層・通常依存 `ruviform-audio-core` のみ）。
`GraphBuilder`（u64=1 起点の決定的 ID 採番・private `GraphSpec`）、orthogonal port
属性（`PortFormat::domain()` が domain の唯一の正）、`Validator`（referential →
direction → domain → format 階層 → layout/`ChannelMap` → capacity → required →
全グラフ SCC cycle 判定、`ValidationErrors` 全件収集）、`StableHandleMap`
（Kahn + `BTreeMap` tie-break の deterministic topo）、`GraphProject`
（requested/published revision 区別・失敗時 published 維持）を実装。

設計との相違点（意図的）:
- 通常依存は `ruviform-audio-core` のみ。`ruviform-media-core` は crate-local
  error 型採用により不要となり直接依存しない（推移的にのみ到達）。
- `LatencyStatus` は M1 では未導入（capability 記録のみ・runtime 意味は M2+）。
- 実測した `ChannelLayout::channels()` 戻り型は `u32`（設計想定と一致）。
- `AudioFormat` は `PartialEq`/`Eq`/`Hash` 実装済み — field 比較は不要だった。

## 検証結果（実行済み）

| コマンド | 結果 |
| --- | --- |
| `mise exec -- cargo test -p ruviform-graph --test poc_ex` | exit 0 — ex_001〜ex_006 全 6 件 pass |
| `mise exec -- cargo tree --locked -p ruviform-graph --edges normal` | `ruviform-audio-core` のみ（media-core は推移的）・循環 0 |
| `mise run fmt-check` | exit 0 |
| `mise run check` | exit 0 |
| `mise run lint` | exit 0 |
| `mise run test` | exit 0（workspace 全緑・0 failed） |
| `mise run test-rt` | exit 0（0 FAILED） |
| `rg "ruviform_rt\|RtContext\|unsafe" crates/ruviform-graph/{src,tests,Cargo.toml}` | 0 match |
| `git fetch origin && git merge origin/main` | merge commit 818e88e（index 衝突は union 解消・再生成） |
| `mise exec -- cargo generate-lockfile` | merge 後に再生成・DAG 再確認済み |

merge 後に fmt-check/check/lint/test/test-rt/poc_ex を再実行し全て exit 0。

## 未検証項目

- CI 実行（repo の CI は workflow_dispatch 手動のみ — 未トリガ）。
- M2 以降の compiler/runtime 統合（本 migration 範囲外）。
- serde/永続化・delayed-cycle 実行（M1 非対象として設計で明示）。
