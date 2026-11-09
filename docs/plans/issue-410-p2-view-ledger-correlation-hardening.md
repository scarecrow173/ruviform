---
id: "PLAN-034Uwdhmr1M7BRW1PnLrbn"
type: "plan"
status: "approved"
title: "Issue #410 P2 — view ledger correlation hardening"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/410"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-410-p2-view-ledger-correlation-hardening.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/view.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/behavior.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-410-p2-view-ledger-correlation-hardening.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-410-p2-view-ledger-correlation-hardening.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-410-p2-view-ledger-correlation-hardening.md"
    - "docs/designs/issue-410-p2-view-ledger-correlation-hardening.md"
  derived-by:
    - "docs/tasks/issue-410-p2-view-ledger-correlation-hardening.md"
    - "docs/test-specs/issue-410-p2-view-ledger-correlation-hardening.md"
  verifies:
    - "docs/test-specs/issue-410-p2-view-ledger-correlation-hardening.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #410 P2 — view ledger correlation hardening Implementation Plan

## Goal

`ViewHost` に `control_id` 別 projection revision floor を導入し、
`unmount` → `mount` を跨ぐ古い projection の誤適用を閉じる。あわせて
slice 1（PR #502）で実装済みの bounded retention・outbox 上限・
512-cap 退役・owner dedup が Issue #410 受入条件 5 行を満たすことを
対応 test で検証・固定する。

## Scope

単一の実装ストリーム: `crates/ruviform-ui` の `view.rs` /
`behavior.rs` 内の correlation floor 実装と対応 unit test。
`ruviform-ui-test`/`ruviform-ui-gpui`/driver/consumer は変更しない
（公開 API signature・`StateProjection`/`Revision` の型は維持）。

## Upstream Documents

- `docs/specs/issue-410-p2-view-ledger-correlation-hardening.md`
- `docs/designs/issue-410-p2-view-ledger-correlation-hardening.md`（approved）
- `docs/specs/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md`（slice 1 spec）
- `docs/specs/issue-387-component-view-specs.md`（親 contract）

## Design Inputs

- `docs/designs/issue-410-p2-view-ledger-correlation-hardening.md`

## File Map

- `crates/ruviform-ui/src/behavior.rs` — `on_projection` の
  revision 比較を `pub(crate) fn revision_advances` に抽出し、
  `CommandBinding::on_projection` から利用。対応 unit test は既存
  guard の回帰として維持。
- `crates/ruviform-ui/src/view.rs` — `ViewHost` に
  `projection_floors: HashMap<ControlId, Revision>` を追加し、
  `apply_projection` で floor 検査・受理時更新を実装。
  `unmount`/`detach`/`update` では clear しないことをコメントで
  明示。correlation 契約を doc comment に記述。epoch 境界の
  回帰 unit test を追加。
- `docs/test-specs/issue-410-p2-view-ledger-correlation-hardening.md`
  — 保証すべき性質の正本。

## Implementation Sequence

- [x] `behavior.rs`: `revision_advances(prev, next)` を追加し、
  `on_projection` の advances 判定を置き換える。
- [x] `view.rs`: `projection_floors` field・初期化・
  `apply_projection` の floor 検査/更新・doc comment 更新。
- [x] `view.rs`: unit test 追加 —
  remount 後 stale revision reject + newer accept、
  detach→attach またぎの順序逆転 reject、detached 中の drop、
  `deliver_event`（Notify）経路での floor 強制、
  floor が棄却 projection で巻き戻らないこと、
  control_id 別 floor の独立性。
- [x] slice 1 対応 test の棚卸し（`command_index_*`・
  `outbox_*`・`event_backlog_drop_*`・`duplicate_command_owner_id_*`・
  `tab_alias_*`・`alias_invocation_*`）— 受入条件との対応を
  test-spec に記録し、不足があれば補う。
- [x] `mise run fmt-check` → `check` → `lint` → `test` →
  `test-rt` → Rust 側 `test-ui-*` lane で検証（exit 0。
  `ui/*` TypeScript lane は本 crate 非依存で NotRun —
  impl record Validation 参照）。

## Dependencies and Risks

- 依存: なし（blocked by 無し）。slice 1（PR #502）は main 統合済み。
- リスク: consumer の revision リセットで floor が display を
  凍結 → 契約として「revision は attach epoch 内で単調」を明記
  （design Risks #1）。binding 内 guard と同じ失敗型で新規リスク
  ではない。
- `binding_mut()` 経由の迂回は canonical entry point 契約で
  文書化（design Contract 節）。

## Task Breakdown

- 単一 task: `docs/tasks/issue-410-p2-view-ledger-correlation-hardening.md`
  （floor 実装 + test は一つの論理変更として不可分 — 同一ファイル・
  同一 contract の hardening であり分割すると中間状態が不整合に
  なる）。

## Verification

- [x] `mise run fmt-check` exit 0
- [x] `mise run check` exit 0
- [x] `mise run lint` exit 0
- [x] `mise run test` exit 0（`ruviform-ui` unit test を含む）
- [x] `mise run test-rt` exit 0
- [x] Rust 側 `mise run test-ui-*` lane exit 0
  （component-view / driver-contract / parity / native-slot /
  golden / scenario-contract / gpui / consumer-observation /
  test-msrv / webview。`ui/*` TS package lane は非依存で NotRun）
- [x] 新規 unit test で epoch 境界の誤適用防止を固定

## Open Questions / Gaps

- envelope epoch segment（wire-protocol）は deferred のまま。
  floor は先行実装であり、envelope 確定後も防御的多層として残せる。
- dead-generation `PendingRecord` zombie・reentrant-borrow nit は
  対象外（spec Deferred Design Concerns 参照）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（本 worktree の Worker が
  実施。独立 stream 無し）。
- Independent work streams: none
- User approval required before dispatch: no（Worker dispatch 内で
  実行）

## Review Handoff

- reviewer focus: floor の更新タイミング（受理時のみ・棄却で
  巻き戻らない）、`unmount`/`detach`/`update` で floor を clear
  しないこと、`revision_advances` が `on_projection` と同一規則
  であること、epoch 境界 test が remount/detach の両経路を覆う
  こと、slice 1 test との整合。
