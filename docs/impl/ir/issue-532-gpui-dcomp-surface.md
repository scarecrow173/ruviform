---
id: "IMPL-034XB4yHhkxYZTBzEB6MRO"
type: "impl"
status: "completed"
title: "Issue #532 GPUI DComp public-surface 監査 実装記録"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/discovery/issue-532-gpui-dcomp-surface.md"
      - type: "file"
        path: "docs/designs/issue-532-gpui-dcomp-surface.md"
      - type: "file"
        path: "docs/specs/issue-532-gpui-dcomp-surface.md"
      - type: "file"
        path: "docs/plans/issue-532-gpui-dcomp-surface.md"
      - type: "file"
        path: "docs/tasks/issue-532-gpui-dcomp-surface.md"
      - type: "file"
        path: "docs/test-specs/issue-532-gpui-dcomp-surface.md"
      - type: "file"
        path: "docs/impl/ir/issue-532-gpui-dcomp-surface.md"
      - type: "file"
        path: "docs/impl/exp/issue-532-gpui-dcomp-surface.md"
      - type: "file"
        path: "evidence/rf532/gpui-public-surface-audit.md"
    modified:
      - type: "file"
        path: "docs/discovery/issue-532-dcomp-native-dock.md"
      - type: "file"
        path: "docs/designs/issue-532-dcomp-native-dock.md"
      - type: "file"
        path: "docs/discovery/README.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/discovery/README.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
  implements:
    - "docs/tasks/issue-532-gpui-dcomp-surface.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-532-gpui-dcomp-surface.md"
    - "docs/plans/issue-532-gpui-dcomp-surface.md"
    - "docs/specs/issue-532-gpui-dcomp-surface.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-532-gpui-dcomp-surface.md"
    - "docs/specs/issue-532-gpui-dcomp-surface.md"
    - "docs/designs/issue-532-gpui-dcomp-surface.md"
    - "docs/plans/issue-532-gpui-dcomp-surface.md"
    - "docs/tasks/issue-532-gpui-dcomp-surface.md"
    - "docs/test-specs/issue-532-gpui-dcomp-surface.md"
    - "docs/impl/exp/issue-532-gpui-dcomp-surface.md"
    - "evidence/rf532/gpui-public-surface-audit.md"
    - "docs/discovery/issue-532-dcomp-native-dock.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/issue-532-gpui-dcomp-surface.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-532-gpui-dcomp-surface.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/262"
    - "https://github.com/scarecrow173/synvelis/issues/176"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-532-gpui-dcomp-surface.md"
    rejected: []
---

# Issue #532 GPUI DComp public-surface 監査 実装記録

## Summary

vendored `gpui-0.2.2`（cargo registry、読み取り専用）の Windows
platform 層を 7 項目監査し、方式 A/B/C の技術入力を正本 discovery
・証拠ノート・設計文書として固定した docs-only slice。コード変更・
patch/vendor 適用・実機検証は含まない。

## Context

`docs/tasks/issue-532-gpui-dcomp-surface.md`・
`docs/plans/issue-532-gpui-dcomp-surface.md`・
`docs/specs/issue-532-gpui-dcomp-surface.md`・
`docs/designs/issue-532-gpui-dcomp-surface.md`・
`docs/test-specs/issue-532-gpui-dcomp-surface.md` に従う。先行の
headless 監査 slice（`docs/discovery/issue-532-dcomp-native-dock.md`
status: resolved、merge 済み）が残した Open Questions の upstream
public-surface 調査分を処理する。

## Implementation

### Changes

- `evidence/rf532/gpui-public-surface-audit.md` — 監査証拠ノート
  （コードパス・行番号・判定根拠の全文）。
- `docs/discovery/issue-532-gpui-dcomp-surface.md` — 正本監査記録
  （F1-F7・方式 A/B/C への技術入力・Open Questions 更新）。
- `docs/specs|designs|plans|tasks|test-specs|impl/ir|impl/exp/
  issue-532-gpui-dcomp-surface.md` — 文書チェーン。
- `docs/discovery/issue-532-dcomp-native-dock.md` — Open Questions
  を更新（回答可能分を解消・実 OS 依存分を open 維持・status は
  resolved のまま）。
- `docs/designs/issue-532-dcomp-native-dock.md` — upstream 調査
  リスク記述を更新（監査結果への参照を追加）。
- 各 `docs/<type>/README.md` index — `doc-maintenance` で再生成。

### Decisions During Implementation

- **A'（zero-code 迂回）の比較表への追加**: `GPUI_DISABLE_DIRECT_COMPOSITION`
  が patch 不要の代替経路であることを監査で確定したため、方式 A/B/C
  の比較枠組みに「A'（env var 迂回）」を追加した。実機効果は未検証。
- **exit-127 は仮説のみで close しない**: コードパスから原因を一意に
  特定できないため、H1-H5 の仮説を記録し PENDING として残した。
  実 OS での stderr/exit code 捕捉再現が必要。
- **`topmost=false` の効果は断定しない**: MSDN 上の意味論と実機
  挙動が異なりうるため、fixture 実測待ちとして open を維持。
- **Option B を OS interposition として再定義**: gpui の DComp object
  は全て `pub(crate)` で再利用不能のため、B の実質的設計空間は
  env var/`SetWindowRgn`/第二 DComp target の OS 公開 API
  interposition に限定されることを確認。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-532-gpui-dcomp-surface.md` — 監査手法の
  試行記録（行番号再検証・crate 全体 grep・version 情勢確認の
  方法と結果）。

### Rejected

- なし。

## Validation

- [x] 監査 7 項目が `docs/discovery/issue-532-gpui-dcomp-surface.md`
  と `evidence/rf532/gpui-public-surface-audit.md` に記録された。
- [x] 全ての行番号・コードパスが vendored source で再検証された
  （`directx_renderer.rs:793`・`platform.rs:138-139`・`window.rs:
  409-411` 等）。
- [x] 実機依存事項（exit-127・`topmost=false` 効果・実 plugin
  画素）が PENDING/未検証として明記された。
- [ ] `doc-maintenance` `index apply` + `doc-status` で relations
  解決を確認。
- [ ] `mise exec -- prek run --files <変更ファイル>` が exit 0。

## Risks

- exit-127 の原因が未解明のまま残る（実 OS 再現が必要）。
- `topmost=false`/env var/`SetWindowRgn` の実画素効果が未検証。
- 方式 A/B/C の採否は本 slice では決定しない。

## Follow-ups

- Windows L2/L3 利用要件発生時に実機調査を再開（`docs/designs/
  issue-532-dcomp-native-dock.md` の手順 + 本監査の技術入力）。
- exit-127 の実 OS 再現と原因確定。
- 方式 A/B/C（A' 含む）の採否判定と #262 ADR 入力。
