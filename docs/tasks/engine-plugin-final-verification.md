---
id: "TASK-034Mrt2jiucAb69qBSYoEg"
type: "task"
status: "todo"
title: "engine-plugin 統合 final verification + closeout（Full Gate・AC-to-test 証跡・status 同期）"
created: "2026-10-26"
updated: "2026-10-26"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-113-engine-plugin-integration.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-113-engine-plugin-integration.md"
    - "docs/tasks/plugin-graph-e2e-and-rt-lane.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-113-engine-plugin-integration.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/plugin-endpoint-contract-tests.md"
    - "docs/test-specs/plugin-graph-e2e-tests.md"
    - "docs/test-specs/plugin-rt-retirement-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# engine-plugin 統合 final verification + closeout（Full Gate・AC-to-test 証跡・status 同期）

## Work

Plan actions 20–21。全実装 task（T1–T6）完了後に 1 回のみ実行する canonical closeout — **Full Gate**（全 mise 検証列 + doc audit + DAG 監査）と **closeout**（実績 IR・coverage 記述・task/plan/test-spec status 同期・正本 doc の実績同期）。本計画は planning artifact のみ — 実装・commit・GitHub merge は lead の authoritative `gh` が管理する（本 task が実装後に動く前提の記述であり、planning 時点では実行しない）。

- **Full Gate（1 回のみ・本計画最後段）**:
  - `mise run fmt-check`
  - `mise run check`
  - `mise run lint`
  - `mise run test`
  - `mise run test-rt` — **実一致件数を記録**（`rt_endpoint_*`/`rt_plan_*`/`rt_fn_node_*` + 既存 `rt_` — 0 件一致スキップを成功と偽らない・実件数が期待と一致するか確認）
  - `mise run bench-smoke`
  - `mise exec -- prek run --all-files`
  - `mise exec -- node .agents/skills/doc-status/scripts/doc_status.js audit --json` — doc 監査チェーン起因 findings 0
  - `mise exec -- cargo tree -p ruviform-audio` — DAG に `ruviform-plugin-*` が出現しない（AC3 — `ruviform-audio` の公開 API・`[dependencies]` に plugin 型・plugin 依存がないことを機械監査）
  - `mise exec -- cargo doc -p ruviform-plugin-host -p ruviform-audio --no-deps` — **zero warnings**（endpoint/re-export・crate doc・getter の rustdoc が warning なく構築される — `-D warnings` 相当の確認を log に記録）
  - 全 log を `docs/impl/exp/data/issue-113-final-gate-*-*.log` へ保存（タイムスタンプ・exit code 付き）
  - CI `verify` ジョブ（3 OS matrix・`workflow_dispatch`）での `cargo test --workspace`（E2E 含む）実績は lead が記録 — ローカルは Windows 実証のみ（macOS/Linux は CI 実行時のみ記録・planning 側は未実行の主張をしない）
- **canonical closeout**:
  - 実績 IR（`docs/impl/ir/issue-113-*.md`）— 実装の構造・設計との差異・pinned mechanism の実現確認
  - coverage 記述 — 11 AC ↔ actions ↔ test-spec ↔ 実 test 名の写像表（AC-to-test 証跡 — AC1↔`plugin_graph_e2e` L0/L1・AC2↔`expect_send`・AC3↔`cargo tree`・AC4↔`prepare_endpoint` rejection・AC5↔`Failed`/pass-through/`fill(0.0)`・AC6↔`latency_frames`・AC7↔`rt_endpoint_*`/`rt_plan_*`・AC8↔DropFlag+`retired_overflows`・AC9↔E2E・AC10↔crate doc・AC11↔本 gate）
  - task status 同期 — T1–T7 を実装・検証実績に基づき `done` へ（planning 時点では `todo` のまま — 実装・gate 実績なしに status を変えない）
  - plan status — 実装・gate 全通過後に `done`（planning 時点では `approved` のまま）
  - 3 test-spec を実装・実結果に基づき `approved` へ（planning 時点では `proposed` のまま — 実績なしに status を変えない）
  - design/spec doc の実装相違は正本へ同期反映 — `composite.rs` と `lib.rs`（L44-49 付近の crate doc `Latency contract (L1)` 節）**両方**の旧遅延式（`× block_frames / sample_rate`）が修正版式（`lookahead × activated max_frames`）へ同期済みであることを確認（T4 で実施 — stale parity の fix site は 2 箇所）・その他実装と design の実績差異があれば design 側に記録
  - GitHub merge・issue close は lead の authoritative `gh` のみ（subagent は行わない）

## Classification

- `normal-plan-task`（final gate + closeout — T1–T6 全完了が前提）

## Upstream Documents

- `docs/plans/implement-issue-113-engine-plugin-integration.md`（actions 20–21・Verification・Rollback）
- `docs/designs/engine-plugin-host-integration.md`（全 pinned mechanism の実現確認）
- `docs/specs/engine-plugin-host-integration.md`（AC1–AC11 — AC-to-test 証跡の正本）

## Dependencies

- `docs/plans/implement-issue-113-engine-plugin-integration.md`
- `docs/tasks/plugin-graph-e2e-and-rt-lane.md`（T6 — E2E/`rt_` レーン完了が前提。T1–T5 は推移的に全て前提）

## Affected Files

- `docs/impl/ir/issue-113-*.md`（新規 — 実績 IR・coverage 記述）
- `docs/impl/exp/data/issue-113-final-gate-*-*.log`（新規 — Full Gate 全 log）
- `docs/tasks/plugin-core-endpoint-primitives.md`・`l0-shared-slot-and-endpoint-inner.md`・`l1-endpoint-arm-and-leg-return.md`・`plugin-host-endpoint-surface.md`・`audio-fnmut-node-and-plan-retirement.md`・`plugin-graph-e2e-and-rt-lane.md` — status `done` への同期（実装・検証実績に基づく）
- `docs/plans/implement-issue-113-engine-plugin-integration.md` — status `done` への同期
- `docs/test-specs/plugin-endpoint-contract-tests.md`・`plugin-graph-e2e-tests.md`・`plugin-rt-retirement-tests.md` — status `approved` への同期（実績に基づく）
- `docs/designs/engine-plugin-host-integration.md`・`docs/specs/engine-plugin-host-integration.md` — 実装相違の正本反映（必要に応じ）
- `docs/tasks/README.md`・`docs/test-specs/README.md`・`docs/plans/README.md` — status 反映（index 整合）

## Done When

- [ ] Full Gate 全コマンド（`fmt-check`/`check`/`lint`/`test`/`test-rt`/`bench-smoke`/`prek`/doc audit/`cargo tree` DAG 監査）が exit 0 — 全 log が `docs/impl/exp/data/` に保存される
- [ ] `test-rt` が実一致件数で証明される（`rt_endpoint_*`/`rt_plan_*`/`rt_fn_node_*` の実件数が記録され 0 件スキップでない）
- [ ] `cargo tree -p ruviform-audio` の DAG に `ruviform-plugin-*` が出現しない（AC3 — `mise run test` 成功だけでは検出しないため明示監査）
- [ ] `cargo doc -p ruviform-plugin-host -p ruviform-audio --no-deps` が zero warnings で構築される（rustdoc warning を記録 — 警告を黙殺しない）
- [ ] AC-to-test 証跡表が記録される（11 AC ↔ actions ↔ test-spec ↔ 実 test 名）
- [ ] task T1–T7・plan・3 test-spec の status が実装・検証実績に基づき同期される（planning 時点では `todo`/`approved`/`proposed` のまま — 実績なしに変えない）
- [ ] design/spec doc の実装相違が正本へ反映される（`composite.rs` + `lib.rs`（L44-49 付近 crate doc）両方の遅延式・その他差異）
- [ ] doc audit が findings 0
- [ ] GitHub merge/issue close が lead の `gh` に委譲される（本 task では行わない）

## Verification

- [ ] `mise run fmt-check` — exit 0
- [ ] `mise run check` — exit 0
- [ ] `mise run lint` — exit 0
- [ ] `mise run test` — exit 0
- [ ] `mise run test-rt` — exit 0・実一致件数記録（`rt_endpoint_*` 4 本 + `rt_plan_*` 3 本 + `rt_fn_node_*` + 既存 `rt_` — 実件数が期待と一致）
- [ ] `mise run bench-smoke` — exit 0
- [ ] `mise exec -- prek run --all-files` — exit 0
- [ ] `mise exec -- node .agents/skills/doc-status/scripts/doc_status.js audit --json` — findings 0
- [ ] `mise exec -- cargo tree -p ruviform-audio` — DAG に `ruviform-plugin-*` なし（AC3）
- [ ] `mise exec -- cargo doc -p ruviform-plugin-host -p ruviform-audio --no-deps` — zero warnings（rustdoc warning なし・log 記録）
- [ ] CI `verify` ジョブ（3 OS matrix）で `cargo test --workspace` — lead が記録（ローカルは Windows のみ・3 OS は CI）
- [ ] 対象 test-spec 3 件（`plugin-endpoint-contract-tests`/`plugin-graph-e2e-tests`/`plugin-rt-retirement-tests`）— 全規則が実 test に写像される
