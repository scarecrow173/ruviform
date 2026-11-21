---
id: "TSPEC-034ZccDcompBandEsc01"
type: "test-spec"
status: "approved"
title: "Issue #532 dcomp-band-escape — residual class 確定記録（WS_EX_LAYERED 適用不能の確定診断）"
created: "2026-11-21"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-532-dcomp-band-escape.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/test-specs/issue-532-band-escape.md"
    - "docs/specs/issue-532-dcomp-residual.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-532-band-escape.md"
    - "docs/specs/issue-532-band-escape.md"
    - "docs/specs/issue-532-dcomp-residual.md"
    - "docs/discovery/issue-532-dcomp-residual.md"
    - "docs/test-specs/issue-532-dcomp-residual.md"
    - "evidence/rf532/band-escape-results.md"
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

# Issue #532 `real-dcomp-bandescape` — residual class 確定記録

`real-dcomp-bandescape` case（band-escape probe — container-level
`osmut` mutation family）について、根治 fix（Issue #839）統合後の
pinned-dep run で記録された `fail` の class 確定と処遇を記録する。
case 自体の検証契約は姉妹文書
`docs/test-specs/issue-532-band-escape.md` が正本であり、本文書は
その verdict の **residual class** を対象とする。

## 確定した class: C — `WS_EX_LAYERED` 適用不能の確定診断

本 case が問うた「container を `WS_EX_LAYERED` 化して band 脱出
（presentation 経路切替）できるか」は、実走により **適用不能**
と確定した。

- `be_apply_layered_exstyle`:
  `SetWindowLongPtrW(GWL_EXSTYLE, ... | WS_EX_LAYERED)` は
  last-error 上成功だが `GetWindowLongPtrW` read-back で bit
  absent — **silent no-op**（`applied: false`、`win32_error: null`）。
- `be_apply_layered_lwa`: `SetLayeredWindowAttributes(LWA_ALPHA)` は
  error 87（`ERROR_INVALID_PARAMETER`）— `WS_EX_LAYERED` 非保持
  window への LWA 呼出拒否と一致。
- reparent 経路（`SetParent(GetDesktopWindow())` + `HWND_TOP` +
  同一 rect）は実行され、container は GPUI root 直上の top-level
  に到達したが、screen capture は baseline と byte 同一 — pixel
  欠損は不変（`evidence/rf532/band-escape-results.md`）。
- `be_restore` = pass（`parent_restored`/`exstyle_restored`/
  `rect_restored` 全 true）— probe は設計どおり完走・復元済み。

`fail` の根拠は case の verdict 契約（F4:「必須 mutation の拒否
（applied=false）は `fail`」）どおりであり、probe 実装の欠陥や
未確定の失敗ではない。診断目的（layered 経路の可否）は達成済み
で、判定結果は根治 fix slice の経路棄却
（`docs/discovery/issue-839-gpui-dcomp.md`「棄却した経路」:
`SetWindowLongPtrW` silent no-op、`WS_CHILD|WS_EX_LAYERED` 作成
`0x80070006`）に反映されている。

## 処遇決定

`docs/specs/issue-532-dcomp-residual.md` の決定に従い:

- **既知 Fail として継続**（#532 内で class C 確定）。verdict の
  昇格・`Unsupported` re-baseline は行わない — case は正常完走
  しており refused op の記録は verdict 契約どおり honest。
- **別 Issue 切り出し不要** — 診断目的達成済みで、追加調査や
  製品修正の対象が残っていない。失敗は「棄却済み fix 経路の
  確定記録」として価値を持つ。
- 将来の選択肢（記録のみ）: lane の恒久的 `fail` を避けたい場合、
  「拒否を期待結果として検証する」re-scope（refused を Pass 期待
  とする断言型）または entry-gate `Unsupported` 化が考えられるが、
  verdict 意味論変更を要し本 docs-only slice では実施しない。

## 証拠

- `evidence/rf532/plugin-dock-real-pinned.jsonl` —
  `real-dcomp-bandescape` row（pinned-dep run の `fail`）。
- `evidence/rf532/band-escape-results.md` — staged mutation の
  全 op 記録・byte 同一 capture・fix-decision table。
- `evidence/rf532/residual-disposition.md` — AC 行との突合。
- `docs/discovery/issue-532-dcomp-residual.md` — class 確定の
  全根拠。
