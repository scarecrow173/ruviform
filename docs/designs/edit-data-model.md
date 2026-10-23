---
id: "DESIGN-034KNBPbww1kvzVUF4Ab1H"
type: "design"
status: "approved"
title: "ruviform-edit 設計（NLE 編集データモデル + 評価射影）"
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
  implements:
    - "docs/specs/edit-data-model.md"
  implemented-by: []
  depends-on:
    - "docs/specs/edit-data-model.md"
    - "docs/designs/second-target-design.md"
    - "docs/adr/second-target-editing-model.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/edit-data-model.md"
    - "docs/designs/second-target-design.md"
    - "docs/adr/second-target-editing-model.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "docs/plans/implement-issue-96-edit-model.md"
    - "docs/test-specs/edit-data-model-tests.md"
  verifies: []
  verified-by:
    - "docs/test-specs/edit-data-model-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# ruviform-edit 設計（NLE 編集データモデル + 評価射影）

## Context

Issue #96 — 第 2 ターゲット設計の後続 Issue 分解・第 1 項目。
ADR `second-target-editing-model` の D1-a（非破壊編集モデル:
タイムライン/クリップ/トラック + ソース参照 + in/out + 属性）と
D3-a（既存 pull engine への射影）を、非 RT クレート `ruviform-edit` の
データモデル + 評価 API へ具体化する。
編集モデルは**データ + 評価**のみを責務とし、undo/redo・UI・素材管理は
アプリ層、engine 構造体（`Graph`/`FramePipeline` 等）の生成は consumer 責務
（第 2 ターゲット設計の境界規則）。

## 決定サマリ

| 論点 | 決定 |
| --- | --- |
| クレート名・層 | `ruviform-edit` — engine と同順位の利用側層（engine ではない。RT パス・デバイス・セッションを持たない純データモデル）。全 API 非 RT |
| 依存 | `ruviform-media-core`・`ruviform-audio-core`・`ruviform-video-core`（+ Primitives）のみ。engine 非依存 — 射影は中立データ |
| ID | `TrackId`/`ClipId` = `Timeline` が採番する単調増加 `u64` newtype。`ClipSource` = アプリ定義の `u64` 不透明ハンドル |
| オーバーラップ | 同一トラック内の重複はモデルで許容（編集中の一時状態を拒否しない）。評価時の defined resolution = 開始位置が最も後のクリップが優先・同開始は後挿入が優先 |
| ギャップ | 射影はアクティブクリップのみ列挙 — ギャップ部の無音/黒は consumer が生成 |
| タイムベース | `Timeline` が単一 `TimeBase` を所有（`Timeline::new(base)`）。格納 `Clip.range` は全て `timeline.base()` 揃え — mutation 境界で `convert(Rounding::NearestEven)` して正規化する（`source_range` はソース固有 base のまま）。評価での丸めは `at` 変換の 1 回 + `checked_add` 呼出しごとに最大 1 回（本経路では 2 呼出し → 最大 3 回） |
| 読み取り | 不変条件を持つ格納モデル（`Timeline`/`Track`/`Clip`）は全フィールド private（media-core 規約）で accessor 公開。`Evaluation`/`ActiveClip`/`ClipSpec` は不変条件を持たない pub フィールドの plain data（読み取り専用スナップショット/入力 spec）。`ActiveClip` は `kind: TrackKind` を持ち consumer の engine 振分けを担う |
| 編集操作 | 可変 `Timeline` + 明示 mutation API（不変モデルの再構築は採らない） |
| エラー | クレート独自 `#[non_exhaustive]` `Copy` enum `Error`（`HostError` 規約）。media-core 時間エラーは `InvalidValue`/`TimeOverflow` へ正規化 |

## Scope

### In Scope

- データモデル型: `Timeline`・`Track`・`Clip`・`TrackKind`・
  `ClipAttributes`・`TrackId`/`ClipId`/`ClipSource` newtype
- 評価: `Timeline::evaluate(at: MediaTime) -> Result<Evaluation>` と
  `Evaluation`/`ActiveClip` 値型
- mutation: `add_track`/`remove_track`/`insert_clip`/`remove_clip`/
  `move_clip`/`trim_clip` + トラック `enabled` フラグ
- 読み取り API: 不変条件を持つ格納モデル（`Timeline`/`Track`/`Clip`）の
  フィールドは private で accessor 経由（media-core 規約）—
  `Timeline::tracks()`/`track()`/`clip()`/`end()`/`base()`、
  `Track::id()`/`kind()`/`clips()`/`enabled()`、
  `Clip::id()`/`source()`/`range()`/`source_range()`/`attributes()`。
  `Evaluation`/`ActiveClip`/`ClipSpec` は不変条件を持たない
  pub フィールドの plain data（accessor なし）
- `Error`/`Result` エラー型

### Out of Scope

- engine 構造体の生成・undo/redo・UI・素材管理・直列化（spec Out of Scope と同一）
- トランジション/エフェクト・コンポジットカーネル・再生速度変更

## Components and Boundaries

| 型 | 形状 | 責務 |
| --- | --- | --- |
| `TrackId`/`ClipId` | `u64` newtype・`Copy` | `Timeline` 採番の単調 ID（採番番号の再利用なし）。generational 化はしない — `PluginHandle` のみの generational 規約に倣い過剰設計しない。ステール ID は検索失敗として `*NotFound` で検出 |
| `ClipSource` | `u64` newtype・`Copy` | ソース実体への不透明参照 — アプリが media-io/素材レジストリへ写像する外部キー。モデルは実体を解決・検証しない |
| `TrackKind` | `#[non_exhaustive]` enum（`Audio`/`Video`） | トラック種別 — クリップ属性との整合検証と consumer の engine 振分け |
| `Clip` | `{id, source, range: MediaRange, source_range: MediaRange, attributes}`（全フィールド private — `id()`/`source()`/`range()`/`source_range()`/`attributes()` で読み取り） | タイムライン占有区間 `[start, start+duration)`・ソース選択 `[in, out)`・属性。`range` は `timeline.base()` 揃えで格納し、`source_range` はソース固有 base を保持する。不変条件: 両区間とも正 duration かつ `range.duration() <= source_range.duration()`（1:1 写像で `out` を越えないこと） |
| `ClipSpec` | `{source, range, source_range, attributes}`（pub フィールドの plain data — 不変条件は持たず `insert_clip` で検証） | `insert_clip` の引数束（id 採番前のクリップ定義）。`range` の base は任意 — 格納時に timeline base へ正規化される |
| `ClipAttributes` | `#[non_exhaustive]` enum `Audio{gain: f32, mute: bool}`/`Video{opacity: f32}` | 種別属性の最小セット — 解釈済み値であり未定義 payload ではない。`gain` は f32 リニア倍率（`[0.0, ∞)` かつ `is_finite()`）、`opacity` は不透明度（`[0.0, 1.0]` かつ `is_finite()`）で、値域を mutation 時に検証する。トラック `kind` と variant の一致も mutation 時に強制 |
| `Track` | `{id, kind, clips: Vec<Clip>, enabled: bool}`（全フィールド private — `id()`/`kind()`/`clips() -> &[Clip]`/`enabled()` で読み取り） | 開始位置順に維持されるクリップ列（同位置は挿入順・stable）。`enabled = false` は評価で全クリップを非アクティブにする。`clips()` はアプリ側の挿入前オーバーラップ検査にも使う |
| `Timeline` | `{base: TimeBase, tracks: Vec<Track>}` + 採番カウンタ（全フィールド private） | `Timeline::new(base)` で構築する全 mutation の authoritative 窓口 — クリップ列のソート不変条件・kind 整合・区間検証・`range` の base 正規化を集約。読み取り: `base() -> TimeBase`・`tracks() -> &[Track]`・`track(TrackId) -> Option<&Track>`・`clip(TrackId, ClipId) -> Option<&Clip>`・`end() -> Option<MediaTime>`（全トラック横断の最大解決済みクリップ end。クリップなしでは `None`） |
| `Evaluation` | `{at: MediaTime, active: Vec<ActiveClip>}`（pub フィールドの plain data — 読み取り専用スナップショット） | 位置 `at` の射影結果。トラック挿入順の per-track アクティブ列（ギャップ・無効トラックは要素を出さない） |
| `ActiveClip` | `{track: TrackId, clip: ClipId, kind: TrackKind, source: ClipSource, source_range: MediaRange, attributes: ClipAttributes}`（pub フィールドの plain data — 読み取り専用スナップショット） | 1 トラックの勝者クリップ + トラック種別（consumer の engine 振分け用）+ 到達可能な残ソース区間 `[source_time, min(out, in + range.duration()))` + 属性スナップショット |
| `Error` | `#[non_exhaustive]` `Copy` enum | `TrackNotFound`/`ClipNotFound`/`InvalidRange(&'static str)`/`KindMismatch`/`InvalidValue(&'static str)`/`TimeOverflow` |

## Data and Control Flow

### mutation

- `Timeline::new(base: TimeBase)` で空タイムライン — タイムラインは単一の
  `TimeBase` を所有し、格納される全 `Clip.range` はこの base 揃えの不変条件を
  持つ。`add_track(kind) -> TrackId` は末尾へ追加
  （トラック順 = 評価列挙順であり、video では consumer が積層順として解釈する中立データ）
- `insert_clip(track, ClipSpec) -> Result<ClipId>`: `spec.range` の
  start/duration を `convert(timeline.base(), Rounding::NearestEven)` で
  正規化してから検証し、開始位置順を保つ位置へ挿入する。base 不一致は
  丸め込みで受容し拒否しない（変換は `NearestEven` で丸め得る — 格納値は
  入力と半 tick 以下ずれ得ることを契約として明記する）。変換の overflow は
  `TimeOverflow` として伝播する。`source_range` はソース固有 base のまま
  保持する（正規化対象外）
- `remove_clip(track, clip) -> Result<Clip>`・`remove_track(track) -> Result<Track>`
- `move_clip(track, clip, new_start) -> Result<()>`: duration 不変で開始位置のみ
  更新し再ソート。`new_start` は timeline base へ `convert` してから適用し、
  変換・位置演算の overflow は `TimeOverflow`/`InvalidRange` を返す —
  暗黙の飽和は行わない。トラック間移動は `remove_clip` + `insert_clip` で表現
  （kind 変更を伴うため専用 op は持たない）
- `trim_clip(track, clip, new_range, new_source_range) -> Result<()>`:
  `new_range` を timeline base へ正規化し、区間の再指定を原子的に行い
  不変条件を再検証する。`range.start()` が変化した場合は開始位置順を
  再確立する（start 変更を伴う trim = 再ソート）
- 検証規則: クリップを格納・変更する全 mutation（`insert_clip`/`move_clip`/
  `trim_clip`）は以下の完全ルールセットを毎回再実行する
  - `range`/`source_range` は正 duration — 違反は `InvalidRange`
  - `range.checked_end(Rounding::NearestEven)` で解決済み end が計算可能
    かつ `end > start` — 正 duration が base 変換で半 tick 以下に潰れ
    `end == start` となる base-collapse も `InvalidRange`
  - この end 解決・厳密性規則は `range`（正規化後の timeline base）と
    `source_range`（ソース固有 base のまま — start/duration の内部 base が
    混在し得る）の両方へ適用する — 潰れた `source_range`（`out == in`）は
    `InvalidRange`、end 計算不能は `InvalidRange`/`TimeOverflow`。
    `source_time > in` の反転した `ActiveClip.source_range`
    （`[source_time, in)`）を評価が出さないための事前条件
  - `range.duration() <= source_range.duration()`（media-core の正確な
    cross-base 比較）→ 違反は `InvalidRange`
  - `ClipAttributes` variant = track `kind` → 不一致は `KindMismatch`
  - 属性値サニティ: `gain`/`opacity` は `is_finite()`、`opacity` は
    `[0.0, 1.0]`、`gain` は `[0.0, ∞)` — 違反は
    `InvalidValue("gain")`/`InvalidValue("opacity")`
  - 未知 ID → `*NotFound`

### 評価（`evaluate`）

1. `at` を `convert(timeline.base(), Rounding::NearestEven)` で timeline
   base へ 1 回だけ変換する（変換失敗は `TimeOverflow`）。以降の比較・
   演算は全て同一 base 上の i64 で行い、各 enabled トラックについて
   `at` を含むクリップを走査（半開区間
   `range.start() <= at < range.end()` で判定 — 格納 `range` は
   start/duration とも timeline base 揃えのため `end()` は
   `checked_end(Rounding::NearestEven)` の内部変換が恒等となり、
   `range.start().value() + range.duration().value()` の値演算に一致する）
2. 複数クリップが `at` を含む場合の defined resolution:
   **開始位置が最大のクリップ**を勝者とする。同開始位置はトラック内で後ろ
   （後挿入）が優先 — トラック内順の最後の包含クリップを取る走査で実現
3. 勝者クリップの写像（`in` = `source_range.start()`、`out` = `source_range.end()`）:
   - `offset_ticks = at.value() - range.start().value()` — 同一 base 上の
     正確な i64 差分（`checked_sub` で範囲外は `TimeOverflow`）
   - `offset = MediaDuration::new(offset_ticks, timeline.base())`
   - `source_time = source_range.start().checked_add(offset)` —
     `checked_add` が `offset` を `source_range` の base へ
     `Rounding::NearestEven` で 1 回変換して加算する
   - `ActiveClip.source_range` は到達可能な残区間
     `[source_time, min(out, in + range.duration()))` — 1:1 写像で到達
     できるソース末尾は `in + range.duration()` であり、`source_range` が
     より長い場合の末尾側は到達不能として報告しない。算出は
     `source_range.start().checked_add(range.duration())` と `out` の比較で
     行い（media-core の checked 演算経路）、overflow は `TimeOverflow`
   - 丸め回数: `at` 変換の 1 回 + `checked_add` 呼出しごとに最大 1 回
     （本経路では `source_time` 算出と残端 `in + range.duration()`
     算出の 2 呼出し → 最大 3 回）。cross-base 算術が誤差を暗黙に
     吸収するのではなく、変換箇所と丸めモードをこの経路で固定する
4. ギャップ（`at` を含むクリップなし）・無効トラックは `active` に要素を出さない。
   無音/黒の生成は consumer が行う
5. 時刻扱い: timeline base は `Timeline` が保持し、格納 `range` は全てその
   base 揃え — `source_range` のみがソース固有 base を保持する。base 間の
   写像は上記の明示変換経路のみで行い、audio/video 種別ごとの個別規則を
   持たない
6. overflow: `at` 変換・`offset_ticks` 差分・`checked_add`/`checked_end` が
   i64 範囲を超える場合は `Error::TimeOverflow` — `evaluate` は `Result`
   を返す

## Non-Functional Constraints

- 全 API 非 RT — `evaluate` は `Vec` 確保を含む。RT パスからの呼び出し禁止を
  crate doc で明記（`rt_` テスト対象外）
- `evaluate` は O(トラック数 × トラック内クリップ数) の線形走査。
  大規模タイムライン向けの区間索引は需要が確定してから検討する
- `Timeline` は `Send` 不要求 — 並行アクセスは呼出し側責務
  （`InProcessHost` と同規約）
- 依存構造: foundations + Primitives のみ下向き利用。engine への依存を
  作らないことで射影の中立性を構造的に保証する。許可依存に
  `ruviform-audio-core`/`ruviform-video-core` を含むが、最小属性セットでは
  `media-core` の時間型が主たる依存 — 不要な依存は追加しない（YAGNI）

## Risks and Trade-offs

- **オーバーラップ許容**: 編集中の一時的な重複を正当なデータとして扱えるが、
  authoring ミスをモデルが検出しない — defined resolution（後開始優先）で
  決定性は保証し、禁制を求めるアプリは `Track::clips()` で挿入前に自前検査する
  （policy をモデルへ持ち込まない境界規則）
- **`range.duration() <= source_range.duration()` 不変条件**: 1:1 写像のみを
  モデル化し、速度変更を初期スライスから除外する。両区間の比を rate とする
  拡張は `Clip`/`ClipSpec`/`ClipAttributes` へのフィールド・variant 追加を要し、
  enum レベル `#[non_exhaustive]`（ADR public-enum-non-exhaustive-policy）が
  非破壊化するのは新規 variant の追加のみ — 既存型へのフィールド増設は
  ポリシー上の破壊的変更となる。到達経路は新しい spec 型の追加、または
  `publish = false` 期間中の明示的 breaking change のいずれか
- **`ActiveClip` は値スナップショット**: 評価後の編集は次回 `evaluate` に
  反映される逐次モデル。consumer は `TrackId`/`ClipId` で
  `Timeline::track()`/`clip()` 経由の追参照ができるため、射影へ最小限の
  情報のみ複製する
- **独自 `Error`（`HostError` 規約）を採用し `media_core::Error` 直接再利用は
  不採用**: `*NotFound`・`KindMismatch` 等のドメイン失敗を表現する必要があり、
  `InvalidValue(&'static str)` のみでは失敗の区別が付かない。`Copy` を維持し
  boxed source は持たない（backend 診断を運ばない — media-io 規約は不採用）。
  `From<media_core::Error>` で値エラーを `InvalidValue`/`TimeOverflow` へ正規化

## Deferred Design Concerns

- トランジション/エフェクト属性 | コンポジットカーネル未実装 | video-proc カーネル追加 Issue 後 | 射影スキーマの拡張差分
- トラック単位属性のうち `enabled` 以外（トラック gain/pan 等） | ADR D1-a はトラック属性をモデル責務に含む — 本設計は `enabled` のみを範囲内とする**部分範囲縮小**として記録する | 音声ミキシング/コンソール要件の発生（video-proc カーネル追加が契機ではない） | `Track` 属性追加・射影スキーマの拡張差分
- 再生速度・逆再生 | 1:1 写像で現要件を充足 | NLE 要件確定時 | 写像規則・不変条件の再設計
- 区間評価 `evaluate_range`（ブロック単位セグメント列） | 点評価の連続呼出しで代替可能 | プリフェッチ/キャッシュ要求の発生 | API 追加差分
- トラック順の変更 op・クリップ単位の enabled | 末尾追加＋削除/再挿入で代替可能 | UI からの直接要求 | API 追加差分
- 直列化/プロジェクト形式・ネストシーケンス | 形式・要件未確定 | アプリ層要求発生時 | 依存追加・モデル階層化

## 実装単位

- `crates/ruviform-edit` 新設（workspace 追加・`publish.workspace = true` 継承）
- ファイル分割案: `lib.rs`（crate doc + re-export）・`model.rs`
  （`Timeline`/`Track`/`Clip`/ID newtype・mutation）・`eval.rs`
  （`Evaluation`/`ActiveClip`/`evaluate`）・`error.rs`（`Error`/`Result`）

## References

- docs/specs/edit-data-model.md（本設計の要求元）
- docs/designs/second-target-design.md（責務分割・境界規則・後続 Issue 分解）
- docs/adr/second-target-editing-model.md（D1-a 非破壊モデル・D3-a pull 射影）
- docs/designs/ruviform-architecture-design-baseline.md（層構造・依存方向ルール）

## Verification Notes

- [x] ADR D1-a のデータモデル構成（タイムライン/クリップ/トラック +
  ソース参照 + in/out + 属性）を型設計へ写像済み — トラック属性は
  **部分対応**（`enabled` のみ範囲内。gain/pan 等は Deferred Design
  Concerns に部分範囲縮小として記録）
- [x] D3-a の中立射影 — `Evaluation`/`ActiveClip` が engine 型を含まないことを
  依存構造で保証
- [x] 境界規則 — undo/redo・UI・素材管理をモデル外に維持、依存は
  foundations + Primitives のみ
- [x] オーバーラップ・ギャップ・半開境界・写像規則が defined かつ決定的
