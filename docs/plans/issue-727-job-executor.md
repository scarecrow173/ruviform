---
id: "PLAN-034Z0ufhfy1zqe3TvK5xrp"
type: "plan"
status: "approved"
title: "Issue #727 bounded 非 RT job executor — 責務判断決裁・API 設計・検証"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/727"
    - "https://github.com/scarecrow173/synvelis/issues/518"
  changes:
    added:
      - type: "doc"
        path: "docs/plans/issue-727-job-executor.md"
      - type: "doc"
        path: "docs/tasks/issue-727-job-executor.md"
      - type: "doc"
        path: "docs/test-specs/issue-727-job-executor.md"
      - type: "doc"
        path: "docs/impl/ir/issue-727-job-executor.md"
      - type: "doc"
        path: "docs/impl/exp/issue-727-job-executor.md"
      - type: "file"
        path: "docs/impl/exp/issue-727-job-executor-exp.jsonl"
      - type: "file"
        path: "evidence/rf727/"
    modified:
      - type: "file"
        path: "docs/designs/shared-foundations-and-modularization-policy.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-727-job-executor.md"
  implemented-by:
    - "docs/tasks/issue-727-job-executor.md"
  depends-on:
    - "docs/designs/issue-727-bounded-nonrt-job-executor.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-721-post-seal-temp.md"
    - "docs/plans/issue-716-runtime-backend.md"
    - "docs/tasks/issue-727-job-executor.md"
    - "docs/test-specs/issue-727-job-executor.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-727-job-executor.md"
    - "docs/designs/issue-727-bounded-nonrt-job-executor.md"
  derived-by:
    - "docs/tasks/issue-727-job-executor.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-727-job-executor.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/727"
    - "https://github.com/scarecrow173/synvelis/issues/518"
    - "https://github.com/scarecrow173/ruviform/issues/155"
    - "https://github.com/scarecrow173/ruviform/issues/24"
    - "https://github.com/scarecrow173/ruviform/issues/316"
  defers: []
  deferred-by: []
metadata:
  issue: 727
---

# Issue #727 bounded 非 RT job executor — 責務判断決裁・API 設計・検証

## Goal

bounded 非 RT job executor primitive の責務判断を
「Ruviform `ruviform-exec` module 追加」で決裁し、ADR・
API 設計 doc・stale 表記の最小修正・検証証拠として確定する
docs-only slice。実装コードは作成しない。

## Scope

単一 implementation stream。内部文書チェーン + shared-
foundations 1 行修正 + evidence のみで、並列分割の必要はない。

## Upstream Documents

- `docs/specs/issue-727-job-executor.md`

## Design Inputs

- `docs/designs/issue-727-bounded-nonrt-job-executor.md`
  （approved — `ExecutorConfig`/`JobExecutor`/`QueuedJob`/
  `SubmitError`/`ExecutorError` の API 契約）
- `docs/adr/issue-727-bounded-nonrt-job-executor-placement.md`
  （accepted — Option 1: ruviform-exec module 追加）

## File Map

- `docs/specs/issue-727-job-executor.md` — 要求・受入条件・
  deferred concern の正本。
- `docs/designs/issue-727-bounded-nonrt-job-executor.md` —
  API 契約・bounded/cancel/preemption/RT 分離の設計正本。
- `docs/adr/issue-727-bounded-nonrt-job-executor-placement.md` —
  責務判断の決裁記録（sv#518 から参照可能）。
- `docs/plans/issue-727-job-executor.md` — 本文書。
- `docs/tasks/issue-727-job-executor.md` — 実施 task 記録。
- `docs/test-specs/issue-727-job-executor.md` — 文書契約・
  整合性の検証規範。
- `docs/designs/shared-foundations-and-modularization-policy.md`
  — L282 の stale 行番号参照と ADR 参照の最小修正（1 行）。
- `evidence/rf727/` — baseline/事後の doc_status lint・
  route_graph・fmt/prek の実行証拠。
- `docs/impl/ir/issue-727-job-executor.md`・
  `docs/impl/exp/issue-727-job-executor.md` +
  `issue-727-job-executor-exp.jsonl` — 実装記録・実験記録。

## Implementation Sequence

- [x] `crates/ruviform-exec` 既存 primitive の契約確認
      （`lib.rs`/`job.rs`/`offline.rs`/`README.md` — read-only）
- [x] consumer 要件の照合（synvelis `orchestrator.rs` の
      `RenderExecutor` port・`run()`/`drive()`・
      `CancellationToken` 使用、audio-lens `al-job` —
      read-only 参照）
- [x] baseline・shared-foundations・synvelis-driven 実行境界
      ADR・issue-155/issue-716 の判断材料確認
- [x] 責務判断の決裁（Ruviform `ruviform-exec` module 追加）
- [x] spec/design/ADR/plan/task/test-spec/impl/exp の文書
      チェーン完成
- [x] `shared-foundations-and-modularization-policy.md` L282
      の stale 行番号参照を最小修正 + ADR 参照追加
- [x] `doc_status` lint 前後差分・`route_graph`・`prek`・
      `mise run fmt-check` の実測と `evidence/rf727/` 記録

## Dependencies and Risks

- **route_graph baseline blocked**: main 上に既存の
  `broken-relation:*` が多数あり `probe` で blocked を返す
  baseline 状態。新規文書が新たな finding を追加しないことを
  前後差分で確認する（baseline は `evidence/rf727/` に記録）。
- **docs index 未同期**: `docs/**/README.md` は Coordinator が
  統合時に再生成するため、本 slice では編集しない。
- **API 契約の実装時確定事項**: queue 具象型・result 配送・
  `JobState` queued 表現・worker naming/priority は実装
  Issue で確定（設計 doc に Deferred として記録済み）。

## Task Breakdown

- `docs/tasks/issue-727-job-executor.md` — 単一 task
  （責務判断決裁・API 設計・検証）。並列分割なし。

## Verification

- [x] `node .agents/skills/doc-status/scripts/doc_status.js lint
      --dir docs/<type>`（adr/designs/specs/plans/tasks/
      test-specs/discovery/impl）— baseline 採取済み、
      新規文書由来の finding がないことを差分で確認
- [x] `node .agents/skills/doc-driven-dev-graph/scripts/
      route_graph.js --json` — baseline blocked を記録、
      新規文書追加後も同等以下であることを確認
- [x] `node .agents/skills/adr-doc/scripts/review_adr.js
      --dir docs/adr`・`audit_adr.js`・`check_code_links.js`
      — 実行して結果を記録
- [x] `mise run fmt-check`（`cargo fmt --all -- --check`）
- [x] `mise exec -- prek run --files <変更ファイル>`
      （trailing-whitespace/end-of-file/mixed-line-ending 等）

## Open Questions / Gaps

- `JobState` に queued 状態を追加するか `Running` で報告する
  か — 実装 Issue で確定（契約上は区別可能であることのみ要求）。
- blocking `submit` の worker-thread reentrancy — contract で
  禁止を明記済み。`try_submit` 併用で回避可能。
- baseline 本文の「ワーカースレッドプール」表記改訂 —
  baseline 改訂 Issue に委ねる（本 slice は改訂要否の結論
  のみ記録）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology
      are consistent.

## Implementation Handoff

- Recommended mode: inline execution（docs-only slice のため
  実装委譲なし）。
- Independent work streams: none.
- User approval required before dispatch: yes（review-protocol
  で blocker なしを確認済みとして扱う）。

## Review Handoff

- ADR の Option 3（consumer 側実装）却下理由が
  synvelis-driven 実行境界 ADR Decision 6 と整合しているか。
- 設計 doc の API 契約が `OfflineJob` の既存 surface と
  矛盾しないか。
- `JobState` queued 表現の未確定事項が contract 上問題ないか。
