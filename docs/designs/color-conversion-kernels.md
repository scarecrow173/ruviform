---
id: "DESIGN-034LFikCuBIoaD4mWBEjIU"
type: "design"
status: "approved"
title: "ruviform-video-proc 色変換カーネル設計（f32 linear working space + ICC 入出力経路）"
created: "2026-10-25"
updated: "2026-10-25"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/99"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/color-conversion-kernels.md"
  implemented-by: []
  depends-on:
    - "docs/specs/color-conversion-kernels.md"
    - "docs/discovery/color-conversion-kernels.md"
    - "docs/designs/second-target-design.md"
    - "docs/adr/broad-color-management.md"
    - "docs/adr/video-proc-kernel-api-and-color-hdr-scope.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/composite-transition-kernels.md"
    - "docs/designs/composite-transition-kernels.md"
    - "docs/specs/edit-data-model.md"
    - "docs/designs/edit-data-model.md"
    - "docs/designs/ofx-host-adapter.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/color-conversion-kernels.md"
    - "docs/discovery/color-conversion-kernels.md"
    - "docs/designs/second-target-design.md"
    - "docs/adr/broad-color-management.md"
    - "docs/adr/video-proc-kernel-api-and-color-hdr-scope.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "docs/plans/implement-issue-99-color-conversion-kernels.md"
    - "docs/test-specs/color-working-space-kernel-tests.md"
    - "docs/test-specs/icc-transform-contract-tests.md"
    - "docs/specs/video-proc-rt-verification-lane.md"
    - "docs/plans/implement-issue-126-video-proc-rt-lane.md"
  verifies: []
  verified-by:
    - "docs/test-specs/color-working-space-kernel-tests.md"
    - "docs/test-specs/icc-transform-contract-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# ruviform-video-proc 色変換カーネル設計（f32 linear working space + ICC 入出力経路）

## Context

Issue #99 — 第 2 ターゲット設計の後続 Issue 分解・第 4 項目。
ADR `broad-color-management` の D1-b（scene-referred 内部表現 =
f32 linear + 広色域 primaries・`ColorInfo` で明示）・D2-a（入出力
ICC プロファイル経路・`moxcms` 有力候補、採否は本設計に委任）・
D3-b（ACES 後続）と、video-proc カーネル ADR の D1-b（非 RT
`prepare`/コンストラクタ → alloc-free `process` の二相形状）・
D7（`ruviform_media_core::Error` 再利用）を、初期カーネル集合・
画素演算規約・working-space 定義・ICC 分離形・等価性契約へ具体化
する。consumer は `ruviform-edit`（#96）評価射影が駆動する
engine/射影層 — 複数ソースの linear-light 混合と、ICC/CICP で
記述された素材の一貫した入出力変換を必要とする。

本設計は discovery（`docs/discovery/color-conversion-kernels.md`）
の Open Questions 全項目を確定する。

## 決定サマリ

| 論点 | 決定 |
| --- | --- |
| f32 ピクセル表現 (a) | `PixelFormat` に **`RgbaF32`** を追加 — packed RGBA・4× IEEE-754 binary32 little-endian・1 plane・16 bytes/pixel。video-core 拡張（`#[non_exhaustive]` で non-breaking）。内部 scratch 案は working space がフレームをまたがないため棄却 |
| ICC エンジン + 分離形 (b) | **`moxcms` 0.9 採用**・`ruviform-video-proc` の **Cargo feature `icc`**（`optional` 依存・`default = []`）で opt-in 分離。`default-features = false` + `features = ["lut", "extended_range"]`。moxcms 型・エラー・feature 名は公開 API に露出しない（rtrb と同じ private-implementation 閉じ込め） |
| working-space primaries + `ColorInfo` 写像 (c) | working space = **BT.2020 primaries・`Linear` transfer・`matrix: Identity`・`range: Full`** の `RgbaF32`。色域は絶対輝度正規化（`1.0` = 100 cd/m² SDR 基準白）。`Unspecified` の解釈: primaries → `Bt709`・transfer → `Bt709`（既存 `matrix → Bt601`・`range → Limited` 既定表の上位拡張）。RGB フォーマット上の `matrix` は非適用（出力で `Identity` を記録）、`range` は code value そのまま |
| HDR→SDR トーンマップ (d) | **ITU-R BT.2390 EETF**（PQ 信号域の Hermite スプライン）を輝度に適用し RGB 比保存でスケール。ソースピーク輝度 `L_HS` は `hdr.mastering_display.max_luminance` → `content_light_level.max_cll` → 既定 1000 nits の優先順。ターゲットは SDR 100 nits 固定 |
| 初期カーネル集合 (e) | `ToWorking`（src `{Rgba8,RgbaF16,RgbaF32}` → `RgbaF32`・decode+gamut 融合）・`FromWorking`（`RgbaF32` → `{Rgba8,RgbaF32}`・ctor で dst primaries/transfer 焼付け）・`ToneMapSdr`（`RgbaF32`→`RgbaF32`）・`IccTransform`（feature `icc`・`{Rgba8,RgbaF32}` 任意組合せ）+ `IccIntent` enum。既存 3 カーネル（`CompositeOver`/`Crossfade`/`AffineTransform`）に `RgbaF32` 対応を追加（#97 deferred の linear-light 合成を閉じる） |
| RT/非 RT 分離 (f) | ctor/prepare = 非 RT（行列解決・moxcms profile parse + executor 構築 — 確保可）。`process` = alloc-free・非ブロッキング・有界（`width`×`height` 画素走査・スタック上 `[f32;4]` のみ）。既存と同一の `**RT contract:**` ラベル + `rt_process_*` 検証規約 |
| エラー (g) | `ruviform_media_core::Error::InvalidValue(&'static str)` — 全検証は書込み前・panic-free・部分書込みなし。ctor 失敗は `Result<Self>`（`RgbToYuv` 先例）。未知 enum variant は `_` arm で graceful 拒否 |

## Scope

### In Scope

- `PixelFormat::RgbaF32`（video-core 拡張 — plane 算定の波及確認含む）
- `ToWorking` カーネル — transfer decode（EOTF 系）+ src primaries →
  BT.2020 の 3×3 行列を融合した 1 パス変換（`Unspecified` 既定規則適用）
- `FromWorking` カーネル — BT.2020 → dst primaries 行列 + transfer
  encode を ctor で焼付けた 1 パス変換（dst は `Rgba8`/`RgbaF32`）
- `ToneMapSdr` カーネル — BT.2390 EETF による HDR→SDR 輝度写像
  （`HdrMetadata` 参照規則込み）
- `IccTransform` カーネル + `IccIntent`（feature `icc`・moxcms
  `create_transform_f32` 経路 — `Rgba8`/`RgbaF32` 任意組合せ）
- 既存 `CompositeOver`/`Crossfade`/`AffineTransform` への `RgbaF32`
  フォーマット追加（straight alpha・符号化ドメイン非依存の写像/合成
  規約は不変 — linear working space 上の合成を可能にする）
- 各カーネルの `**RT contract:**`・等価性契約・メタデータ規約・
  エラー規約の doc 明記・golden vector / `rt_process_*` テスト・
  crate doc（`lib.rs`）カーネル一覧・feature 記述の更新
- `Cargo.toml` — `moxcms` optional 依存 + `[features]` ブロック追加

### Out of Scope

- OCIO 設定消費・ACES IDT/ODT/RRT フル管（ADR D2-b 保留・D3-b 後続 —
  変換ポート追加で吸収可能な構造は維持）
- GPU 計算カーネル・バックエンド実装（ruviform-gpu Issue）
- planar YUV への直接適用 — `Yuv*p` は既存 `YuvToRgb`/`RgbToYuv` 経由
  の二段処理（本設計でも変更しない）
- ICC プロファイルの供給経路（素材メタデータからの ICC 取得 —
  media-io/アプリ層責務）・`ColorInfo` → CICP profile 生成の
  便利コンストラクタ・CMYK/多チャネル layout・`in_place`/`options`/
  `any_to_any`/SIMD 系 moxcms feature
- intent 拡張集合（Saturation/AbsoluteColorimetric）・黒点補償
- 既存 code-value カーネル（`YuvToRgb`/`RgbToYuv`/`Rgba*F16`）の挙動
  変更・`Processor` trait 実装・`RtContext` 引数・フレームプール・
  `ruviform-edit`/engine/OFX への配線
- `RgbaF16` dst（`FromWorking`）・`RgbaF32 ↔ RgbaF16` shuffle・
  10/12-bit integer パイプライン・トーンマップ対象ピークの
  パラメータ化（固定 100 nits SDR）
- video-proc への `assert_no_alloc` 導入（#97 設計と同じく
  RT 検証レーン拡大の別 Issue 管轄）

## Components and Boundaries

| 型 | 形状 | 責務 |
| --- | --- | --- |
| `PixelFormat::RgbaF32` | video-core enum 拡張 — `plane_count() == 1`・`is_planar() == false`・`bytes_per_sample(0) == 16`・`chroma_subsampling() == None` | working space の保持形。1 画素 = RGBA 4× binary32 LE・16 bytes。`#[non_exhaustive]` により non-breaking |
| `ToWorking` | `new()`（unit ctor + `Default`）+ `process(&mut self, src: &VideoFrame, dst: &mut VideoFrame) -> Result<()>` | src `ColorInfo` の transfer を decode（EOTF 系・絶対輝度正規化へ写像）し primaries を BT.2020 へ行列変換する融合パス — working space への入口カーネル。`src.color()` を呼出し毎に読む（`YuvToRgb` と同一規約） |
| `FromWorking` | `new(dst_primaries: ColorPrimaries, dst_transfer: TransferCharacteristics) -> Result<Self>`（`RgbToYuv` 先例の ctor 検証）+ `process(src, dst)` | working space からの出口カーネル — dst primaries/transfer を ctor で解決・焼付け（`Unspecified` → `Bt709` 既定・未知 variant 拒否）。`Rgba8` dst は encode + [0,1] clamp + RHU 量子化、`RgbaF32` dst は encode のみ（無 clamp 書出し規則は後述） |
| `ToneMapSdr` | `new()`（unit ctor + `Default`）+ `process(src, dst)` — 両者 `RgbaF32` | BT.2390 EETF で HDR 絶対輝度域を SDR 100 nits 域へ写像。`src.color().hdr` を呼出し毎に読み `L_HS`/`L_min` を導出 — 消費した `hdr` は出力で `None` にする |
| `IccTransform` | `new(src_profile: &[u8], dst_profile: &[u8], intent: IccIntent) -> Result<Self>` + `process(src, dst)` — src/dst ∈ `{Rgba8, RgbaF32}` 任意組合せ | ICC profile 対の変換経路。非 RT ctor が moxcms `ColorProfile::new_from_slice` で両 profile を parse し `create_transform_f32(Layout::Rgba, dst, Layout::Rgba, options)` で executor を焼付け。feature `icc` のみで公開 |
| `IccIntent` | `#[non_exhaustive]` enum `{Perceptual, RelativeColorimetric}`（feature `icc` 配下） | 初期 intent 集合。内部で moxcms `RenderingIntent` へ写像 — moxcms 型を公開しない |

### `RgbaF32` と working-space 定義

- 画素レイアウト: `RgbaF16` と同形で 4× binary16 の代わりに
  4× IEEE-754 binary32（little-endian）。`stride > row_bytes` の
  パディング規則・`row`/`row_mut` 経由アクセスは不変
- working-space 意味（doc 明記・全カーネル共通の解釈）:
  `transfer: Linear` + `primaries: Bt2020` + `matrix: Identity` +
  `range: Full` の `RgbaF32` フレームは **linear 光量値**を保持し、
  `1.0` = 100 cd/m²（SDR 基準白）の**絶対輝度スケール**とする。
  値域は `[0,1]` に制限しない — scene-referred の 1.0 超・広色域
  変換由来の負値を保持する（lossless 搬送）。alpha は straight・
  規格化済み（`[0,1]` 基準）で無変換パススルー
- この規約により `transfer` の decode は「信号 → 絶対輝度
  （`/100 nits` 単位）」、encode はその逆写像として一意に定義される:
  - `Linear` — identity（値は既に絶対輝度単位）
  - `Bt709`/`Smpte170m` — H.273 OETF 逆関数（linear toe `v/4.5`
    for `v < 0.081`・`((v+0.099)/1.099)^(1/0.45)` for `v ≥ 0.081`）。
    出力は display-relative `[0,1]` — 絶対輝度 1.0 とみなす
    （SDR 白 = 100 nits 前提）。toe 分岐は負値を自然に写像するため
    負入力も定義済み
  - `Pq` — ST 2084 EOTF: `N = V^(1/m2)`、`L = 10000·((max(N−c1,0))/(c2−c3·N))^(1/m1)` nits
    （`m1=2610/16384`・`m2=78.84375`・`c1=0.8359375`・`c2=18.8515625`・
    `c3=18.6875`）→ `/100` で正規化。信号域 `[0,1]` 外の入力は
    decode 境界で clamp（`v < 0` → 0、`v > 1` → 100.0）
  - `Hlg` — BT.2100: `S = v²/3`（`v ≤ 0.5`）・`S = (exp((v−c)/a)+b)/12`
    （`v > 0.5`）で scene-relative を得て、`display_rel = S^1.2`
    （reference system γ = 1.2）× nominal peak 1000 nits → `×10` で
    絶対輝度へ（`a=0.17883277`・`b=0.28466892`・`c=0.55991073`）。
    信号域外は clamp。`display_rel` 1.0 ↔ 10.0（=1000 nits）—
    PQ の 1000 nits と同一スケール
  - encode は各々の厳密な逆写像。`Pq`/`Hlg` の encode 出力は信号域
    `[0,1]` に clamp（定義域外の絶対輝度は飽和 — doc 明記）。
    `Bt709`/`Linear` の encode は非 clamp（負・1 超の信号を保持）
- primaries 変換: 各 `ColorPrimaries` variant の RGB→XYZ 正規化行列
  （D65 — 全 variant が D65 白のため chromatic adaptation 不要）を
  定数テーブルとし、`src→BT.2020` / `BT.2020→dst` の結合 3×3 を
  f64 で導出した定数として保持（実行時の逆行列計算なし）。
  変換は RGB 3 チャネルのみ — alpha 不変
- `ColorInfo` 更新規則:
  - `ToWorking` — `dst.color = {primaries: Bt2020, transfer: Linear,
    matrix: Identity, range: Full, hdr: src.hdr}`。`pts`/`duration`
    は `src` から伝播
  - `FromWorking` — `dst.color = {primaries: ctor 解決値,
    transfer: ctor 解決値, matrix: Identity, range: Full,
    hdr: src.hdr}`。`pts`/`duration` 伝播
  - `ToneMapSdr` — `dst.color = src.color` だが `hdr` は `None`
    （消費済み — 内容は既に SDR レンジ）。`pts`/`duration` 伝播
  - `IccTransform` — `dst.color = {primaries: Unspecified,
    transfer: Unspecified, matrix: Identity, range: Full,
    hdr: None}` — ICC profile が記述する色空間は CICP enum へ
    写像できないため unspecified が正直な記録（consumer が既知
    なら `set_color` で上書き）。`pts`/`duration` 伝播

### `ToWorking` 規約

- src ∈ `{Rgba8, RgbaF16, RgbaF32}` — `Rgba8` は `v/255`、`RgbaF16`
  は `f16_bits_to_f64` 経由、`RgbaF32` は raw 読取で正規化信号を得る。
  planar `Yuv*p`・未知 `#[non_exhaustive]` variant は
  `InvalidValue("src format")` — RGB への下準備は `YuvToRgb` 等の
  既存経路
- dst は `RgbaF32`・同一寸法・排他所有（共有 backing 拒否）。
  画素毎: 4 チャネル decode →（RGB のみ）3×3 行列 → alpha パススルー
- `src.color()` の解釈（呼出し毎）: `primaries`/`transfer` は本設計の
  既定規則（`Unspecified` → `Bt709`）で解決し、解決不能な未知
  variant は `InvalidValue("color primaries")`/`"color transfer"`。
  `matrix` は RGB 入力に非適用（検証しない）。`range` は code value
  をそのまま扱う（RGB は全 range として解釈 — 検証しない）
- 等価性契約: 参照実装 = f64 で評価した上記式の binary32 丸め。
  最適化 backend はチャネル毎 **≤ 4 ulp（binary32）**。
  `Rgba8`/`RgbaF16` decode 入力は厳密。非有限入力・clamp 境界の
  挙動は定義済みどおり — 契約は有限値の出力誤差のみを規定
  （プラットフォーム `powf` の libm 差を吸収するため golden は
  厳密一致ではなく bound 比較）

### `FromWorking` 規約

- `new(dst_primaries, dst_transfer)` — 既定規則解決（`Unspecified` →
  `Bt709`）・未知 variant は `InvalidValue`。結合行列 + encode
  パラメータを焼付け（非 RT だが実際には alloc なし — `RgbToYuv`
  と同じ「非 RT by convention」表記）
- src は `RgbaF32`・dst ∈ `{Rgba8, RgbaF32}`・同一寸法・排他所有。
  画素毎: 3×3 行列 →（RGB のみ）encode → alpha パススルー。
  `Rgba8` dst は encode 値を `round(clamp(v,0,1)·255)`（RHU）で
  量子化。`RgbaF32` dst は encode 規則どおりの値を書く
  （`Pq`/`Hlg` は信号域 clamp 済み・`Bt709`/`Linear` は非 clamp）
- working → working の identity 経路（ctor が `Bt2020`/`Linear`）
  は行列 = 恒等・encode = passthrough — `RgbaF32` の純粋量化
  （`Rgba8` 化）は本カーネルで表現できる（shuffle 専用カーネル
  は持たない）
- 等価性契約: `ToWorking` と同規則（f64 参照 → binary32 ≤ 4 ulp、
  `Rgba8` 出力は量子化境界の ±1 code を許容）

### `ToneMapSdr` 規約

- src/dst とも `RgbaF32`・同一寸法・排他所有。`src.color()` の
  `primaries`/`transfer` が working space（`Bt2020`/`Linear`）を
  示すことは検証しない — メタデータ整合は consumer 責務（composite
  規約と同型）。ただし `primaries`/`transfer` が working-space
  記述でない入力は定義上の誤用として doc 明記
- 画素毎: BT.2020 輝度 `Y = 0.2627·R + 0.6780·G + 0.0593·B`
  （絶対値 ×100 nits）に対し BT.2390-8 §5.4 EETF を適用:
  1. `e = pq_eotf_inv(Y_nits/10000)` で PQ 信号域へ
  2. `e_src = pq_eotf_inv(L_HS/10000)` で正規化 — `E = e/e_src`
  3. `maxLum = pq_eotf_inv(L_DS/10000)/e_src`（`L_DS = 100` nits）
  4. `ks = 1.5·maxLum − 0.5` — `E ≤ ks` なら `E' = E`、
     `E > ks` なら `t = (E−ks)/(1−ks)` の Hermite:
     `E' = (2t³−3t²+1)·ks + (t³−2t²+t)·(1−ks) + (−2t³+3t²)·maxLum`
  5. 黒浮き補正 `E'' = b·(1−E')⁴ + E'`（`b = pq_eotf_inv(L_min/10000)/e_src`）
  6. `Y_out = pq_eotf(E''·e_src)·10000` nits → `/100` で working 値
  7. `RGB' = RGB·(Y_out/Y_src)`（`Y_src ≤ 0` は passthrough —
     負輝度は写像不能）。alpha 不変
- `L_HS` 導出: `mastering_display.max_luminance`（`u32`・ST 2086
  の 1/10 000 cd/m² 単位 → `/10_000` で nits へ換算）が `> 0`
  ならそれ、`content_light_level.max_cll`（`u16`・cd/m² 直値）
  が `> 0` ならそれ、両者不在/0 なら既定 `1000` nits。
  `L_min` = `mastering_display.min_luminance`（同スケール・
  `max_luminance` 未満の場合のみ採用 — 不正順序は `b = 0` で無視）
- `L_HS ≤ L_DS`（SDR 等値入力）なら `ks ≥ 1` となり全域
  passthrough — no-op でなく定義上の正しい非圧縮として扱う
- 等価性契約: f64 参照 → binary32 ≤ 4 ulp（`Y_src ≤ 0` の
  passthrough 経路は厳密一致）

### `IccTransform` 規約（feature `icc`）

- `new(src_profile, dst_profile, intent)` — `IccIntent` → moxcms
  `RenderingIntent` 写像。`ColorProfile::new_from_slice` の parse
  失敗は `InvalidValue("icc profile")`、`create_transform_f32`
  の失敗は `InvalidValue("icc transform")`（`CmsError` は
  `InvalidValue` の文字列へ写像 — moxcms 型を露出しない）。
  `TransformOptions`: `rendering_intent` = intent 写像値・
  `allow_use_cicp_transfer = true`・`prefer_fixed_point = false`
  （f32 executor の参照精度を固定小数に落とさない）・
  `allow_extended_range_rgb_xyz = true`（`extended_range` feature —
  matrix-shaper profile で [0,1] 外を保持）・interpolation/
  barycentric は既定（`Linear`/`Low`）
- `process` — src/dst ∈ `{Rgba8, RgbaF32}` 任意組合せ・同一寸法・
  排他所有。画素毎: 入力を `[f32;4]` スタック配列へ正規化
  （`Rgba8` は `v/255`・`RgbaF32` は raw LE 読取 — moxcms は
  `[0,1]` 正規化入力を要求）→ executor `transform(&[src_px],
  &mut [dst_px])`（`TransformExecutor::transform` は画素配列の
  スライス `&[[V]]`/`&mut [[V]]` を取る — 1 画素 = 1 要素スライス
  で呼ぶ）→ 出力エンコード（`RgbaF32` raw 書出し・`Rgba8` は
  `round(clamp(v,0,1)·255)`）。alpha は moxcms `Layout::Rgba` の
  alpha パススルー経路（実装時に alpha 非変換を検証 — doc に規約明記）
- moxcms executor の `transform` エラーは `InvalidValue("icc transform")`
- 等価性契約: **pin した moxcms バージョン + feature 集合の出力が
  参照** — executor は決定的（SIMD feature 無効のスカラー経路のみ・
  ホスト非依存）であり、本クレートの正規化/量子化 shim は厳密な
  ため、golden vector は executor 出力に対し bit-exact で検証可能。
  moxcms バージョン/feature 変更時は golden を再導出する
  （クロスエンジン等価性は契約外）

### 既存カーネルの `RgbaF32` 拡張

- `CompositeOver`/`Crossfade`/`AffineTransform` の受理フォーマットを
  `{Rgba8, RgbaF16, RgbaF32}` へ拡張（src/dst 一致規則は不変）—
  linear working space 上の合成・遷移・配置を可能にし、#97 設計の
  deferred「linear-light コンポジット」を閉じる
- 演算規約は不変: `RgbaF32` では `RgbaF16` パスと同じ f32 評価を
  binary16 丸めなしで行う — 等価性契約は「参照 f32 評価との
  ≤ 2 ulp（binary32）」。straight alpha・Q8 重み量子化・
  範囲外 transparent black・メタデータ規約は全て踏襲。
  非有限チャネル値は f32 セマンティクスどおり伝播（契約外）
- `YuvToRgb`/`RgbToYuv`/`Rgba8ToRgbaF16`/`RgbaF16ToRgba8` は変更
  しない — `_` arm で `RgbaF32` を graceful 拒否する現行挙動を維持

## Data and Control Flow

### パイプライン形状

- NLE linear-light 合成パターン: `YuvToRgb`（必要時）→ `ToWorking`
  → `CompositeOver`/`Crossfade`/`AffineTransform`（`RgbaF32` 上で
  連結 — 中間フレーム確保は呼出し側・非 RT 層の責務）→
  `ToneMapSdr`（HDR ソースのみ）→ `FromWorking` → 表示/出力
- ICC 経路: 素材由来 profile バイト列（media-io/アプリ層が供給 —
  本クレートの責務外）→ `IccTransform::new` で焼付け → `process`
  が任意端点フォーマット間を直接変換。working space との接続は
  `ToWorking`/`FromWorking` の CICP 経路と `IccTransform` の profile
  経路を呼出し側が選択する — CICP-described 変換を ICC 経由に
  統一しない（両経路は独立・等価性主張なし）
- 全カーネル共通: `process` は検証を全て書込み前に実施 — 失敗呼出しは
  `dst` を部分書込みしない。`dst` の確保は呼出し側、カーネルは
  確保しない（既存規約不変）

### エッジケース

- 全カーネル: `stride > row_bytes` パディングは読み書きしない
  （`row`/`row_mut` 経由のみ）、共有 `dst`（clone 保持中）は
  `InvalidValue("dst plane")`、`src` は非破壊、`pts`/`duration` は
  変換カーネル規約どおり `src` 由来で上書き
- `ToWorking`: `RgbaF32 → RgbaF32` の transfer=Linear・primaries=
  Bt2020 は数学的 identity だが画素コピーを通す（no-op early
  return は設けない — 出力フレームの ColorInfo 記録が責務）。
  f32 入力の NaN/±Inf は f32 セマンティクスどおり伝播
- `FromWorking`: `Rgba8` 出力で `Bt709`/`Linear` encode の負・1 超
  値は量子化段で clamp（HDR 残留輝度は飽和 — `ToneMapSdr` 未通過の
  HDR→SDR 直結は consumer の誤用として doc 明記）
- `ToneMapSdr`: `Y_src ≤ 0` 画素は passthrough。`hdr` 不在は既定
  `L_HS = 1000` nits で動作。`L_HS ≤ 100` は全域 passthrough
- `IccTransform`: `RgbaF32` dst は moxcms の extended-mode 出力
  （負・1 超を保持 — HDR/広色域 profile 変換の情報損失を防ぐ）、
  `Rgba8` dst は量子化 clamp

## Non-Functional Constraints

- 全カーネルはスカラー参照実装。等価性契約はカーネル doc に明記
  — f32 経路は f64 参照式の binary32 丸めを規範とする **ulp-bound**
  （`ToWorking`/`FromWorking`/`ToneMapSdr` ≤ 4 ulp、composite 拡張
  ≤ 2 ulp）、`Rgba8` 量子化は boundary ±1 code、`IccTransform` は
  pin 版 moxcms 出力に bit-exact
- `process` は `width`×`height` の線形画素走査で有界・alloc-free・
  非ブロッキング — `**RT contract:** RT-suitable` ラベルを API 単位
  で付す。`IccTransform::process` の alloc-free は moxcms executor が
  事前構築パイプライン上の純粋写像である設計に依拠 — 実装レビューで
  確認（`rt_process_*` は repeatable/deterministic・非破壊を検証、
  alloc の機械検出は別 Issue — #97 設計と同一規約）
- ctor の非 RT 性: `IccTransform::new` は profile parse + executor
  構築で alloc する（非 RT 前提の正しい分離）。他 ctor は alloc しない
  が「非 RT by convention」の表記規約を維持
- `rt_` 命名規約: 各新規カーネルに `rt_process_*`（repeatable/
  deterministic・never-mutates-source）テストを持ち `mise run
  test-rt` で選択可能。非 RT テスト名に `rt_` を含めない
- feature `icc` は `default = []` 配下の opt-in — ベース経路
  （feature 無効）の依存は `ruviform-video-core`・
  `ruviform-media-core` のみで外部依存ゼロを維持。moxcms 型・
  エラー・feature 名は公開 API に露出しない（`icc` 有効時も同じ —
  private implementation detail 閉じ込め）
- `moxcms` 依存追加の正当化記録（暫定規約「依存追加」）:
  - **採用**: `moxcms = { version = "0.9", optional = true,
    default-features = false, features = ["lut", "extended_range"] }`
  - メンテナンス: 2025-02 初版・0.9.1 = 2026-09 リリースで活発
    （比較対象: `qcms` 0.3.0 は 2024-01 で停滞・`icc-profile`
    0.0.6 は Gray/RGB のみ・`tpt-colour` は採用実績不足・
    `lcms2` は C FFI で unsafe/ツールチェイン規約と構造不整合）
  - ライセンス: BSD-3-Clause OR Apache-2.0（依存 `pxfm` も同一
    デュアル・`num-traits` は MIT OR Apache-2.0）— licensing
    方針と整合
  - 依存木: moxcms + `num-traits` + `pxfm` = 3 crate・純 Rust・
    C ツールチェイン不要
  - MSRV: 宣言 1.89.0 = workspace `rust-version` と一致
    （edition 2024）
  - unsafe: `default-features = false` では moxcms 側で
    `#![forbid(unsafe_code)]` が有効化される（同 crate Cargo.toml
    記述）— SIMD feature を有効化しない選択と合わせて依存経路の
    unsafe を実質ゼロにする
  - feature 選定: `lut`（mft1/mft2/mAB LUT profile — display/
    printer profile の実カバレッジに必須）・`extended_range`
    （`allow_extended_range_rgb_xyz` — f32 HDR/広色域の [0,1] 外
    保持に必須）。SIMD 系（`avx`/`sse`/`neon`/`avx512`/`sve`）は
    有効化しない — golden vector のクロスホスト決定性と参照実装の
    性格を優先。`in_place`/`options`/`any_to_any` も初期不要
- feature `icc` の検証経路: `cargo test --workspace` は default
  feature のみを検証するため、実装では `cargo test -p
  ruviform-video-proc --features icc`（または `--all-features`）を
  実行する mise タスク/CI 手順を追加する — doc には feature 記述を
  crate doc に明記
- 依存は `ruviform-video-core`・`ruviform-media-core`（+ feature
  `icc` 時の `moxcms`）のみ — `ruviform-rt`・同順位・engine 層
  依存なし（ADR D6 不変）
- 公開 enum: `IccIntent` は enum レベル `#[non_exhaustive]`。
  `PixelFormat` 拡張は `#[non_exhaustive]` により non-breaking —
  既存カーネルの `_` arm は未知 variant を graceful 拒否し続ける

## Risks and Trade-offs

- **`RgbaF32` の公開 enum 拡張**: video-core の公開 API 面を広げるが、
  working space をフレーム間で受け渡す設計意図（linear-light 連結
  パイプライン）は内部 scratch では実現できない — `#[non_exhaustive]`
  で non-breaking・plane 算定は generic に従うため波及は
  `bytes_per_sample(0) == 16` の 1 行 + doc のみ。`RgbaF16` 流用案は
  PQ 10 000 nits 域の精度不足（binary16 significand 11 bit）で棄却
- **BT.2020 working primaries**: P3D65 案は BT.2020 ソースの gamut
  clipping を招く — 全 `ColorPrimaries` variant を包含する最広の
  民生規格 BT.2020 を採用（ACES AP0/AP1 は ADR D3-b で後続 —
  primaries 変換構造は ACES variant 追加で吸収可能）
- **`Unspecified` 既定の意味づけ**: 既定値みなし（`Bt709`）は
  unlabeled SDR 映像の最大公約数 — 厳密拒否は未タグ素材の一般的
  ケースを過剰拒否する（既存 `matrix → Bt601`・`range → Limited`
  の既定哲学に一致）。既定値が誤解釈を生むケースは doc で
  「解釈は consumer が確定する責務」と明記
- **stage 分割でなく融合 `ToWorking`/`FromWorking`**: decode→行列→
  encode の個別 stage カーネルは working-space ハブ構成で使い道が
  なくフレームパスを倍化する — YAGNI で融合。gamut-only/transfer-only
  経路は identity 行列・`Linear` の組合せで表現可能（別 stage
  カーネルは後続需要で追加可能）
- **HLG の nominal peak 1000 nits 固定**: OOTF の L_W は表示依存
  パラメータだが、`ColorInfo`/`HdrMetadata` はそれを搬送しない —
  BT.2100 nominal 1000 nits を規約値として固定・doc 明記（絶対輝度
  スケールに整合: PQ 1000 nits ≡ HLG 1.0 ≡ 10.0）
- **BT.2390 選択**: BT.2446 A/C は色空間補正を含む高精度系だが
  実装・検証コストが初期範囲を超える。EETF は決定的閉形式・
  HDR→SDR の標準化手法・`HdrMetadata` 消費と整合 — 他 operator
  （mobius/hable/BT.2446）は需要発生時の後続論点
- **moxcms `prefer_fixed_point = false`**: f32 executor 上の
  Q2.13 fixed-point 経路を外して float 経路に固定 — 参照実装の
  精度契約と一致（`extended_range` と併せて [0,1] 外保持）。
  `allow_use_cicp_transfer = true` は CICP profile の TRC を正確適用
- **per-pixel executor 呼出し**: `IccTransform` は行単位の画素配列
  スライスではなく 1 画素単位（1 要素 `&[[f32;4]]` スライス）で
  executor を呼ぶ — 行バッファ確保を避け alloc-free を構造保証する
  代わりに dyn call オーバーヘッドを許容（参照実装の性格に一致。
  バッチ経路は最適化 backend 論点）
- **feature `icc` 経路が default 検証対象外**: `cargo test
  --workspace` では feature-gated コードは untested — mise タスク/
  CI に `--features icc` 検証列を追加する義務を本設計で記録
  （未実施なら AC「検証列全パス」は base のみ意味を持つ）

## Deferred Design Concerns

- OCIO 設定消費・ACES IDT/ODT/RRT フル管 | ADR D2-b 保留・D3-b 後続（構造上は変換ポート追加で吸収可能 — working space の primaries 拡張 + IccTransform 同型の変換カーネル経路） | NLE 級の OCIO 要求・ACES ワークフロー要求の発生 | OCIO/ACES 前提の制作パイプラインが扱えない
- ICC intent 拡張集合（Saturation/AbsoluteColorimetric）・黒点補償・`options`/`any_to_any`/`in_place`/SIMD 系 moxcms feature・CMYK/多チャネル layout | 初期 `IccIntent` で映像入出力の必要十分な集合 | 印刷・proofing 需要の発生 | 色忠実度の要求を満たせない
- `ColorInfo` → CICP profile 便利コンストラクタ（`ColorProfile::new_from_cicp` 経由で CICP-described フレームを ICC 経路へ直結） | profile バイト列供給の実需が先 | media-io の ICC 供給経路確立 | 経路選択が呼出し側に残る
- planar YUV への直接 gamut/transfer 適用 | クロマ格子・符号化ドメインを考慮した設計が別論点 | YUV 直接処理の要求発生 | RGB 変換を介する二段処理が必要
- `FromWorking` の `RgbaF16` dst・`RgbaF32 ↔ RgbaF16` shuffle・10/12-bit integer パイプライン | `RgbaF32`/`Rgba8` で初期要件を充足 | 高ビット深度要求の発生 | f32 経路の精度/性能コストが残る
- トーンマップ対象ピークのパラメータ化（`L_DS` 固定 100 nits → 可変）・追加 tone operator（BT.2446/mobius/hable）・HLG L_W の設定化 | 初期 SDR 100 nits 固定で要件を充足 | 非 SDR ターゲット・品質要求の発生 | 適応範囲が限られる
- `stage` 分割カーネル（gamut-only・transfer-only の独立適用） | 融合経路 + identity 組合せで表現可能 | stage 単位適用の実需発生 | 2 パス化による性能コスト
- video-proc への `assert_no_alloc` 導入（`IccTransform` の executor 内部 alloc を含む機械検証レーン） | 現行 `rt_process_*` は determinism/非破壊/レビュー担保 — #97 設計と同じフォローアップ管轄 | 次の RT 保証強化 Issue | alloc 違反の機械検出なし — **Issue #126 で解決済み**: 全 `rt_process_*` へ `assert_no_alloc` 窓を適用し、moxcms executor 内部を含む per-process alloc 0 を実測（IR: [issue-126-video-proc-rt-lane-implementation.md](../impl/ir/issue-126-video-proc-rt-lane-implementation.md)）
- `moxcms` のバージョン/feature 更新時の golden 再導出手続き | pin + `Cargo.lock` で現状固定 | バージョン更新要求 | 等価性契約の基盤が暗黙変化し得る

## 実装単位

- `crates/ruviform-video-core/src/pixel.rs` — `RgbaF32` variant 追加
  （`plane_count`/`is_planar`/`bytes_per_sample(0) == 16`/
  `chroma_subsampling` + 画素レイアウト doc + テスト行）
- `crates/ruviform-video-proc/src/transfer.rs`（新規・`pub(crate)`）
  — transfer decode/encode 関数群（Linear/Bt709/Smpte170m/Pq/Hlg の
  定義式と clamp 規則）・primaries → BT.2020 結合行列の定数
  テーブル・`primaries_slot`/`transfer_slot` 解決関数（既定規則 +
  `_` arm 拒否）・working-space 定数（`L_DS = 100` nits・HLG
  nominal 1000 nits・`SDR_SCALE`）
- `crates/ruviform-video-proc/src/working_space.rs` — `ToWorking`・
  `FromWorking` + 各 doc（規約・`Unspecified` 解釈・メタデータ
  更新・等価性契約・edge case）
- `crates/ruviform-video-proc/src/tone_map.rs` — `ToneMapSdr` +
  BT.2390 EETF + `L_HS`/`L_min` 導出規則
- `crates/ruviform-video-proc/src/icc.rs` — `IccTransform`・
  `IccIntent`（`#[cfg(feature = "icc")]` 配下・moxcms 閉じ込め）
- `crates/ruviform-video-proc/src/composite.rs`・`src/affine.rs` —
  `RgbaF32` arm 追加（受理集合・f32 評価パス・doc 等価性契約更新）
- `crates/ruviform-video-proc/src/lib.rs` — カーネル一覧・feature
  `icc` 記述・re-export 追加・「色管理カーネル」予約記述の差替え
- `crates/ruviform-video-proc/Cargo.toml` — `moxcms` optional 依存 +
  `[features] default = []`・`icc = ["dep:moxcms"]`
- `crates/ruviform-video-proc/README.md` — カーネル一覧・feature・
  許可依存の更新
- `mise.toml` / CI — `--features icc`（または `--all-features`）で
  video-proc を検証するタスク/ステップ追加（base 経路の検証列は
  不変のまま）
- テストは各モジュール `#[cfg(test)] mod tests`（golden vector・
  端点・検証拒否・`rt_process_*`）— 既存規約どおり inline

## References

- docs/specs/color-conversion-kernels.md（本設計の要求元・
  acceptance criteria）
- docs/discovery/color-conversion-kernels.md（Open Questions・
  ICC エンジン比較・ギャップ分析）
- docs/designs/second-target-design.md（色管理行・責務分割・
  後続 Issue 分解 4 番・deferred 表）
- docs/adr/broad-color-management.md（D1-b f32 linear working
  space・D2-a ICC 経路 + OCIO 保留・D3-b ACES 後続）
- docs/adr/video-proc-kernel-api-and-color-hdr-scope.md（D1-b 二相
  API・D3-a メタデータ搬送規則・D6 配線・D7 エラーモデル）
- docs/designs/ruviform-architecture-design-baseline.md（層構造・
  依存方向・RT 契約・DSP/SIMD 決定）
- docs/specs/composite-transition-kernels.md /
  docs/designs/composite-transition-kernels.md（同一クレートの兄弟
  設計 — カーネル規約・`RgbaF32` 拡張の対象・linear-light deferred）
- docs/designs/ofx-host-adapter.md・docs/specs/edit-data-model.md /
  docs/designs/edit-data-model.md（同一 Issue 分解系列の兄弟成果物）
- ITU-R BT.2390-8 §5.4（EETF Hermite スプライン）・ITU-R BT.2100
  （HLG OETF/OOTF・PQ = SMPTE ST 2084）・ITU-T H.273（primaries/
  transfer 符号）
- moxcms 0.9.1 — `docs.rs/moxcms`（`ColorProfile::new_from_slice`/
  `new_from_cicp`・`create_transform_f32(Layout, &ColorProfile,
  Layout, TransformOptions) -> Result<Arc<TransformF32Executor>,
  CmsError>` — f32 変換は extended mode で負値・1 超を返し得る・
  `TransformExecutor<V>::transform(&[[V]], &mut [[V]])`・
  feature 定義 — `default-features = false` 時は
  `forbid(unsafe)` が有効と Cargo.toml に明記）

## Verification Notes

- [x] ADR `broad-color-management` D1-b — working space =
  `RgbaF32` 保持の f32 linear + `ColorInfo` primaries 明示
  （`Bt2020`/`Linear`/`Identity`/`Full` の記録規則を定義）
- [x] ADR `broad-color-management` D2-a — ICC 経路を moxcms で実装、
  opt-in 分離 = feature `icc`（依存追加の正当化を本書に記録、
  OCIO は構造維持で後続）
- [x] ADR D1-b（video-proc）— 全カーネルが非 RT ctor → alloc-free
  `process` の二相形状。`RtContext`・`Processor` trait は参照しない
- [x] ADR D7 — `ruviform_media_core::Error`/`Result` を再利用。
  検証は書込み前・panic-free・部分書込みなし。ctor 失敗は
  `Result<Self>`（`RgbToYuv` 先例）
- [x] 二ターゲット設計の deferred 表 — HDR→SDR トーンマップ個別方式
  を BT.2390 EETF + `HdrMetadata` 消費規則で確定
- [x] spec Requirements — base 依存限定（feature off で外部依存
  ゼロ）・`#[non_exhaustive]`・`rt_` 規約・メタデータ規約の doc
  明記・等価性契約のカーネル毎定義
- [x] discovery Open Questions — (a) `RgbaF32` 拡張・(b) moxcms +
  feature `icc`・(c) BT.2020 + 既定表・(d) BT.2390 + `HdrMetadata`
  優先順・(e) 4 新規 + 3 拡張カーネル・(f)/(g) 二相分離 +
  `InvalidValue` — 全項目を確定
