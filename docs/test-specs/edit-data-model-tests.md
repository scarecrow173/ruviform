---
id: "TSPEC-034KPgCJjA5nOIvEtscQj8"
type: "test-spec"
status: "approved"
title: "ruviform-edit 編集データモデル テスト仕様"
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
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/edit-data-model.md"
  derived-by: []
  verifies:
    - "docs/designs/edit-data-model.md"
    - "docs/specs/edit-data-model.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ruviform-edit 編集データモデル テスト仕様

## Purpose

Issue #96（`ruviform-edit` NLE 編集データモデル + 評価射影）の受入契約を
固定する。mutation の完全検証ルールセット（base 正規化・区間不変条件・
kind 整合・属性サニティ）と評価射影の決定的規則（オーバーラップ解決・
ギャップ・ソース時刻写像・到達可能残区間）が将来の変更で退行しない
ことを保証する。

## Feature

単一 `TimeBase` を所有する `Timeline`（順序付きトラック列・単調採番 ID）への
mutation（トラック追加/削除・クリップ挿入/削除/移動/トリム・enabled）と、
再生位置 `at` からの `evaluate` 射影（`Evaluation`/`ActiveClip` 中立データ）。

## Rules

- 格納 `Clip.range` は全て `timeline.base()` 揃え — mutation 境界で
  `convert(base, Rounding::NearestEven)` 正規化（base 不一致は丸め込みで
  受容し拒否しない。格納値は入力と半 tick 以下ずれ得る。`source_range` は
  ソース固有 base のまま正規化対象外）
- `insert_clip`/`move_clip`/`trim_clip` は完全検証ルールセットを毎回再実行:
  `range`/`source_range` とも正 duration かつ `checked_end(NearestEven)` が
  解決可能で `end > start`（base-collapse による `end == start`/`out == in`
  も `InvalidRange`、end 計算不能は `InvalidRange`/`TimeOverflow`）、
  `range.duration() <= source_range.duration()`（cross-base 正確比較 →
  違反は `InvalidRange`）、`ClipAttributes` variant = track `kind`
  （不一致は `KindMismatch`）、`gain` は `[0.0, ∞)` かつ `is_finite()`、
  `opacity` は `[0.0, 1.0]` かつ `is_finite()`（違反は `InvalidValue`）、
  未知 ID は `*NotFound`
- `TrackId`/`ClipId` は `Timeline` 採番の単調増加 `u64` — 削除後も番号を
  再利用しない。ステール ID の参照は `*NotFound`
- トラック内クリップ列は `(range.start(), ClipId)` 順を維持
  （`ClipId` は挿入順に単調増加するため、同開始位置は常に後挿入が
  後ろ＝評価の勝者 — insert/move/trim のいずれの再ソートでも同一規則）。
  `move_clip`/start 変更を伴う `trim_clip` は再ソートする
- 評価: `at` を timeline base へ `convert(NearestEven)` で 1 回だけ変換
  （失敗は `TimeOverflow`）。各 enabled トラックについて半開区間
  `range.start() <= at < range.end()` を含むクリップを走査
- オーバーラップ defined resolution: 開始位置が最大のクリップが勝者。
  同開始位置はトラック内順で最後（後挿入）が優先
- 写像: `offset = at - range.start()`（同一 base の i64 差分）、
  `source_time = source_range.start().checked_add(offset)`（source base へ
  `NearestEven` 変換）。`ActiveClip.source_range` は到達可能残区間
  `[source_time, min(out, in + range.duration()))` — `source_range` が
  range より長い場合の末尾側は到達不能として報告しない。残スパンが
  ソース base の 1 tick 未満へ丸められる場合、独立した `NearestEven`
  変換同士が衝突し残区間は空（`end == start`）となり得る —
  consumer は zero duration を許容する
- ギャップ・無効トラックは `active` に要素を出さない（無音/黒は consumer
  責務）。`Evaluation`/`ActiveClip`/`ClipSpec` は pub フィールドの
  plain data、格納モデル（`Timeline`/`Track`/`Clip`）は全フィールド
  private で accessor 経由
- 全 API 非 RT — `evaluate` は `Vec` 確保を含み、RT パスからの呼出しは
  契約違反（`rt_` テスト対象外）

## Examples

- `Timeline::new(base)` → `base()` が入力を返し `tracks()` 空・`end() = None`。
  `add_track(TrackKind::Audio)` → `TrackId` 発番・トラックは末尾追加
- ID 単調・再利用なし: トラック削除後の `add_track` は以前より大きい
  `TrackId` を返す。削除済み ID の `track()`/`clip()`/`remove_*` 参照は
  `None`/`TrackNotFound`/`ClipNotFound`
- base 正規化: `spec.range` が timeline base と異なる base を持つ
  `insert_clip` → 拒否せず `NearestEven` 変換して格納（格納 `clip.range()` は
  timeline base 揃え）
- 拒否: duration 非正 → `InvalidRange`。`end == start` へ潰れる `range` と
  `out == in` の `source_range` の両方 → `InvalidRange`。
  `range.duration() > source_range.duration()` → `InvalidRange`。
  Audio トラックへ `ClipAttributes::Video` → `KindMismatch`。
  `gain = -1.0`/`NaN` → `InvalidValue("gain")`、`opacity = 1.5`/`NaN` →
  `InvalidValue("opacity")`
- `move_clip` は duration 不変で再ソート・`new_start` を base 変換して適用
  （overflow は `TimeOverflow`/`InvalidRange` — 飽和しない）。`trim_clip` は
  `new_range` 正規化後に区間を原子的に再指定し不変条件を再検証、start 変化時は
  再ソート
- 評価: ギャップ位置 → そのトラックは `active` に要素なし。重複する
  2 クリップ → 開始位置が後のクリップが勝者。同開始 2 クリップ → 後挿入が
  勝者。`enabled = false` トラック → 全クリップ非アクティブ
- 写像: `range = [10, 20)`・`source_range = [100, 200)`・`at = 15` →
  `source_time = 105`、`ActiveClip.source_range = [105, min(200, 100+10))
  = [105, 110)`。`source_range` が range と同長なら残区間は
  `[source_time, out)` まで到達可能
- cross-base `at`: 別 base の `MediaTime` を指定 → timeline base へ 1 回変換
  して評価。`at` 変換・差分・`checked_add`/`checked_end` の i64 超過 →
  `TimeOverflow`
- accessor: `track(id)`/`clip(track, clip)`/`tracks()`/`end()`（全トラック
  横断の最大解決済みクリップ end）・`Track::{id,kind,clips,enabled}`・
  `Clip::{id,source,range,source_range,attributes}`。`Evaluation`/
  `ActiveClip` の pub フィールドは読み取り専用スナップショット形状

## Guarantees

- 評価は決定的 — 同一 `Timeline` と `at` に対し常に同一の `Evaluation`
- 検証失敗の mutation は状態を変更しない（格納前検証・原子適用）
- `Evaluation`/`ActiveClip`/`ClipSpec` は engine 型を含まない中立データ —
  依存が `ruviform-media-core`（+ 必要時 foundations）に限定され、engine
  層への依存を持たない
- `ActiveClip` は `kind: TrackKind` を持ち、consumer が audio/video engine
  へ振分けられる

## Non-goals

- RT 契約・`rt_` no-alloc テスト — 全 API が非 RT で `evaluate` の確保は
  設計上許容されるため、no-alloc 規律テストは設けない
- engine 構造体生成・undo/redo・素材管理・直列化・トランジション/
  エフェクト・再生速度変更・区間評価
- オーバーラップの禁止 — モデルは許容が契約。禁制を求めるアプリは
  `Track::clips()` で挿入前に自前検査する

## Risk

- base-collapse 検査の片側見落とし（`source_range` の `out == in`）→ 評価が
  反転した `ActiveClip.source_range`（`[source_time, in)`）を出力する
- `range.duration() <= source_range.duration()` の欠落・cross-base 近似比較
  → 1:1 写像を逸脱し残区間端 `in + range.duration()` が `out` を越える
- オーバーラップ解決の非決定性 → 同一タイムラインで再生結果が経路依存になる
- 空の残区間の見落とし → `end == start` の `ActiveClip.source_range` を
  consumer が不正値として拒否する（zero duration は契約上の正常値）
- 丸め箇所の増殖 → cross-base 誤差を暗黙に吸収（設計は変換箇所を `at` 1 回 +
  `checked_add` 2 呼出しに固定）
