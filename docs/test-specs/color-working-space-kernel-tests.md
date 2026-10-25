---
id: "TSPEC-034LFChh51ZlHnSWXsKqDw"
type: "test-spec"
status: "approved"
title: "video-proc 色変換・working space カーネル テスト仕様"
created: "2026-10-25"
updated: "2026-10-25"
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
    - "docs/designs/color-conversion-kernels.md"
  derived-by: []
  verifies:
    - "docs/specs/color-conversion-kernels.md"
    - "docs/designs/color-conversion-kernels.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# video-proc 色変換・working space カーネル テスト仕様

> status: approved — Issue #99 の実装（Task 1–6・full gate 全 green）で契約が実結果により確認済み。run 可能コマンドは task 側に定義する（本書は契約と oracle の正本）。

## Purpose

spec `docs/specs/color-conversion-kernels.md`（proposed）・approved design `docs/designs/color-conversion-kernels.md` に基づき、Issue #99 の色管理カーネル群の受入契約を固定する — `PixelFormat::RgbaF32`（video-core 拡張）・`ToWorking`/`FromWorking`（f32 linear working space への入口/出口）・`ToneMapSdr`（BT.2390 EETF による HDR→SDR 写像）・既存 `CompositeOver`/`Crossfade`/`AffineTransform` への `RgbaF32` arm 追加。design が確定した絶対輝度スケール（1.0 = 100 cd/m²）・transfer decode/encode 式・`Unspecified` 既定規則・`ColorInfo` 更新規則・ulp-bound 等価性契約が将来の変更・最適化 backend 導入で退行しないことを保証する。feature `icc` 配下の `IccTransform`/`IccIntent` は `icc-transform-contract-tests.md` が担当する。本カーネル群が置換・再設計された場合に廃止してよい。

## Feature

`VideoFrame` 上のスカラー参照カーネル 4 系: `ToWorking`（`new()` unit ctor + `process(src, dst)` — src ∈ {`Rgba8`, `RgbaF16`, `RgbaF32`} → dst `RgbaF32` の decode+gamut 融合パス）、`FromWorking`（`new(dst_primaries, dst_transfer) -> Result<Self>` + `process(src, dst)` — src `RgbaF32` → dst ∈ {`Rgba8`, `RgbaF32`}）、`ToneMapSdr`（`new()` + `process(src, dst)` — `RgbaF32`→`RgbaF32` の輝度写像 + `HdrMetadata` 消費）、および `CompositeOver`/`Crossfade`/`AffineTransform` の `RgbaF32` 受理 arm。working space = `RgbaF32` + `{primaries: Bt2020, transfer: Linear, matrix: Identity, range: Full}` の絶対輝度 linear-light 表現。

## Rules

- **`PixelFormat::RgbaF32` レイアウト**: packed RGBA・4× IEEE-754 binary32 little-endian・1 plane・16 bytes/pixel — `plane_count() == 1`・`is_planar() == false`・`bytes_per_sample(0) == 16`（範囲外 plane は 0）・`chroma_subsampling() == None`。`#[non_exhaustive]` により non-breaking であり、既存カーネル（`YuvToRgb`/`RgbToYuv`/`Rgba8ToRgbaF16`/`RgbaF16ToRgba8`）の `_` arm は `RgbaF32` を graceful に拒否し続ける（それらの挙動は不変）
- **working-space 意味**: `RgbaF32` + `{Bt2020, Linear, Identity, Full}` のフレームは linear 光量値を保持し `1.0` = 100 cd/m²。値域は `[0,1]` に制限しない — 1.0 超の scene-referred 値と広色域変換由来の負値を lossless に搬送する。alpha は straight・規格化済みで全カーネルが無変換パススルー
- **transfer decode（信号 → 絶対輝度 `/100` nits 単位）**:
  - `Linear` — identity（値は既に絶対輝度単位）
  - `Bt709`/`Smpte170m` — H.273 OETF 逆関数（`v < 0.081` で linear toe `v/4.5`・`v ≥ 0.081` で `((v+0.099)/1.099)^(1/0.45)`）。出力は display-relative `[0,1]` を絶対輝度 1.0 とみなす（SDR 白 = 100 nits 前提）。toe 分岐は負値を自然に写像するため負入力も定義済み
  - `Pq` — ST 2084 EOTF（`m1=2610/16384`・`m2=78.84375`・`c1=0.8359375`・`c2=18.8515625`・`c3=18.6875`）で nits を得て `/100` 正規化。信号域 `[0,1]` 外の入力は decode 境界で clamp（`v < 0` → 0、`v > 1` → 100.0）
  - `Hlg` — BT.2100: `S = v²/3`（`v ≤ 0.5`）・`S = (exp((v−c)/a)+b)/12`（`v > 0.5`、`a=0.17883277`・`b=0.28466892`・`c=0.55991073`）で scene-relative を得て `S^1.2`（reference γ=1.2）× nominal peak 1000 nits → `×10` で絶対輝度。信号域外は clamp。`display_rel` 1.0 ↔ 10.0（=1000 nits）— PQ の 1000 nits と同一スケール
  - encode は各々の厳密な逆写像。`Pq`/`Hlg` encode 出力は信号域 `[0,1]` に clamp。`Bt709`/`Linear` encode は非 clamp（負・1 超の信号を保持）
- **primaries 変換**: 各 `ColorPrimaries` variant の RGB→XYZ 正規化行列（D65 — chromatic adaptation 不要）を定数テーブルとし、`src→BT.2020`/`BT.2020→dst` の結合 3×3 を f64 導出定数として保持。変換は RGB のみ・alpha 不変
- **`ToWorking`**: src ∈ {`Rgba8`（`v/255` 正規化）, `RgbaF16`（`f16_bits_to_f64` 経由）, `RgbaF32`（raw 読取）}。planar `Yuv*p`・未知 variant は `InvalidValue("src format")`。dst は `RgbaF32`・同一寸法・排他所有。`src.color()` を呼出し毎に読み、`primaries`/`transfer` は `Unspecified` → `Bt709` 既定規則で解決 — 解決不能な未知 variant は `InvalidValue("color primaries")`/`InvalidValue("color transfer")`。`matrix`/`range` は RGB 入力に非適用で検証しない。`RgbaF32`→`RgbaF32` の identity 入力でも画素コピーを通す（no-op early return なし — `ColorInfo` 記録が責務）。f32 入力の NaN/±Inf は f32 セマンティクスどおり伝播
- **`FromWorking`**: ctor は `Unspecified` → `Bt709` 既定解決・未知 variant を `InvalidValue` で拒否（`RgbToYuv` 先例の `Result<Self>` — 非 RT by convention・実 alloc なし）。src は `RgbaF32`・dst ∈ {`Rgba8`, `RgbaF32`}・同一寸法・排他所有。`Rgba8` dst は encode 値を `round(clamp(v,0,1)·255)`（RHU）で量子化 — 負・1 超は量子化段で飽和（`ToneMapSdr` 未通過の HDR→SDR 直結は consumer の誤用として doc 明記）。`RgbaF32` dst は encode 規則どおりの値を書く。ctor `Bt2020`/`Linear` の identity 経路は `RgbaF32` の純粋量化（`Rgba8` 化）を表現する
- **`ToneMapSdr`**: src/dst とも `RgbaF32`・同一寸法・排他所有。`src.color()` の working-space 記述は検証しない（メタデータ整合は consumer 責務 — composite 規約と同型・doc 明記）。BT.2020 輝度 `Y = 0.2627·R + 0.6780·G + 0.0593·B`（絶対値 ×100 nits）に BT.2390-8 §5.4 EETF: `e = pq_eotf_inv(Y/10000)`、`e_src = pq_eotf_inv(L_HS/10000)`、`E = e/e_src`、`maxLum = pq_eotf_inv(L_DS/10000)/e_src`（`L_DS = 100`）、`ks = 1.5·maxLum − 0.5`、`E ≤ ks` で `E' = E`・`E > ks` で `t = (E−ks)/(1−ks)` の Hermite `E' = (2t³−3t²+1)·ks + (t³−2t²+t)·(1−ks) + (−2t³+3t²)·maxLum`、黒浮き補正 `E'' = b·(1−E')⁴ + E'`（`b = pq_eotf_inv(L_min/10000)/e_src`）、`Y_out = pq_eotf(E''·e_src)·10000` → `/100`、`RGB' = RGB·(Y_out/Y_src)`。`Y_src ≤ 0` は passthrough・alpha 不変
- **`L_HS`/`L_min` 導出**: `mastering_display.max_luminance`（u32・1/10 000 cd/m² 単位 → `/10_000`）が `> 0` ならそれ、`content_light_level.max_cll`（u16・cd/m² 直値）`> 0` ならそれ、両者不在/0 なら既定 `1000` nits。`L_min` = `mastering_display.min_luminance`（同スケール・`max_luminance` 未満のみ採用 — 不正順序は `b = 0` で無視）。`L_HS ≤ L_DS`（SDR 等値入力）なら `ks ≥ 1` で全域 passthrough — 定義上の正しい非圧縮
- **`ColorInfo` 更新規則**（doc 明記・実装一致を検証）: `ToWorking` — `{Bt2020, Linear, Identity, Full, hdr: src.hdr}`。`FromWorking` — `{ctor 解決値, ctor 解決値, Identity, Full, hdr: src.hdr}`。`ToneMapSdr` — `src.color` だが `hdr` は `None`（消費済み）。全て `pts`/`duration` を `src` から伝播
- **既存カーネルの `RgbaF32` arm**: `CompositeOver`/`Crossfade`/`AffineTransform` の受理集合を {`Rgba8`, `RgbaF16`, `RgbaF32`} へ拡張（src/dst 一致規則・straight alpha・呼出し毎 Q8 重み量子化・範囲外 transparent black・`map.src` 検証/非交差 black-fill・メタデータ規約は不変）。`RgbaF32` は `RgbaF16` パスと同じ f32 評価を binary16 丸めなしで行う。非有限チャネルは f32 セマンティクスどおり伝播
- **検証/エラー規約（全カーネル共通）**: 全検証は書込み前 — 共有 `dst`（`plane_mut` → `None`）は `InvalidValue("dst plane")`、format/dims/color 解釈不能は対応する `InvalidValue` — 拒否呼出しは `dst` を一切変更しない（panic-free・部分書込みなし）。`stride > row_bytes` のパディングは読み書きしない
- **等価性契約**: 参照実装 = f64 で評価した定義式の binary32 丸め。最適化 backend はチャネル毎 ≤ 4 ulp（binary32 — `ToWorking`/`FromWorking`/`ToneMapSdr`）・composite 拡張は参照 f32 評価と ≤ 2 ulp。`Rgba8` decode 入力は厳密・`Rgba8` 出力は量子化境界の ±1 code を許容。`Y_src ≤ 0` の passthrough 経路は厳密一致。契約は有限値の出力誤差のみを規定（プラットフォーム `powf` の libm 差を吸収するため golden は厳密一致ではなく bound 比較）
- **RT 契約**: 全 `process` は `**RT contract:**` ラベル付きで alloc-free・非ブロッキング・有界（ctor は非 RT — `IccTransform` 以外 alloc しないが「非 RT by convention」表記を維持）。`rt_process_*` 命名テスト（repeatable/deterministic・ソース非破壊 — alloc の機械検出は別 Issue・#97 と同一レーン）を各カーネルに持ち `mise run test-rt` で選択可能。非 RT テスト名に `rt_` を含めない

## Examples

- **`RgbaF32` layout**: `PixelFormat::RgbaF32.plane_count() == 1`・`bytes_per_sample(0) == 16`・`bytes_per_sample(1) == 0`・`is_planar() == false`・`chroma_subsampling() == None` — 既存 5 format の行を維持したまま表へ追加
- **`ToWorking` decode golden**: `Rgba8` `v = 255`・`Bt709` タグ → linear 1.0（= 100 nits）。`v = 128`・`Pq` タグ → ST 2084 の f64 参照値 `/100`（≤ 4 ulp）。`HLG` `v = 1.0` → 10.0（1000 nits 相当）。`Unspecified` primaries/transfer → `Bt709` 既定で解決
- **`ToWorking` gamut**: `P3D65` タグの単位 R 画素 → BT.2020 座標（f64 参照との ≤ 4 ulp・負成分が出る広色域側ケースも値が clamp されず保持される）
- **`ToWorking` メタデータ**: dst の `color` が `{Bt2020, Linear, Identity, Full}`・`hdr` が `src.hdr` を引継ぎ・`pts`/`duration` 伝播
- **`ToWorking` 拒否**: `Yuv420p` src → `InvalidValue("src format")`。未知 transfer variant（モンキーパッチ不能のため `_` arm の構造的到達は review 担保・到達可能な規約差のみ機械検証）→ `dst` 不変
- **`FromWorking` encode golden**: working-space 1.0（= 100 nits）・ctor `Bt709`/`Bt709` → `Rgba8` 255（`round(clamp(1.0)·255)`）。`Pq` ctor → ST 2084 逆写像の `[0,1]` 信号 → `Rgba8` ±1 code。ctor `Bt2020`/`Linear` + `Rgba8` dst は純粋量子化（`round(clamp·255)`）— HDR 残留 2.0 は 255 へ飽和
- **`FromWorking` ctor 拒否**: 未知 `ColorPrimaries`/`TransferCharacteristics` variant は `InvalidValue`（`Result<Self>` — フレームを触らない）
- **`ToneMapSdr` golden**: `L_HS = 1000` 既定（`hdr` 不在）で `Y = 10.0`（1000 nits）画素 → EETF 後の f64 参照値（≤ 4 ulp・`Y_out < 10.0` に圧縮）。`mastering_display.max_luminance = 4_000_000`（= 400 nits）のみ → `L_HS = 400`。`mastering_display` 不在 + `content_light_level.max_cll = 2000` → `L_HS = 2000`。`L_HS = 100`（SDR 等値）→ 全域 passthrough（`ks ≥ 1`）。`Y_src ≤ 0`（負・ゼロ輝度）→ 画素 passthrough 厳密一致
- **`ToneMapSdr` hdr 消費**: `hdr` 付き入力の出力 `dst.color().hdr` が `None`・`primaries`/`transfer` は `src` を引継ぎ・`pts`/`duration` 伝播
- **`RgbaF32` arm（composite）**: `CompositeOver` `opacity = 1.0` で不透明 src が `dst` を `RgbaF32` 上で完全上書き・`opacity = 0` early return で `dst` byte-identical（共有 `dst` 拒否は `opacity = 0` でも先行）。`Crossfade` `t = 0`/`t = 1` の端点再現（`RgbaF32` は有限チャネルを数値再現 — `−0.0`/`NaN` ペイロード正規化は f16 と同規則）。`AffineTransform` 恒等写像で `RgbaF32` bit-exact 複製・`map.dst` キャンバス外は transparent black
- **共通拒否**: `Rgba8`/`RgbaF32` 混在・寸法不一致・共有 `dst` backing（clone 保持）→ `InvalidValue` で `dst` 部分書込みなし。`stride > row_bytes` の `dst` パディング byte は呼出し前の内容を保持
- **`rt_process_*`**: 同一カーネルの反復呼出しは決定的で panic せず、`src` は byte-identical — `dst` への書込みのみが副作用

## Guarantees

- 全 `process` は検証を書込み前に完了し、拒否呼出しは `dst` を一切変更しない（panic-free・部分書込みなし — ADR D7）
- `ToWorking`/`FromWorking`/`ToneMapSdr` は f64 参照式に対しチャネル毎 ≤ 4 ulp（binary32）の ulp-bound 等価性契約を持ち、`Rgba8` 量子化は境界 ±1 code まで許容 — プラットフォーム libm 差を吸収する bound 比較であり厳密一致は要求しない（composite `RgbaF32` 拡張は ≤ 2 ulp）
- `src` の immutable 参照フレームは `process` が変更しない — 副作用は `dst` への書込みのみ
- `process` は内部確保・ブロッキング・ロックを持たない — alloc-free は API 単位の `**RT contract:**` ラベルで保証され、レビュー担保として検証される（`assert_no_alloc` 機械検出は後続 Issue — #97 と同一レーン）
- `ColorInfo` 更新規則（working-space 記述・ctor 解決値・`hdr` 消費）は doc と実装が一致する — spec AC5

## Non-goals

- feature `icc` 配下の `IccTransform`/`IccIntent`（`icc-transform-contract-tests.md` が担当）
- planar YUV への直接 gamut/transfer 適用（`Yuv*p` は既存 `YuvToRgb`/`RgbToYuv` 経由の二段処理 — design で変更しない確定事項）
- OCIO・ACES・GPU 経路・`RgbaF16` dst の `FromWorking`・`RgbaF32 ↔ RgbaF16` shuffle・10/12-bit integer パイプライン・tone-map 対象ピークのパラメータ化（`L_DS` 固定 100 nits）・`L_W` 設定化・stage 分割カーネル
- `src`/`dst` の `ColorInfo` 一致検査・working-space 記述の強制（解釈整合は consumer 責務 — design が明示的に非検証と決定）
- `assert_no_alloc` による alloc 機械検出（design で後続 Issue へ defer 済み）
- `ruviform-edit`・engine・OFX への配線（consumer 側責務）

## Risk

- transfer decode/encode の式・clamp 境界の誤実装（toe 分岐点 `0.081`・PQ 定数・HLG nominal peak 誤り）→ golden vector が定義式の f64 参照を固定
- `Unspecified` 既定（`Bt709`）の未適用・厳密拒否化 → 未タグ素材の一般的ケースを誤拒否 — 既定規則の挙動を固定
- `Y_src ≤ 0`・`oa` 系ガード欠落 → 0 除算/NaN 生成 — passthrough/guard 経路の厳密一致を固定
- `ToneMapSdr` の `L_HS` 優先順位（mastering → MaxCLL → 既定 1000）・`L_min` 不正順序の `b = 0` 無視が退行 → `HdrMetadata` 組合せケースで固定
- `hdr` 消費規則（`ToneMapSdr` 出力で `None`）の欠落 → 二重トーンマップを招く — メタデータ規約で固定
- `Rgba8` encode の非 clamp 誤実装 → HDR 残留輝度が wrap/飛和せず破損 — 量子化 clamp の飽和挙動を固定
- `rt_process_*` 欠落・非 RT テストへの `rt_` 混入 → `mise run test-rt` の選択性と RT 契約の検証基盤が崩れる
- `RgbaF32` の `bytes_per_sample` 誤値（8/4 等）→ frame 確保の stride/row_bytes が破損 — plane 算定の表駆動テストで固定
