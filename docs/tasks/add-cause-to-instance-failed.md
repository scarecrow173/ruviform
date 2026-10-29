---
id: "TASK-034OpsgIJWNistmrGfoPXD"
type: "task"
status: "done"
title: "T3. InstanceFailed に cause field + variant 非排他（全呼出サイト更新）"
created: "2026-10-29"
updated: "2026-10-30"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/121"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/composite.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/supervisor.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-clap/tests/l1_real_adapter_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/tests/l1_real_adapter_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_video_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_supervisor_e2e.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-121-l1-diagnostics.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-121-l1-diagnostics.md"
    - "docs/tasks/add-failure-cause-and-spawn-failed.md"
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
    - "docs/test-specs/issue-121-l1-failure-cause-notification-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# T3. InstanceFailed に cause field + variant 非排他（全呼出サイト更新）

## Work

`composite.rs` の `HostNotification::InstanceFailed` に `cause: FailureCause` field を追加し、variant レベル `#[non_exhaustive]` を付与（`SupervisionEvent` 慣例 — `supervisor.rs:217-297`）。全構築サイトで cause を供給 — この時点では分類配線前なので暫定値 `FailureCause::Internal` でよい（T7 で実分類に置換）。`lib.rs` の re-export リストに `FailureCause` を追加（`HostError`/`ChildExit` と同列 — 公開 `cause` の型を downstream が命名可能に）。

完全 pattern `{handle, exit}` は source break — 5 ファイルを `..` 付与で対応: `l1_real_adapter_e2e`（clap/vst3 両方・**`#![cfg(feature="host")]` ゲートで既定 target 外**）・`l1_video_e2e`・`l1_supervisor_e2e`・`l1_proxy_e2e`（`plugin_graph_e2e.rs:428` は既に `..` 使用で無変更）。変更サイト: `composite.rs` 構築 ~1165（cause 供給）・完全 pattern destructure ~2645（`..` 付与）・`supervisor.rs` 合成 ~1328（`:933` の `{handle, ..}` は無影響）。**`composite.rs:~2983` の `InstanceFailed{handle: reported, ..}` は既に `..` 付きの destructure で無変更** — 「構築×3」ではなく 構築×1+destructure×1+合成×1 が実数。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-121-l1-diagnostics.md`
- `docs/designs/issue-121-l1-diagnostics.md`（「公開 API 面」節）

## Dependencies

- `docs/tasks/add-failure-cause-and-spawn-failed.md`（`FailureCause` 型の前提）

## Affected Files

- `crates/ruviform-plugin-host/src/composite.rs`・`supervisor.rs`・`lib.rs`
- `crates/ruviform-plugin-{clap,vst3}/tests/l1_real_adapter_e2e.rs`（cfg ゲート配下）
- `crates/ruviform-plugin-host/tests/{l1_proxy_e2e,l1_video_e2e,l1_supervisor_e2e}.rs`

## Done When

- [ ] `InstanceFailed{handle, exit, cause}` が `Copy` 維持 + variant 非排他で追加されている
- [ ] 全完全 pattern が `..` 付与済み・全構築サイトが `cause` 供給済み
- [ ] `lib.rs` が `FailureCause` を re-export
- [ ] `host` feature lane を含め全 consumer が compile 通過（crate 境界 compile の承認条件）

## Verification

- [ ] `mise run check` exits 0
- [ ] `mise run lint` exits 0
- [ ] `mise run lint-l1-real-plugin` exits 0 — l1_real 両ファイルの compile をカバーする唯一の lane
