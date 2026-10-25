---
id: "ADR-lg9vK3EAHePSiJf7YHThrm"
type: "adr"
status: "accepted"
title: "広範色管理の段階的範囲"
created: "2026-11-03"
updated: "2026-11-03"
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
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/second-target-design.md"
    - "docs/adr/video-proc-kernel-api-and-color-hdr-scope.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/second-target-design.md"
    - "docs/adr/video-proc-kernel-api-and-color-hdr-scope.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "docs/specs/color-conversion-kernels.md"
    - "docs/designs/color-conversion-kernels.md"
    - "docs/plans/implement-issue-99-color-conversion-kernels.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 広範色管理の段階的範囲

## Context and Problem Statement

ベースラインは「色管理の広範統合（`moxcms` / OpenColorIO 級 — 第 2 ターゲット設計時）」を保留とした。video-proc ADR D3 は初期範囲を「メタデータ搬送 + 非破壊」に確定済み。第 2 ターゲットとしての色管理方針を確定する。

## Considered Options

### D1. 内部 working space

- D1-a display-referred のまま（変換は入出力のみ）
- D1-b scene-referred 内部表現（f32 linear + 広色域 primaries）

### D2. 変換エンジン

- D2-a ICC プロファイル経路（Rust ネイティブ `moxcms` 級）
- D2-b OpenColorIO 設定消費（C++ 依存・ACES 標準）
- D2-c 両対応（ICC → 将来 OCIO）

### D3. ACES 完全管

- D3-a ACES IDT/ODT/RRT フルパイプラインを初期範囲へ
- D3-b ACES は後続（需要ベース）

## Decision Outcome

- **D1 — D1-b 採用。** 第 2 ターゲット（編集・コンポジット）では複数ソースの混合処理が前提であり、linear working space が正確性の要件。内部表現 = f32 linear + primaries は `ColorInfo` メタデータで明示（既存「メタデータ搬送」規約の上位拡張であり矛盾しない）。
- **D2 — D2-a 採用 + D2-b 保留。** 入出力変換は ICC 経路（`moxcms` 有力候補 — 採否は実装 Issue の依存追加規約に委任）。OCIO 設定消費は C++ 依存の重さに対し第 2 ターゲット初期要求が非比例 → 後続保留（構造上は変換ポートの追加で吸収可能）。
- **D3 — D3-b 採用。** ACES フル管は初期範囲外。linear working space 設計は ACES 導入を阻害しない。

### Consequences

- 責務: 色変換カーネル群（gamut/transfer 適用・トーンマップ）は video-proc カーネルへ段階追加（コンポジットと同層）。working space 変換は非 RT 準備 + RT カーネル実行の prepare/process 分離に従う。
- `ColorInfo` は working space の記述にも使用（新設の working-space 記述子との写像は design で定義）。
- HDR→SDR トーンマップ個別方式は実装 Issue で採否。

## Verification

- design が段階範囲（初期 ICC 経路 + linear working space・OCIO/ACES 後続）をクレート責務へ落とす。
