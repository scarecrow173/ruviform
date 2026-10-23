---
id: "TSPEC-034KSVh4JNdOEJMzGYbZic"
type: "test-spec"
status: "approved"
title: "video-proc コンポジット/遷移カーネル テスト仕様"
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
    - "docs/designs/composite-transition-kernels.md"
  derived-by: []
  verifies:
    - "docs/designs/composite-transition-kernels.md"
    - "docs/specs/composite-transition-kernels.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# video-proc コンポジット/遷移カーネル テスト仕様

## Purpose

Issue #97（`ruviform-video-proc` コンポジット/遷移カーネル初期セット —
`CompositeOver`・`Crossfade`・`AffineTransform` + `Rect`/`RectMap`）の
受入契約を固定する。design が確定した演算規約（`Rgba8` の自己無撞着
Q8 整数パイプライン・`RgbaF16` の f32 評価 + RNE binary16 丸め・
セル中心最近傍写像）と検証/エラー規約（書込み前検証・panic-free・
部分書込みなし）が将来の変更・最適化 backend 導入で退行しないことを
保証する。本カーネル群が置換・再設計された場合に廃止してよい。

## Feature

`VideoFrame` 上の 3 つのスカラー参照カーネル: `CompositeOver`
（`process(src, dst, opacity)` — `dst` を基底層とする source-over
in-place 合成）、`Crossfade`（`process(from, to, dst, t)` — 全 4
チャネル加重混合ディゾルブ）、`AffineTransform`（`process(src, dst,
map)` — `Rect`/`RectMap` による translate+scale・最近傍・
transparent-black 充填の完結変換）。対象フォーマットは `Rgba8`・
`RgbaF16` のみ。

## Rules

- `CompositeOver`（`Rgba8`、全て整数・round-half-up）: `w =
  round(opacity·255)` の Q8 重みへ呼出し毎に 1 回だけ量子化。実効
  ソースアルファ `sa = round(a_s·w/255)`、backdrop 項 `b =
  round(a_b·(255−sa)/255)` を画素毎に 1 回だけ計算し `oa = sa + b`・
  `oc = round((c_s·sa + c_b·b)/oa)` の両方で再利用する。`c_s`,`c_b` ≤
  255 より分子 ≤ `255·oa` — `oc ≤ 255` は構造的に保証されクランプ
  しない。`oa == 0` なら出力は `(0,0,0,0)`
- `CompositeOver`（`RgbaF16`）: 同一式を f32 ドメインで評価し
  binary16 へ RNE 丸め。量子化は呼出し毎の `w` のみ（`sa =
  a_s·(w/255)` — 画素値は連続値を使い 257 段へ再量子化しない）。
  `oa == 0.0` → `(0,0,0,0)`。チャネルはクランプしない（HDR の
  1.0 超を保持）。`oa < 2⁻¹⁴`（binary16 最小 normal）の denormal 領域は
  ill-conditioned のため契約から切離し、定義済み出力は `(0,0,0,0)`。
  非有限チャネル値（NaN/±Inf）は f32 セマンティクスどおり伝播
- `opacity`/`t` は `is_finite()` かつ `[0.0, 1.0]` — 違反は
  `InvalidValue("opacity")`/`InvalidValue("t")`（NaN・負値・1 超・
  ±Inf）。全検証は書込み前に完了し、拒否呼出しは `dst` を一切
  変更しない（部分書込みなし）
- `CompositeOver` の `opacity = 0`（`w == 0`）は検証通過後・画素
  書込み前に early return — `dst` を一切書換えない（transparent
  `dst` の RGB を正準化することもない）。検証は early return に
  先行するため、共有 `dst` backing は `opacity = 0` でも拒否される
- `Crossfade`（`Rgba8`、整数・round-half-up）: `tq = round(t·255)` へ
  呼出し毎に 1 回量子化し `oc = round((a_c·(255−tq) + b_c·tq)/255)`。
  `RgbaF16` は `from·(1 − tq/255) + to·(tq/255)` を f32 評価 +
  binary16 丸め。`t = 0`/`t = 1` で入力を再現する端点再現性を持つ —
  `Rgba8` は bit-exact、`RgbaF16` は有限チャネルを数値的に再現
  （`−0.0` は `+0.0` へ・NaN ペイロードは `0x7E00` へ正規化され得る）。
  `from`/`to` の同一フレーム参照は許容（両者とも immutable borrow）—
  `dst` の共有 `Arc` backing のみ拒否
- `AffineTransform`: `dst` 矩形内の画素 `(x, y)` について `offset =
  x − map.dst.x` と置き、`sx = map.src.x + (2·offset·map.src.w +
  map.src.w) / (2·map.dst.w)`（`sy` 同形）— `floor((offset +
  0.5)·map.src.w/map.dst.w)` の正確な整数評価によるセル中心揃え
  最近傍。`offset ≤ map.dst.w−1` より商は常に `< map.src.w` —
  サンプル座標は構造的に `map.src` 内に収まりクランプ不要。全スカラー
  写像演算は `i128`/`u128`、`x+w ≤ width` 等の境界検査は `i64` 拡幅で
  評価する（いかなるフレーム寸法・矩形値でもオーバーフローしない）
- `map.src` は非空（`w`/`h` > 0）かつ `src` フレーム内に収まる
  （`x`/`y` ≥ 0・`x+w ≤ width`・`y+h ≤ height`）— 違反は
  `InvalidValue("src rect")`。`map.dst` の非空・キャンバス交差は
  **検証しない** — 空矩形・完全キャンバス外はエラーとせず `dst`
  全画素が transparent black となる定義済み結果
- フォーマット規約: `src`/`dst`（`Crossfade` は `from`/`to`/`dst`
  3 者）の `format()` が一致し {`Rgba8`, `RgbaF16`} — 違反は
  `InvalidValue("src format")`/`"dst format"`/`"to format"` 等の
  `InvalidValue`。`CompositeOver`/`Crossfade` は寸法一致必須
  （`InvalidValue("dimensions")`）— `AffineTransform` は寸法一致を
  要求しない（変換が寸法を変える）。未知 `PixelFormat` variant は
  `_` arm で `InvalidValue` へ graceful に落とす
- 共有 backing 拒否: 全 3 カーネルで `plane_mut` が `None` を返す
  共有 `dst`（clone 保持中の `Arc` backing）は `InvalidValue("dst
  plane")` で書込み前に拒否
- メタデータ: `CompositeOver` は `dst` の `ColorInfo`/`pts`/`duration`
  を不変（in-place キャンバス）。`Crossfade` は `from` の値を `dst`
  へ伝播。`AffineTransform` は `src` の値を `dst` へ伝播
- 等価性契約: `Rgba8` 演算・`AffineTransform` 全体は bit-exact、
  `RgbaF16` 合成演算は参照実装規範の ulp-bound（最適化 backend は
  チャネル毎 ≤ 2 ulp・binary16）
- `stride > row_bytes` のパディングは読み書きしない（`row`/`row_mut`
  経由のみ）。`RgbaF16` の 8 byte 画素は little-endian `binary16`×4
- RT 契約: 全 `process` は `**RT contract:**` ラベル付きで
  alloc-free・非ブロッキング・有界。`rt_process_*` 命名の RT 契約
  テスト（repeatable/deterministic・ソース非破壊・仕様上の
  alloc-free はレビュー担保）を各カーネルに持ち `mise run test-rt`
  で選択可能。非 RT 系テスト名に `rt_` を含めない
  （`assert_no_alloc` は本クレートに導入しない — 機械検出は後続
  Issue）

## Examples

- `CompositeOver` golden（`Rgba8`・`opacity = 1.0` → `w = 255`）:
  不透明 `src`（`a_s = 255`）は `sa = 255`・`b = 0`・`oa = 255`・
  `oc = c_s` で `dst` を完全上書き。透明 `src`（`a_s = 0`）は
  `sa = 0`・`b = a_b`・`oc = c_b` で `dst` を画素値そのまま維持
- `oc` 境界 trap vector（`w = 255` 時）: `a_s = 254`/`a_b = 127`
  （`sa = 254`・`b = round(127/255) = 0`・`oa = 254` — `c_s = 255`
  で `oc` は上限 255 ちょうどに収まり overflow しない）と
  `a_s = 1`/`a_b = 128`（`sa = 1`・`b = round(128·254/255) = 127`・
  `oa = 128` — `c_s = c_b = 255` で分子は `255·oa` の構造的上限に
  達し `oc = 255` ちょうど）の 2 ケース
- `oa == 0` ガード: `a_s = 0`・`a_b = 0` の合成は両フォーマットで
  出力 `(0,0,0,0)`（`Rgba8` の整数域・`RgbaF16` の f32 域とも）
- `opacity = 0` early return: 検証通過後に `dst` が byte-identical に
  維持される（`src` が何を含んでも書込みなし）
- `opacity` 検証拒否: `NaN`・`−0.5`・`1.5` → `InvalidValue("opacity")`
  で `dst` 不変
- `CompositeOver` の in-place メタデータ: 合成後も `dst` の
  `ColorInfo`/`pts`/`duration` は呼出し前の値を保持
- `Crossfade` 端点: `t = 0.0` → `dst` が `from` を、`t = 1.0` → `to`
  を再現（`tq = 0`/`255`。`Rgba8` は bit-exact — `RgbaF16` は
  `−0.0` → `+0.0`・NaN ペイロード → `0x7E00` の正規化を除き有限
  チャネル一致）。中間 `t` の golden vector は `tq` 量子化込みの
  整数式で検証
- `Crossfade` 同一フレーム: `from` と `to` に同じ `&VideoFrame` を
  渡しても成功する（`dst` の clone 共有のみ `InvalidValue("dst
  plane")`）
- `Crossfade` メタデータ: `dst` の `ColorInfo`/`pts`/`duration` が
  `from` の値となる（`to` 由来ではない）
- `AffineTransform` 恒等写像: `map.src` = `src` 全体・`map.dst` =
  `dst` 全体の同寸法で `dst` が `src` の bit-exact 複製
- `AffineTransform` 拡大複製 trap vector: `src` 1 px → `dst` 2 px の
  1→2 拡大は両出力画素が `src` 画素 0 の複製（offset `[0,1]` →
  `sx` `[0,0]`）。`src` 2 px → `dst` 4 px の 2→4 拡大は
  `[0,0,1,1]` の複製で最終列が `src.x + 1` に収まる（商が
  `map.src.w` に到達しない構造的保証の実証）
- `map.dst.x` 負値: キャンバス左外へはみ出す配置は交差部分のみ
  書込み、キャンバス内の矩形外画素は transparent black
- `map.dst` の右端・下端オーバーハング: `x + w > width`・
  `y + h > height` の矩形はキャンバス端でクリップされ、交差部分のみ
  対応する `offset` で写像（クリップされた offset の列・行は
  サンプルされない）— 矩形外のキャンバス画素は transparent black
- `map.src` 検証拒否: `w = 0`/`h = 0` の空矩形・`x < 0`/`y < 0`・
  `x+w > width`/`y+h > height` の境界逸脱 → `InvalidValue("src rect")`
- `map.dst` 完全キャンバス外・空矩形: エラーなしで `dst` 全画素が
  transparent black — 有界・panic-free
- `AffineTransform` メタデータ: `dst` の `ColorInfo`/`pts`/`duration`
  が `src` の値となる
- 検証拒否（全カーネル共通）: `Rgba8`/`RgbaF16` 混在・非対象
  フォーマット（`Yuv420p` 等）・`CompositeOver`/`Crossfade` の寸法
  不一致 → `InvalidValue`。共有 `dst` backing → `InvalidValue("dst
  plane")`。いずれも `dst` は部分書込みなしで不変
- `RgbaF16` 非有限伝播: NaN/±Inf を含む入力チャネルは対応する f32
  セマンティクスの非有限結果として出力へ現れる（ulp 契約の適用外）
- パディング: `stride > row_bytes` の `dst` は行データのみ書換わり
  パディング byte は呼出し前の内容を保持
- `rt_process_*` 契約: 同一カーネルの反復呼出しは決定的で panic
  せず、`src`（`Crossfade` は `from`/`to` 両者）は呼出しを跨いで
  byte-identical — `dst` への書込みのみが副作用

## Guarantees

- 全 `process` は検証を書込み前に完了し、拒否呼出しは `dst` を
  一切変更しない（panic-free・部分書込みなし — ADR D7）
- `Rgba8` 経路は整数演算のみで完全に決定的 — 同一入力に同一出力を
  bit-exact に返す。`AffineTransform` は画素値を演算しない
  写像/複写のみで両フォーマット bit-exact
- `oc ≤ 255` は演算構造から導かれる不変条件であり、クランプ等の
  下流補正に依存しない
- `src`/`to`/`from` の immutable 参照フレームは `process` が
  変更しない — 副作用は `dst` への書込みのみ
- `process` は内部確保・ブロッキング・ロックを持たない —
  alloc-free は API 単位の `**RT contract:**` ラベルで保証され、
  レビュー担保として検証される

## Non-goals

- GPU 経路・OFX 写像・追加 blend モード（multiply/screen 等）・
  Porter-Duff 全演算・linear-light 合成・premultiplied 変種・
  フルアフィン（回転/せん断）・bilinear 以上のフィルタ・planar YUV
  適用・wipe/dip-to-black 等の追加遷移
- `assert_no_alloc` による alloc の機械検出 — 本クレートへ未導入の
  依存を要するため design で後続 Issue へ明確に defer 済み
- `ruviform-edit` 評価結果や engine への配線（consumer 側責務）
- `src`/`dst` の `ColorInfo` 一致検査（符号空間演算 — 解釈整合は
  consumer 責務として design が明示的に非検証と決定）
- `map.dst` の非空・キャンバス交差のエラー化（design が定義済み
  transparent black 結果と決定 — 拒否契約ではない）

## Risk

- `oc` の分子が `255·oa` を超える実装（`b` の再利用漏れ・独立
  計算・丸め規則の相違）→ `oc > 255` の構造的保証が破れ u8 出力が
  破損する — trap vector が境界を固定
- `w == 0` early return が検証より先に置かれる → 共有 `dst` や
  不正フォーマットが `opacity = 0` で暗黙通過する — 順序を固定
- `oa == 0` の除算ガード欠落 → 0 除算/NaN 生成 — 両フォーマットの
  `(0,0,0,0)` ガードを固定
- `AffineTransform` の写像演算を `i32`/`usize` 等の narrow 型で評価
  → 大寸法・負の矩形座標でオーバーフローし panic または不正
  サンプリング — `i128`/`u128` + `i64` 拡幅の算術幅を固定
- 拡大時の最終列が `map.src.w` に達する丸め実装 → 範囲外読み —
  セル中心式の正確な整数評価で構造的に防止
- `map.dst` のキャンバス非交差をエラー化する実装 → design が定義済み
  結果とする有界ケースを誤拒否 — 非拒否契約を固定
- `tq`/`w` の量子化を画素単位・パス別に行う → `Rgba8`/`RgbaF16` が
  異なる重みで発散 — 呼出し毎 1 回のスカラー量子化を固定
- `rt_process_*` テスト欠落・非 RT テストへの `rt_` 混入 →
  `mise run test-rt` の選択性と RT 契約の検証基盤が崩れる
