---
id: "SPEC-034KNvB6d6rMoH8b8xST22"
type: "spec"
status: "approved"
title: "ruviform-edit 新設（NLE 非破壊編集データモデル + 評価射影）"
created: "2026-10-23"
updated: "2026-10-23"
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
  implemented-by:
    - "docs/designs/edit-data-model.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/second-target-design.md"
    - "docs/designs/second-target-design.md"
    - "docs/adr/second-target-editing-model.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "docs/designs/edit-data-model.md"
  verifies: []
  verified-by:
    - "docs/test-specs/edit-data-model-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# ruviform-edit 新設（NLE 非破壊編集データモデル + 評価射影）

## What Should Be Built

`ruviform-edit` クレートの新設 — 第 2 ターゲット（NLE／映像制作級）の
非破壊編集データモデル + 評価射影クレート。
タイムライン（トラック × クリップ = ソース参照 + in/out + 属性）を保持し、
再生位置から audio グラフ / video pipeline への pull 射影に必要な
中立データ（アクティブクリップ列 + ソース時刻写像 + 属性）を生成する。
モデルの責務は**データ + 評価**のみ — engine 構造体の生成は行わない。

## Why Now

Issue #96。第 2 ターゲット設計（#32・approved）が後続実装 Issue 分解の
第 1 項目として「NLE 編集データモデル + 評価射影クレート」を定義済み。
ADR `second-target-editing-model`（accepted）で D1-a 非破壊編集モデル・
D3-a 既存 pull エンジンへの射影が確定し、設計入力が揃った。

## Users and Value

- NLE アプリケーション層 — タイムライン編集データの保持と再生位置評価を
  基盤に委譲し、undo/redo・UI・素材管理のみを自責務として実装できる
- 既存 engine（`ruviform-audio`/`ruviform-video`）の利用側 — 射影結果は
  中立データであり、engine 本体への変更なし（非侵襲）で pull 駆動へ接続できる

## Scope

### In Scope

- `Timeline`（順序付きトラック列）・`Track`（id・kind: Audio|Video・
  位置順クリップ列・enabled フラグ）・`Clip`（`ClipSource` 不透明ハンドル・
  タイムライン占有区間・ソース in/out 区間・種別属性）のデータモデル
- 評価 API: 再生位置 `MediaTime` から各トラックのアクティブクリップを解決し、
  タイムライン位置 → ソース時刻の写像とトラック種別・属性を返す
  （非 RT・確保許容）
- クリップ属性の最小セット: audio = `gain`（f32 linear）+ mute、
  video = `opacity`（f32）
- 編集操作 API（クリップ挿入/削除/移動/トリム等の最小 mutation）と
  クレート固有エラー型
- 同一トラック内オーバーラップの defined resolution・ギャップの契約

### Out of Scope

- undo/redo・編集 UI・素材管理/プロキシ運用（アプリ層責務 — 第 2 ターゲット
  設計の境界規則）
- engine 構造体（`Graph`/`FramePipeline`/`GraphPlan` 等）の生成 —
  射影は中立データのみであり、インスタンス化は consumer 責務
- トランジション・エフェクト・コンポジットカーネルとの配線
  （`ruviform-video-proc` カーネル追加の後続 Issue）
- `ruviform-audio`/`ruviform-video` engine 本体への変更（engine 非侵襲）・
  これらへの依存
- 再生速度変更（time-stretch）・逆再生・ネストシーケンス/複合クリップ
- タイムラインの直列化/復元フォーマット（serde 等）

## Requirements

- 許可依存は `ruviform-media-core`・`ruviform-audio-core`・
  `ruviform-video-core`（+ Primitives）に限定。engine 層
  （`ruviform-audio`/`ruviform-video`）への依存禁止 —
  射影が中立データであることを依存構造で保証する
- 全 API 非 RT — RT パスからの呼び出しは契約違反（`evaluate` の
  確保を含む）。RT 契約・`rt_` テストの適用対象外
- 時間は `MediaTime`/`MediaDuration`/`MediaRange`（有理数・半開区間）で
  表現し、ソース時刻写像は design で定義する規則に従う
- 公開 enum は `#[non_exhaustive]` 方針（ADR public-enum-non-exhaustive-policy）
- 評価結果は consumer が engine へ振分けるのに必要な情報（トラック種別を
  含む）を持ち、モデルの格納データは読み取り API 経由で参照できる
  （フィールド直接公開はしない）
- クリップ属性種別とトラック kind の整合は mutation 時に検証する

## Acceptance Criteria

- [ ] `Timeline`/`Track`/`Clip`・評価 API が test-spec の全ケースをパス
  （オーバーラップ解決・ギャップ・半開境界・ソース時刻写像の既知例を含む）
- [ ] engine への依存なし — 依存が `ruviform-media-core`・
  `ruviform-audio-core`・`ruviform-video-core`（+ Primitives）に限定される
  （他 foundations — `ruviform-dsp`・`ruviform-video-proc`・
  `ruviform-media-io`・`ruviform-audio-analysis` 等 — への依存なし）
- [ ] `mise run` 検証列全パス・doc 監査チェーン起因 findings 0

## Deferred Design Concerns

- トランジション/エフェクト属性 | コンポジットカーネル未実装 | video-proc カーネル追加 Issue 後 | 射影スキーマの拡張差分
- トラック単位属性（gain/pan 等 — `enabled` は範囲内） | ADR D1-a はトラック属性をモデル責務に含むが、`enabled` のみ採用する部分範囲縮小 | 音声ミキシング/コンソール要件の発生（video-proc カーネルとは別契機） | `Track` 属性・射影スキーマの拡張差分
- 再生速度・逆再生 | 1:1 写像で現要件を充足 | NLE 要件確定時 | 写像規則の再設計
- 区間評価（ブロック単位のセグメント列） | 点評価の連続呼出しで代替可能 | プリフェッチ/キャッシュ要求の発生 | API 追加差分
- 直列化/プロジェクト形式 | 保存フォーマット要件未確定 | アプリ層の保存要求発生時 | 依存追加・モデル拡張

## Sources and Relations

- docs/designs/second-target-design.md（責務分割・境界規則・後続 Issue 分解 #1）
- docs/adr/second-target-editing-model.md（D1-a 非破壊モデル・D3-a pull 射影・accepted）
- docs/specs/second-target-design.md（Issue #32 umbrella spec・approved）
- docs/designs/ruviform-architecture-design-baseline.md（層構造・依存方向ルール）
- Issue #96（本 spec の対象 Issue）・Issue #32（第 2 ターゲット設計）
