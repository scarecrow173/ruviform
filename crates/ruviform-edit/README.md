# ruviform-edit

NLE 非破壊編集データモデル + 評価射影（Issue #96）。
単一 `TimeBase` を所有する `Timeline`（順序付きトラック列・単調採番 ID）への
mutation と、再生位置 `at` からの `evaluate` 射影（engine 中立データ）を
提供する。ADR `second-target-editing-model` の D1-a（タイムライン/クリップ/
トラック + ソース参照 + in/out + 属性）・D3-a（既存 pull engine への射影）を
実体化したクレート。

## 責務

- 格納モデル: `Timeline` / `Track` / `Clip`（全フィールド private・accessor 公開）
- 入力 spec: `ClipSpec`（pub フィールド plain data — `insert_clip` が完全検証）
- mutation: `add_track` / `remove_track` / `insert_clip` / `remove_clip` /
  `move_clip` / `trim_clip` / `set_track_enabled`
- 評価射影: `Timeline::evaluate(at) -> Result<Evaluation>` —
  `Evaluation`/`ActiveClip`（pub フィールド plain data・engine 型を含まない）

undo/redo・UI・素材管理・直列化は本クレートの責務外（アプリ層）。
engine 構造体（`Graph`/`FramePipeline` 等）の生成は consumer 責務 —
射影は中立データのみを返す（第 2 ターゲット設計の境界規則）。

## 層・許可依存

- 層: 利用側層（engine と同順位 — engine ではない。RT パス・デバイス・
  セッションを持たない純データモデル）
- 許可依存: `ruviform-media-core` のみ（`MediaTime`/`MediaDuration`/
  `MediaRange`/`TimeBase`/`Rounding` — `convert`/`checked_end`/`checked_add`・
  cross-base 正確比較）。spec で `ruviform-audio-core`/`ruviform-video-core`
  も許可されているが、最小属性セットでは media-core 時間型が主依存 —
  実需が出た時点で追加する（YAGNI）。engine クレート・他 foundations
  への依存は禁止（射影の中立性を依存構造で保証）。

## RT 契約

全 API 非 RT — `evaluate` を含めアロケーションを行うため、RT パスからの
呼び出しは契約違反（`rt_` テスト対象外）。`Timeline` は `Send` 不要求 —
並行アクセスは呼出し側責務（`InProcessHost` と同規約）。

## 関連

- 設計ベースライン: docs/designs/ruviform-architecture-design-baseline.md（層構造）
- spec: docs/specs/edit-data-model.md
- 設計: docs/designs/edit-data-model.md
- ADR: docs/adr/second-target-editing-model.md（D1-a / D3-a）
- テスト仕様: docs/test-specs/edit-data-model-tests.md
- Issue #96（クレート新設）
