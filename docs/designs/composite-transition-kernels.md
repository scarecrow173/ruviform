---
id: "DESIGN-034KScKaiUBXz4qKJZqM3h"
type: "design"
status: "approved"
title: "ruviform-video-proc コンポジット/遷移カーネル設計（blend/affine/基本遷移初期セット）"
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
    - "docs/specs/composite-transition-kernels.md"
  implemented-by: []
  depends-on:
    - "docs/specs/composite-transition-kernels.md"
    - "docs/designs/second-target-design.md"
    - "docs/adr/second-target-editing-model.md"
    - "docs/adr/video-proc-kernel-api-and-color-hdr-scope.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/edit-data-model.md"
    - "docs/designs/edit-data-model.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/composite-transition-kernels.md"
    - "docs/designs/second-target-design.md"
    - "docs/adr/second-target-editing-model.md"
    - "docs/adr/video-proc-kernel-api-and-color-hdr-scope.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "docs/plans/implement-issue-97-composite-kernels.md"
    - "docs/test-specs/composite-transition-kernels-tests.md"
    - "docs/specs/video-proc-rt-verification-lane.md"
    - "docs/plans/implement-issue-126-video-proc-rt-lane.md"
  verifies: []
  verified-by:
    - "docs/test-specs/composite-transition-kernels-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# ruviform-video-proc コンポジット/遷移カーネル設計（blend/affine/基本遷移初期セット）

## Context

Issue #97 — 第 2 ターゲット設計の後続 Issue 分解・第 2 項目。
ADR `second-target-editing-model` D2-a（blend/transform/transition は
`ruviform-video-proc` の RT カーネル）と、video-proc カーネル ADR の
D1-b（非 RT prepare → alloc-free `process` の二相形状）・D2（合成・
スケーリングは後続 Issue = 本 Issue）・D7（`ruviform_media_core::Error`
再利用）を、初期カーネル集合の具体 API・画素演算規約・等価性契約へ
具体化する。consumer は `ruviform-edit`（#96）の評価射影が出力する
`ClipAttributes::Video{opacity: f32}` と、後続の engine/射影配線。

## 決定サマリ

| 論点 | 決定 |
| --- | --- |
| 初期カーネル集合 | `CompositeOver`（source-over アルファ合成・dst in-place）・`Crossfade`（2 ソースのディゾルブ遷移）・`AffineTransform`（整数矩形写像による translate+scale・最近傍サンプリング）。個別の `Mix` カーネルは持たない — 重み付き混合は `Crossfade` と同型であり YAGNI |
| 対象フォーマット | `Rgba8`・`RgbaF16` のみ（`src`/`dst` 間で一致必須）。planar YUV は対象外 — アルファを持たず、クロマ格子を考慮した設計が別論点 |
| 可変パラメータ | opacity・遷移係数 t・写像矩形は `process` の呼出し毎引数とする — アニメーションする編集パラメータであり係数ではない（後述）。ctor はパラメータを焼かず `new()` 無引数 |
| アルファ解釈 | straight（unassociated）alpha — FFmpeg 系 packed RGBA の慣例。premultiplied は非対象・後続 |
| 作業色空間 | 符号空間（code space）のまま合成 — transfer 線形化・gamut 変換を行わない（D3-a 非破壊搬送と同じ境界。linear-light 合成は色管理 Issue） |
| 等価性契約 | `Rgba8` 演算は全て整数演算で **bit-exact**。`RgbaF16` の合成演算はスカラー参照実装を規範とする **ulp-bound**（f32 ドメイン評価 + RNE binary16 丸め — 最適化 backend はチャネル毎 ≤ 2 ulp。`oa` が binary16 denormal 域となる微弱アルファ領域は `(0,0,0,0)` ガード結果として契約から切離す）。`AffineTransform` は画素値を演算しない写像/複写のみで両フォーマット **bit-exact** |
| エラー | `ruviform_media_core::Error`（ADR D7）。`InvalidValue(&'static str)` で全検証を書込み前に実施 — panic-free・部分書込みなし |
| メタデータ | `CompositeOver` は `dst` の `ColorInfo`/`pts`/`duration` を不変（in-place キャンバス）。`Crossfade` は第一ソース `from` の値を `dst` へ伝播。`AffineTransform` は `src` の値を `dst` へ伝播 |
| RT 検証 | 既存カーネルと同じ `rt_` 規約 — `rt_process_*` 命名の determinism・source 非破壊・仕様上の alloc-free（レビュー担保）テスト。`assert_no_alloc` 依存は本クレートに未導入であり本設計でも追加しない — 機械検出レーンの適用は次の RT 保証強化 Issue をトリガとするフォローアップ |

## Scope

### In Scope

- `CompositeOver` カーネル — source-over アルファ合成（`dst` を基底層
  として in-place に更新）+ 呼出し毎の `opacity`
- `Crossfade` カーネル — `from`/`to` 2 ソースの加重混合遷移 +
  呼出し毎の係数 `t`
- `AffineTransform` カーネル + `Rect`/`RectMap` パラメータ型 —
  translate+scale（対角アフィン）+ ソース矩形 crop + 最近傍
  サンプリング + 範囲外 transparent black 充填
- 各カーネルの `**RT contract:**` ラベル・等価性契約・メタデータ規約・
  エラー規約の doc 明記
- golden vector・端点・エッジケースの単体テストと `rt_` 命名の
  RT 契約テスト
- crate doc（`lib.rs`）のカーネル一覧更新（予約記述の差替え）

### Out of Scope

- GPU 経路・OFX 写像・追加 blend モード・linear-light 合成・
  フルアフィン（回転/せん断）・bilinear 以上のフィルタ・
  premultiplied-alpha 変種・planar YUV 適用・wipe 等の追加遷移
  （spec の Out of Scope / Deferred Design Concerns と同一）
- `ruviform-edit` 評価結果・engine への配線（consumer 側責務）

## Components and Boundaries

| 型 | 形状 | 責務 |
| --- | --- | --- |
| `CompositeOver` | `new()`（無引数）+ `process(&mut self, src: &VideoFrame, dst: &mut VideoFrame, opacity: f32) -> Result<()>` | `dst` の現内容を基底層として `src` を source-over 合成する in-place 演算 — NLE のレイヤスタックを下から順に積層する用途。`opacity` は `src` アルファへの一様乗算 |
| `Crossfade` | `new()` + `process(&mut self, from: &VideoFrame, to: &VideoFrame, dst: &mut VideoFrame, t: f32) -> Result<()>` | `dst = from·(1−t) + to·t` の全 4 チャネル加重混合（アルファを含む）。遷移 A→B の進行度 `t` に対応 — dip-to-black は `to`/黒フレーム、フェードイン/アウトは不透明黒との混合で表現 |
| `AffineTransform` | `new()` + `process(&mut self, src: &VideoFrame, dst: &mut VideoFrame, map: RectMap) -> Result<()>` | `map.dst` 矩形内の `dst` 画素を `map.src` 矩形へ逆写像し最近傍サンプリング。`dst` は全画素を書換え（矩形外は transparent black）— 変換のみの完結した出力フレームを生成 |
| `Rect` | `{x: i32, y: i32, w: u32, h: u32}` — pub フィールドの plain data（不変条件は `process` 検証で強制）。`Copy`/`Clone`/`Debug`/`PartialEq`/`Eq` を derive | 写像矩形。左上原点・整数画素座標、`w`/`h` は画素数。`dst` 側の `x`/`y` は負値を許容（キャンバス外への配置） |
| `RectMap` | `{src: Rect, dst: Rect}` — pub フィールドの plain data | `src` 矩形 → `dst` 矩形の矩形写像。縮尺は `dst.w/src.w`・`dst.h/src.h` の軸毎の有理数比（整数比に限らない） |

### `CompositeOver` 規約

- 意味: `dst' = src over dst`（Porter-Duff source-over、straight
  alpha）。`opacity` はソースの実効アルファへの一様乗算
  （`ClipAttributes::Video{opacity}` の消費形）。
- パラメータ正規化: `opacity` は `is_finite()` かつ `[0.0, 1.0]` —
  違反は `Error::InvalidValue("opacity")`。`w = round(opacity·255)`
  の Q8 重みへ呼出し毎に 1 回だけ量子化し、両ピクセルパス
  （`Rgba8`/`RgbaF16`）が同じ量子化重みを使う
- `Rgba8` 演算（全て整数・round-half-up）: 実効ソースアルファ
  `sa = round(a_s·w/255)`。backdrop 項 `b = round(a_b·(255−sa)/255)`
  は画素毎に 1 回だけ計算し、出力アルファ `oa = sa + b` と色
  チャネル `oc = round((c_s·sa + c_b·b)/oa)` の両方で再利用する
  （標準形 `out = (c_s·âs + c_b·âb·(1−âs))/ôa` の自己無撞着な
  Q8 展開）。`c_s`,`c_b` ≤ 255 より分子 ≤ `255·oa` — `oc ≤ 255`
  が構造的に保証されクランプは不要。`oa == 0` なら出力は
  `(0,0,0,0)`。中間値の最大は `c_s·sa + c_b·b ≤ 255·oa ≤ 130,050`
  で全て `u32` に収まる
- `RgbaF16` 演算: 同一式を f32 ドメインで評価し binary16 へ丸める。
  量子化は呼出し毎のスカラー重み `w` のみで、実効ソースアルファは
  `sa = a_s·(w/255)`（f32）。画素値（`a_s`/`a_b`/`c_s`/`c_b`）は
  f16 デコード後の連続値を使い 257 段への再量子化は行わない
  （HDR の画素精度を損なうため）。`b = a_b·(1−sa)`、`oa = sa + b`、
  `oc = (c_s·sa + c_b·b)/oa`。`oa == 0.0` なら `(0,0,0,0)` —
  `Rgba8` パスと同一のガード。チャネルはクランプしない（HDR の
  1.0 超を保持）。等価性契約: 参照実装（上記 f32 パイプライン +
  RNE binary16 丸め）を規範とし、最適化 backend はチャネル毎
  ≤ 2 ulp（binary16）を満たす。`oa < 2⁻¹⁴`（binary16 の最小
  normal 値）の denormal 領域は除算が ill-conditioned となるため
  契約から切離し、定義済み出力はガード結果 `(0,0,0,0)` とする。非有限
  チャネル値（NaN/±Inf）は f32 セマンティクスどおり伝播させ、
  等価性契約の適用外とする（フレーム画素は検証対象のパラメータ
  ではなく入力データ）
- 検証: `src.format() == dst.format()` ∈ {`Rgba8`, `RgbaF16`} かつ
  寸法一致 — 違反は `InvalidValue("src format")`/`"dst format"`/
  `"dimensions"`。共有 `dst` backing は `InvalidValue("dst plane")`
- `dst` の `ColorInfo`/`pts`/`duration` は不変 — in-place キャンバス
  の意味を保つ。`src`/`dst` の `ColorInfo` 一致は検証しない
  （符号空間演算 — 解釈は consumer の責務）

### `Crossfade` 規約

- 意味: `dst = from·(1−t) + to·t` を RGBA 全 4 チャネルに適用する
  ディゾルブ。`t = 0` で `dst` は `from` を、`t = 1` で `to` を
  再現する端点再現性を持つ — `Rgba8` は bit-exact、`RgbaF16` は
  有限チャネルを数値的に再現（`−0.0` は `+0.0` へ・NaN ペイロード
  は `0x7E00` へ正規化され得る）
- パラメータ正規化: `t` は `is_finite()` かつ `[0.0, 1.0]` — 違反は
  `InvalidValue("t")`。`tq = round(t·255)` の Q8 重みへ呼出し毎に
  1 回だけ量子化し、両ピクセルパスが同じ量子化重みを使う
- `Rgba8` 演算（整数・round-half-up）: `oc = round((a_c·(255−tq) + b_c·tq)/255)`
- `RgbaF16` 演算: `dst = from·(1 − tq/255) + to·(tq/255)` を f32
  ドメインで評価し binary16 へ丸める — 量子化は呼出し毎の `tq`
  のみで画素チャネルは連続値。等価性契約は `CompositeOver` と同一
  （参照実装規範・最適化 backend ≤ 2 ulp・非有限チャネル値は
  契約外）
- 全 4 チャネルを straight のまま lerp するため、`from`/`to` の
  アルファが異なる区間では premultiplied 前提の lerp と結果が
  異なる（不透明クリップ同士では一致。premult 変種は Deferred
  Design Concerns どおり後続論点）
- 検証: 3 フレームともフォーマット一致（{`Rgba8`, `RgbaF16`}）かつ
  寸法一致 — 違反は `InvalidValue`。共有 `dst` は `InvalidValue("dst plane")`
- `dst` の `ColorInfo`/`pts`/`duration` には `from` の値を伝播する
  （規約: メタデータは第一ソース由来。consumer が別の時刻を要する
  場合は `set_pts` 等で明示上書き）

### `AffineTransform` 規約

- 意味: `dst` 全画素を書換える完結変換。`dst` 矩形内の画素 `(x, y)`
  について `offset = x − map.dst.x`（`0 ≤ offset ≤ map.dst.w−1`）と
  置くと、`src` 側座標は
  `sx = map.src.x + (2·offset·map.src.w + map.src.w) / (2·map.dst.w)`
  — `floor((offset + 0.5)·map.src.w/map.dst.w)` の正確な整数評価に
  よるセル中心揃えの最近傍（`sy` も同形）。`offset ≤ map.dst.w−1`
  より商は常に `< map.src.w` — `sx`/`sy` は構造的に `map.src`
  矩形内に収まりサンプル時のクランプは不要（例: 1→2 拡大では
  offset `[0,1]` → `src` 画素 `[0,0]` の複製となる）。`dst` 矩形外の
  画素は transparent black `(0,0,0,0)` を書く
- 算術幅: 座標・矩形フィールドは宣言型どおり `x`/`y` は `i32`・
  `w`/`h` は `u32` とし、写像演算（`2·offset·map.src.w` 等の積を
  含む全スカラー計算）は `i128`/`u128` で評価する — いかなる
  フレーム寸法・矩形値でもオーバーフローしない。`x+w ≤ width` 等の
  境界検査は加算による桁上がりを避けるため `i64` へ拡幅してから
  評価する
- 検証: `map.src` は非空（`w`/`h` > 0）かつ `src` フレーム内に収まる
  （`x`/`y` ≥ 0 かつ `x+w ≤ width`・`y+h ≤ height` — `i64` 拡幅後に
  評価）— 違反は `InvalidValue("src rect")`。`map.dst` の非空・
  キャンバス交差は **エラーとしない** — 空矩形または完全に
  キャンバス外の場合は `dst` 全画素が transparent black となる
  定義済み結果（有界・panic-free を維持）。フォーマット一致・
  共有 `dst` は既存規約どおり `InvalidValue`
- `src`/`dst` の寸法一致は要求しない — 変換こそが寸法を変える
  カーネル（例: `src` 640×360 → `dst` 1920×1080 キャンバス）
- `dst` の `ColorInfo`/`pts`/`duration` には `src` の値を伝播
  （単一フレームの幾何変換 — 解釈・時刻は変えない）

## Data and Control Flow

### 合成規約（カーネル族共通）

- 二相形状は既存カーネルと同一: `new()` は非 RT（本集合は焼付ける
  係数を持たないため trivial — `Rgba8ToRgbaF16` と同じ unit 形状 +
  `Default` impl）。`process` は alloc-free・非ブロッキング・有界で
  `**RT contract:** RT-suitable` ラベルを持つ
- **呼出し毎パラメータ**: opacity/`t`/`RectMap` は `process` の引数。
  これらは係数ではなくフレーム毎にアニメーションする編集入力 —
  ctor 焼付けではフレーム毎の再 prepare が必要となり、非 RT ctor を
  RT パスで呼ぶ違反を誘発する。`process(src, dst)` 二引数形状からの
  逸脱は「コンポジット族は可変パラメータを `process` が取る」規約
  として本設計で定義する（quantization は呼出し毎 1 回のスカラー
  演算 — alloc-free・bounded を維持）
- `process` の検証は全て書込み前に実施 — 失敗呼出しは `dst` を
  部分書込みしない（既存規約と同一）
- NLE 合成パターン: `AffineTransform` で `src` をキャンバス寸法の
  中間フレームへ配置 → `CompositeOver` で積層。中間フレームの確保は
  呼出し側（非 RT 層）の責務 — カーネルは確保しない
- 遷移パターン: `Crossfade(from, to, dst, t)` が 1 パスで完結 —
  `dst` への事前コピーは不要

### エッジケース

- `CompositeOver`: `opacity = 0`（`w == 0`）は検証通過後・画素
  書込み前に early return — `dst` を一切書換えないため `dst`
  不変が実際の早期復帰として保証され、transparent `dst` の RGB を
  正準化することもない。`oa = 0` → transparent black 画素。
  共有 `dst`（clone 保持中）は拒否
- `Crossfade`: `t = 0`/`t = 1` で入力を再現（`Rgba8` は bit-exact、
  `RgbaF16` は `−0.0` → `+0.0`・NaN ペイロード → `0x7E00` の正規化
  を除き数値的一致）。`from`/`to`/`dst` のフォーマット・寸法の
  不一致は拒否
- `AffineTransform`: `map.dst` がキャンバス外/空 → 全画素 transparent
  black。拡大（`dst` 矩形 > `src` 矩形）は同一 `src` 画素の複製、
  縮小は間引き — いずれも有界。`map.src` の境界逸脱・空矩形は拒否
- 全カーネル: `stride > row_bytes` のパディングは読み書きしない
  （`row`/`row_mut` 経由のみ）、`RgbaF16` の 8byte 画素は
  little-endian `binary16`×4 として解釈（`rgba_f16.rs` の規約）

## Non-Functional Constraints

- 全カーネルはスカラー参照実装。`Rgba8` 演算は整数のみで bit-exact、
  `RgbaF16` 演算は f32 評価で ulp-bound — 等価性契約はカーネル doc に
  明記（将来の最適化 backend の検証基盤）
- `process` の計算量は `width`×`height` 画素の線形走査で有界。
  内部確保・ブロッキング・ロックなし
- `rt_` 命名規約: `rt_process_*`（repeatable/deterministic・
  never-mutates-source・仕様上の alloc-free はレビュー担保）を
  各カーネルに持ち `mise run test-rt` で選択可能。非 RT 系テスト名に
  `rt_` を含めない。`assert_no_alloc` による機械検出は本クレートへ
  未導入の依存を要するため本設計では採用しない — 導入は次の
  RT 保証強化 Issue をトリガとするフォローアップ
- 後続 test-spec へ引き渡す trap vector: `CompositeOver` の
  `oc > 255` コーナー（`sa = 254`/`a_b = 127`・`sa = 1`/`a_b = 128`
  の各ケースで出力が上限 255 ちょうどに収まること）・
  `AffineTransform` の 1→2・2→4 拡大における最終列の複製・
  `opacity = 0` の early return・`map.dst.x` 負値・共有 backing 拒否
  （3 カーネル全て — `Crossfade` は `from == to` の同一フレーム
  参照を許容しつつ `dst` の共有 `Arc` は拒否）・`RgbaF16` 非有限
  チャネル値の伝播
- 依存は `ruviform-video-core`・`ruviform-media-core` のみ —
  `ruviform-rt`・同順位・engine 依存を追加しない
- 公開 enum 追加なし（`Rect`/`RectMap` は struct — exhaustive_structs
  の crate レベル allow 済み）。`PixelFormat` 等の未知 variant は
  `_` arm で `InvalidValue` へ落とす graceful 処理

## Risks and Trade-offs

- **`process` 可変引数への逸脱**: 色変換カーネルの `(src, dst)` 形状と
  異なるが、opacity/t/矩形はアニメーション入力であり係数ではない。
  ctor 焼付け案はフレーム毎の非 RT prepare を RT パスへ誘引するため
  棄却 — 二相分離の契約（係数・ワーク領域の prepare / alloc-free
  process）は維持される
- **`CompositeOver` の in-place 設計**: `dst` が入出力を兼ねるため
  「`src` を読み `dst` に書く」単純モデルから外れるが、レイヤ
  スタックの逐次積層は NLE 合成の自然形であり、基底フレームを別引数
  にすると中間フレームの確保が必須化して RT 経路を複雑化する。
  `src` 非破壊・`dst` 読み書きの規約は doc 明記で担保
- **`Crossfade` の 2 ソース引数**: in-place（`dst` に `from` を
  事前コピー）も可能だが、遷移は 2 フレームの純関数であり明示的な
  引数が正直な arity。事前コピーは余分な全画素パスを毎フレーム
  要求する
- **straight alpha 選択**: premultiplied の方が GPU 合成では主流だが、
  デコード/codec 経由フレームは straight が自然（FFmpeg `rgba` 慣例）。
  混在合成を契約外とし、premult 変種は後続論点とする
- **符号空間合成の視覚誤差**: ガンマ符号化領域での加重混合は
  物理光量では線形でない — 既知の近似として doc 明記し、正しい
  linear-light 合成は色管理 Issue（f32 linear working space）へ譲る
- **最近傍サンプリング**: NLE プレビュー品質としては bilinear 以上が
  望ましいが、参照実装の最小・決定的実装として最近傍を採用。品質
  フィルタの追加は等価性契約の別制定とともに後続 Issue
- **メタデータ非検証**: `src`/`dst` の `ColorInfo` 不一致を検査しない
  と解釈の異なるフレームを合成し得るが、`ColorInfo` の等価強制は
  `Unspecified` 混入の一般的ケースを過剰拒否する — 符号空間演算で
  あることを doc 明記し、解釈の整合は consumer 責務とする

## Deferred Design Concerns

- blend 演算の拡張セット（Porter-Duff 全演算・multiply/screen 等） | 初期 `CompositeOver` で NLE 最小要件を充足 | NLE 要件の確定 | 演算選択の API 拡張差分
- linear-light コンポジット（transfer 線形化 + gamut 変換） | 広範色管理は別責務 — 色変換カーネル実装 Issue（第 2 ターゲット分解 #4） | 色管理 Issue 着手 | 符号空間合成の視覚誤差が残る
- フルアフィン（回転/せん断・任意 2×3 行列）・bilinear 以上フィルタ | 整数矩形 + 最近傍で配置要件を充足。浮動小数点写像は等価性契約の再制定を要する | NLE 要件の確定 | 変換表現力・品質の不足
- premultiplied-alpha 変種 | straight で初期要件を充足 | GPU 経路・外部バッファとの整合要求 | alpha 解釈の API 表面拡大
- wipe/dip-to-black 等の遷移追加 | `Crossfade` で dip-to-black は導出可能。wipe は幾何パターン設計が別論点 | NLE 要件の確定 | 遷移カバレッジ不足
- planar YUV へのコンポジット/変換適用 | クロマ格子・siting を考慮した設計が別論点 | YUV 直接処理の要求発生 | RGB 変換を介する二段処理が必要
- video-proc への `assert_no_alloc` 導入 | 本設計の `rt_process_*` は determinism・source 非破壊・レビュー担保の alloc-free のみを検証し機械検出は行わない — レーン適用は RT 検証レーン拡大の別 Issue 管轄 | 次の RT 保証強化 Issue | alloc 違反の機械検出なし — **Issue #126 で解決済み**: 全 `rt_process_*` へ `assert_no_alloc` 窓を適用し per-process alloc 0 を実測（IR: [issue-126-video-proc-rt-lane-implementation.md](../impl/ir/issue-126-video-proc-rt-lane-implementation.md)）

## 実装単位

- `crates/ruviform-video-proc/src/lib.rs` — カーネル一覧の更新・
  re-export 追加・「Scaling and compositing kernels stay reserved」
  記述の差替え。「Kernel API shape」段落へコンポジット族規約を
  追記 — 基本形 `process(src, dst)` に加え、可変アニメーション
  パラメータを `process` が取る `process(src, dst, opacity)` /
  `process(from, to, dst, t)` / `process(src, dst, map)` の形状を
  カーネル族規約として記録する
- `src/composite.rs` — `CompositeOver` + `Crossfade`（Q8 重み
  量子化・straight-alpha 演算を共有）。`rgba_f16.rs` の
  `f64_to_f16_bits`/`f16_bits_to_f64` は現状 private のため
  `pub(crate)` へ格上げ（または同等コーデックの複製）して再利用する
- `src/affine.rs` — `Rect`・`RectMap`・`AffineTransform`
- テストは各モジュールの `#[cfg(test)] mod tests`（golden vector・
  端点・検証拒否・`rt_process_*`）— 既存規約どおり inline

## References

- docs/specs/composite-transition-kernels.md（本設計の要求元）
- docs/designs/second-target-design.md（責務分割・後続 Issue 分解）
- docs/adr/second-target-editing-model.md（D2-a コンポジット =
  video-proc カーネル）
- docs/adr/video-proc-kernel-api-and-color-hdr-scope.md（D1-b 二相
  API・D2 合成予約・D7 エラーモデル）
- docs/designs/ruviform-architecture-design-baseline.md（層構造・
  依存方向・RT 契約・DSP/SIMD 決定）
- docs/specs/edit-data-model.md / docs/designs/edit-data-model.md
  （`ClipAttributes::Video{opacity}` を出力する consumer 側 —
  本カーネルへの配線は同 spec の deferred concern）

## Verification Notes

- [x] ADR D2-a — blend/transform/transition が `ruviform-video-proc`
  の RT カーネルとして位置付けられ、色変換カーネルと同じ二相 API・
  RT 契約規約に整合
- [x] ADR D1-b — 二相形状を維持（可変アニメーションパラメータは
  `process` 引数へ拡張）。`RtContext`・`Processor` trait は
  参照しない
- [x] ADR D7 — `ruviform_media_core::Error`/`Result` を再利用。
  検証は書込み前・panic-free・部分書込みなし
- [x] 初期集合の最小性 — `CompositeOver`/`Crossfade`/`AffineTransform`
  の 3 カーネルで D2-a の blend/affine/transition をカバーし、
  重複する `Mix`・導出可能な dip-to-black・未確定の wipe を除外
- [x] 等価性契約 — 整数演算は bit-exact、浮動小数点経由は ulp-bound
  の規則が baseline DSP/SIMD 決定と整合
- [x] 依存構造 — `ruviform-video-core`・`ruviform-media-core` のみ。
  同順位・`ruviform-rt`・engine 依存なし
