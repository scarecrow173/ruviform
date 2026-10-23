---
id: "DESIGN-Tl9CIdfNfmIADLgFIeaTOq"
type: "design"
status: "approved"
title: "第 2 ターゲット設計（NLE 編集モデル・OFX 範囲・色管理）"
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
  implements: ["docs/specs/second-target-design.md"]
  implemented-by: []
  depends-on:
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/adr/second-target-editing-model.md"
    - "docs/adr/ofx-video-plugin-scope.md"
    - "docs/adr/broad-color-management.md"
    - "docs/designs/plugin-isolation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/second-target-design.md"
    - "docs/adr/second-target-editing-model.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/plugin-isolation.md"
  derived-by:
    - "docs/plans/implement-issue-32-second-target-design.md"
    - "docs/specs/edit-data-model.md"
    - "docs/designs/edit-data-model.md"
    - "docs/specs/composite-transition-kernels.md"
    - "docs/designs/composite-transition-kernels.md"
  verifies: []
  verified-by: ["docs/tasks/second-target-design-verify.md"]
  references: []
  defers: []
  deferred-by: []
---

# 第 2 ターゲット設計

Issue #32 — 設計ベースライン保留項（NLE 範囲・OFX 等映像プラグイン規格・広範色管理）の確定。3 ADR（accepted）の決定を責務分割・写像規則・後続 Issue 分解へ具体化する。**本 Issue は設計のみ — コード実装は後続 Issue。**

## 決定サマリ

| 論点 | 決定 | ADR |
|---|---|---|
| NLE 編集モデル | 非破壊編集データモデル + 既存 engine への評価射影・コンポジットは video-proc カーネル | `second-target-editing-model.md` |
| 映像プラグイン規格 | OFX 採用 — Image Effect API ホスト側・plugin-core アダプタ構造 | `ofx-video-plugin-scope.md` |
| 色管理 | f32 linear working space + ICC 経路（moxcms 候補）・OCIO/ACES 後続 | `broad-color-management.md` |

## 責務分割

- **編集データモデル + 評価射影**: 後続クレート（`ruviform-edit` 相当・命名は実装 Issue 確定）。非 RT・media-core/audio-core/video-core 依存。タイムライン（トラック×クリップ=ソース参照+in/out+属性）を保持し、再生位置から audio グラフ/video pipeline への pull 射影を生成する。engine 自体は非侵襲。
- **コンポジット/遷移**: `ruviform-video-proc` へ RT カーネルとして段階追加（blend/affine/transition）。既存色変換カーネルと同一の prepare/process・RT 契約。
- **OFX**: `ruviform-plugin-ofx`（仮）— plugin-core `PluginProcessor`/ライフサイクル/L0 インプロセス既定（plugin-isolation 設計）に整合する薄アダプタ。ピクセル平面は video-core `VideoFrame`、色は `ColorInfo`、時間は `MediaTime`/`FrameRate` へ写像。GUI は汎用パラメータ UI 生成で代替。
- **色管理**: working space = f32 linear + `ColorInfo` primaries 明示。変換カーネル（gamut/transfer 適用・トーンマップ）は video-proc へ段階追加（非 RT 準備 + RT 実行の分離）。入出力 ICC 変換は別責務（実装 Issue で `moxcms` 級を採否）。

## 境界規則

- 編集モデルは**データ + 評価**のみ — undo/redo・UI・素材管理はアプリ層。
- OFX の GPU suite・authoring・Generator/Transition suite は初期範囲外（ADR D2）。
- OCIO 設定消費・ACES IDT/ODT/RRT フル管は後続保留（変換ポート追加で吸収可能な構造）。
- 依存方向は既存ルール不変: edit/adapter は foundations・Primitives のみ下向き利用。

## 後続 Issue 分解（実装候補・順序案）

1. NLE 編集データモデル + 評価射影クレート（`ruviform-edit` 相当）
2. video-proc コンポジット/遷移カーネル追加（blend/affine 初期セット）
3. `ruviform-plugin-ofx` — OFX Image Effect ホスト側アダプタ
4. 色変換カーネル（linear working space 変換 + ICC 入出力経路・`moxcms` 採否含む）

## Deferred（本設計でも据え置く項目）

| 項目 | 再着手トリガ |
|---|---|
| OCIO 設定消費・ACES フル管 | NLE 級の OCIO 要求・ACES ワークフロー要求の発生 |
| OFX authoring・追加/GPU suite | プラグイン制作側需要・GPU 経路要求 |
| HDR→SDR トーンマップ個別方式 | 色変換カーネル実装 Issue |
| 映像追加規格（Fuse 等） | 規格追加要求 |
