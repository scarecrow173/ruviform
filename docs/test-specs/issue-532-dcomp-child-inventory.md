---
id: "TSPEC-034ZccDcompChildInv01"
type: "test-spec"
status: "approved"
title: "Issue #532 dcomp-child-inventory — residual class 確定記録（WS_EX_LAYERED 適用不能の確定診断）"
created: "2026-11-21"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-532-dcomp-child-inventory.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/test-specs/issue-532-child-inventory.md"
    - "docs/specs/issue-532-dcomp-residual.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-532-child-inventory.md"
    - "docs/specs/issue-532-child-inventory.md"
    - "docs/discovery/issue-532-child-inventory.md"
    - "docs/specs/issue-532-dcomp-residual.md"
    - "docs/discovery/issue-532-dcomp-residual.md"
    - "docs/test-specs/issue-532-dcomp-residual.md"
    - "evidence/rf532/child-inventory-results.md"
    - "evidence/rf532/residual-disposition.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-532-dcomp-residual.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-532-dcomp-residual.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  issue: 532
---

# Issue #532 `real-dcomp-childinv` — residual class 確定記録

`real-dcomp-childinv` case（child-level inventory + capture-plane
診断 + layered/reparent retry）について、根治 fix（Issue #839）
統合後の pinned-dep run で記録された `fail` の class 確定と処遇を
記録する。case 自体の検証契約は姉妹文書
`docs/test-specs/issue-532-child-inventory.md` が正本であり、本文書
はその verdict の **residual class** を対象とする。

## 確定した class: C — `WS_EX_LAYERED` 適用不能の確定診断

本 case が child-level で問うた「`WS_EX_LAYERED` を docked child
HWND へ適用できるか」は、実走により **child-window-state 制約で
適用不能** と確定した。

- `ci_apply_layered_exstyle`: child（caller thread 所有、
  same-thread write が可能なはず）への `GWL_EXSTYLE` write が
  **silent refusal**（`applied: false`、`win32_error: null`）。
- `ci_apply_layered_lwa`: error 87（container-level と同一）。
- retry 実験: `latched_as_child = false` /
  `latched_as_toplevel = true` — `SetParent(GetDesktopWindow())`
  で top-level 化した後にのみ `WS_EX_LAYERED` が latch。失敗機構は
  cross-thread ではなく **`WS_CHILD` 状態の制約**として確定
  （band-escape の container-level silent drop と同機構を
  child-level で再現）。
- `ci_apply_reparent_exstyle` refused、`ci_restore`
  `ops_ok=false` — 必須 op 拒否 + restore 不完全は probe
  semantics どおりの honest 記録。
- `WS_CHILD | WS_EX_LAYERED` の新規作成は `0x80070006` で失敗
  （`docs/discovery/issue-839-gpui-dcomp.md`）。`WS_POPUP` +
  layered なら描画可能だが in-place hosting の要件を満たさない。

case の `fail` は verdict 契約（必須 op 拒否・restore 不完全 =
`Fail`）どおりであり、probe 実装の欠陥ではない。診断目的
（inventory・capture-plane 分離・layered 可否・env arm proxy
検証）は達成済みで、`PW_RENDERFULLCONTENT` 0.0 / `WM_PRINT` 系
1.0 の分離や `dcomp_disabled_by_env` proxy 検証は根治 fix の
根拠づけに既に使われている（`evidence/rf532/
child-inventory-results.md`、issue-839 doc chain）。

## 処遇決定

`docs/specs/issue-532-dcomp-residual.md` の決定に従い:

- **既知 Fail として継続**（#532 内で class C 確定）。verdict の
  昇格・`Unsupported` re-baseline は行わない — refused op の
  記録は verdict 契約どおり honest で、case 自体は unsupported
  ではない。
- **別 Issue 切り出し不要** — 診断目的達成済み。失敗は
  「棄却済み fix 経路の確定記録」として維持する。
- 将来の選択肢（記録のみ）: 「拒否を期待結果化する」re-scope
  または entry-gate `Unsupported` 化は verdict 意味論変更を要し、
  本 docs-only slice では実施しない。

## 証拠

- `evidence/rf532/plugin-dock-real-pinned.jsonl` —
  `real-dcomp-childinv` row（pinned-dep run の `fail`）。
- `evidence/rf532/child-inventory-results.md` — inventory・
  capture-plane・env arm・layered/reparent retry の全測定値。
- `evidence/rf532/childinv-dcomp-on.jsonl` /
  `childinv-dcomp-off.jsonl` / `childinv-pixels/` — raw 証拠。
- `evidence/rf532/residual-disposition.md` — AC 行との突合。
- `docs/discovery/issue-532-dcomp-residual.md` — class 確定の
  全根拠。
