---
id: "TESTSPEC-XADte5w8EHAalnm33WCVyn"
type: "test-spec"
status: "draft"
title: "Issue #404/#405 NativeSurfaceSlot host 境界修復 test 仕様"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/404"
    - "https://github.com/scarecrow173/ruviform/issues/405"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/surface.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-404-405-native-surface-host-boundary.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-388-389-native-dock-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-404-405-native-surface-host-boundary.md"
  derived-by: []
  verifies:
    - "docs/designs/issue-404-405-native-surface-host-boundary.md"
    - "docs/plans/implement-issue-404-405-native-surface-host-boundary.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  issues: [404, 405]
---

# Issue #404/#405 test 仕様

`mise run test-ui-gpui`（`cargo test -p ruviform-ui-gpui --features
gpui`、実 Windows session）で実行する
`#[cfg(all(test, windows))] mod tests`。test は gpui `Application`
を起動しない — `GpuiDriver::new()` + `create_window`（`windows`
map への挿入のみ）+ `inner.windows[id].raw_parent` への非表示
test HWND 注入で adapter の全経路を実 HWND 上で検証する。
`SetWindowRgn`/`GetWindowRgn`/`GetRgnBox`/`MoveWindow`/
`DestroyWindow`/`IsWindow` は message pump を要しない同期 API。

## Feature: clip 解除と再投影（Issue #404）

### Rule: `set_clip(None)` は OS region を解除する

- 例1: `Container::create`（test parent 下）→ `set_clip(Some)` →
  `GetWindowRgn` が region を返す → `set_clip(None)` →
  `GetWindowRgn` が 0（region なし）を返す。[AC-404-1]
  → `clip_clear_removes_os_region`
- 例2: `GpuiSurfaceAdapter` 経路 — declare+lease →
  `set_clip(Some(C))` → `set_clip(None)` → container HWND の
  `GetWindowRgn` が 0。[AC-404-1]
  → `geometry_move_reprojects_stored_clip` 末尾の None 検証。

### Rule: `set_geometry` は保持 clip を新 geometry で再投影する

- 例3: `set_clip(Some(C))` → `set_geometry(G2)`（origin 変更）→
  `GetRgnBox` が `clip_local(C, G2)` と一致（旧 `clip_local(C,G1)`
  ではない）。[AC-404-2] → `geometry_move_reprojects_stored_clip`
- 例4: clip 未設定（`None`）で `set_geometry` → region が設定
  されないまま移動する（副作用なし）。
  → `geometry_move_without_clip_leaves_no_region`

### Rule: OS 失敗時に ledger が先行しない

- 例5: `set_clip` は container 適用成功後にのみ `binding.clip`
  を記録する — code review で順序を確認 + `lease_parent`
  re-apply 失敗が `Err` + lease rollback（`validate_lease` が
  `LeaseRevoked`）であること。[AC-404-3]
  → `clip_declared_before_lease_applies_on_attach`（re-apply 側）・
  失敗経路は独立 review の code 検証で担保（`SetWindowRgn` 失敗を
  誘発する test 手段なし — `CreateRectRgn` 失敗・hwnd 死亡は
  sweep が先行排除する）。

## Feature: owner window teardown の adapter 伝播（Issue #405）

### Rule: owner destroy は slot を終端化する

- 例6: declare+lease → parent `DestroyWindow`（OS 事実: container
  も死亡）→ `crate::gpui_driver::retire` → `drain_events` が
  `LeaseRevoked`+`Destroyed`・`state(slot)` が
  `Some(Released)`・`container_token` が `None`・`bindings` から
  除去。[AC-405-1/3] → `owner_teardown_auto_releases_slot`
- 例7: declared-only（lease なし）slot の owner destroy →
  `Destroyed` のみ（`LeaseRevoked` なし）。[AC-405-1]
  → `unretired_owner_death_detected` の `declared` slot。

### Rule: 死亡 slot への op は typed error

- 例8: owner destroy 後 `set_geometry`/`set_visibility`/`set_clip`/
  `set_zorder`/`request_focus`/`lease_parent`/`mark_busy`/
  `clear_busy`/`release_lease`/`release_slot` → `Ok` を返さない
  （`Released`/`StaleGeneration`/`LeaseRevoked`/`Backend`）。[AC-405-2]
  → `owner_teardown_auto_releases_slot` の op 一覧 assertion
  （`mark_busy`/`clear_busy` は registry 側 `Released` 検証で担保 —
  adapter は sweep 後に透過委譲のみ）。
- 例9: `redeem(lease)`・`validate_lease(lease)` →
  `LeaseRevoked`。[AC-405-3] → `owner_teardown_auto_releases_slot`。

### Rule: 収束と回帰

- 例10: `release_slot`/`release_lease` を auto-release 後に呼ぶ →
  typed error（偽 `Ok` でない）。repeated `retire`/`destroy` →
  event 重複なし。
  → `consumer_release_after_auto_release_errors` /
  `repeated_teardown_is_idempotent`
- 例11: dead slot index は次回 `declare_slot` で generation++
  再利用。旧 id の op は `StaleGeneration`/`Released`。[AC-405-4]
  → `auto_released_index_recycles_generation`
- 例12: 2 window — W1 の slot は W2 destroy で影響を受けない。
  → `teardown_of_other_window_leaves_slot_live`
- 例13: 同一 driver に 2 adapter — 双方の ledger が独立に収束。
  → `two_adapters_reconcile_independently`
- 例14: parent `DestroyWindow` で container HWND が
  `IsWindow()==false`（OS 事実自体を別途確認 — adapter 評価と
  混同しない）。
  → `container_death_on_live_owner_revokes_lease_only` の
  `os_window_alive` assertion。

## Feature: backend 不変条件

- 例15: `lease_parent` は `IsWindow(parent)==false` の stale
  parent に `Backend` で失敗し lease が残らない
  （`validate_lease` → `LeaseRevoked`）。
  → `create_under_dead_parent_is_rejected`（container 層）+
  `lease_under_dead_owner_is_rejected`（adapter 層 — dead parent
  検出は owner death として `Released` に終端化するため、
  ledger に lease/record は残らない）。
- 例16: `destroyed` 未記録の経路（`retire` 未呼出）でも
  `container.is_alive()==false` を検出して sweep が収束する
  （pre-retire/driver-exit gap 補完 — test parent 破棄のみで
  `retire` を呼ばないケース）。
  → `unretired_owner_death_detected`（raw_parent probe 経由 —
  declared-only を含む）。
- 例17: live owner 上で container のみ死亡 → `LeaseRevoked`+
  `Declared`（`Destroyed` なし）→ 再 `lease_parent` 可能。
  → `container_death_on_live_owner_revokes_lease_only`

## 検証しないもの（NotRun/Unsupported 記録）

- macOS/Linux 実機 — stub backend・`Unsupported` 経路不変。
- 実 gpui `Application`/`Window` 経由の end-to-end destroy —
  `retire` を module 内から直接呼出して同じ終端遷移を再現する
  （`retire` は全破棄経路の唯一の終端点 — issue 診断で確認済み）。
- 実外部 VST3/Dock 受入（別 Gate・本 fix の成功と混同しない）。
