---
id: "DISC-034LEXAPQpsGvnYeoKEahA"
type: "discovery"
status: "resolved"
title: "色変換カーネル（f32 linear working space + ICC 入出力経路）の探索（Issue #99）"
created: "2026-10-25"
updated: "2026-10-25"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/99"
    - "https://crates.io/crates/moxcms"
    - "https://github.com/awxkee/moxcms"
    - "https://crates.io/crates/lcms2"
    - "https://crates.io/crates/qcms"
    - "https://crates.io/crates/icc-profile"
    - "https://opensource.tptsolutions.co.nz/tpt-colour"
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
  related:
    - "../adr/broad-color-management.md"
    - "../adr/video-proc-kernel-api-and-color-hdr-scope.md"
    - "../designs/second-target-design.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/color-conversion-kernels.md"
    - "../designs/color-conversion-kernels.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 色変換カーネル（f32 linear working space + ICC 入出力経路）の探索（Issue #99）

## Exploration Goal

Issue [#99](https://github.com/scarecrow173/ruviform/issues/99) は `ruviform-video-proc` への**色管理カーネル群**の段階追加を要求する — f32 linear working space 変換カーネル（gamut/transfer 適用）と入出力 ICC 変換経路。[第 2 ターゲット設計](../designs/second-target-design.md)（approved）「後続 Issue 分解」順序案 4 番に相当する。

既存の上流成果物は [broad-color-management.md](../adr/broad-color-management.md)（accepted — D1-b f32 linear working space・D2-a ICC 経路 + OCIO 保留・D3-b ACES 後続）と [video-proc-kernel-api-and-color-hdr-scope.md](../adr/video-proc-kernel-api-and-color-hdr-scope.md)（accepted — D1-b 二相 API・D3-a HDR = メタデータ搬送+非破壊・D6 配線・D7 エラーモデル）。本 Issue の構造的判断はこれらで確定済みであり、本探索は **issue framing・確認済みギャップ・ICC エンジン候補の比較・境界条件**を記録する。採否・方式の確定権限は design フェーズにあり、本書は最終判断を含めない。

## Key Issues and Assumptions

### ローカル構造の確認済みギャップ（confirmed）

- **`PixelFormat` に f32 表現がない**: 現行は `Yuv420p`/`Yuv422p`/`Yuv444p`（8-bit planar）・`Rgba8`・`RgbaF16` のみ（`crates/ruviform-video-core/src/pixel.rs`）。f32 linear working space を `VideoFrame` で保持するには pixel-format 拡張（例: `RgbaF32`）かカーネル内部の非公開 f32 表現が必要 — どちらを取るかは design の論点
- **`ColorPrimaries` に ACES 系 variant がない**: 現行は `Bt709`/`Smpte170m`/`Bt470Bg`/`Bt2020`/`P3D65`（`crates/ruviform-video-core/src/color.rs`）。ACES は ADR D3-b で後続確定のため初期範囲では不要。working space の primaries 選択と `ColorInfo` への写像（ADR consequences: 「新設の working-space 記述子との写像は design で定義」）は design 論点
- **`TransferCharacteristics` は初期範囲を既にカバー**: `Linear`/`Bt709`/`Smpte170m`/`Pq`/`Hlg` が存在し、transfer 適用カーネルの入出力量子化を enum 追記なしで表現できる
- **`VideoFrame.color` は値型 `ColorInfo`（`Copy`・非 Option・`set_color` writer）**: 既存 `YuvToRgb` と同様、カーネルが呼出し毎に `src.color()` を読む規約を踏襲可能。working space 変換後の `primaries`/`transfer` 更新規則はカーネルごとの doc 明記が必要（#97 spec のメタデータ規約と同型）
- **`Unspecified` の既定解釈が primaries/transfer に未定義**: 既存規約は `matrix: Unspecified → Bt601`・`range: Unspecified → Limited` のみ（ADR D3）。gamut/transfer カーネルが `Unspecified` primaries/transfer をどう扱うか（既定値みなし vs 拒否）は未確定 — design 論点
- **規約上の制約**: `unsafe` 原則禁止・MSRV 1.89・default feature 最小・依存追加には理由記録が必要（AGENTS.md 暫定規約）。C FFI ベースの CMS 直採用は `unsafe`/ビルドツールチェイン導入と衝突する

### 依存メタデータ（観測値 — 採否証明ではない）

宣言値は package メタデータ由来で実コンパイル証明ではない。保守状況・依存木・実効 MSRV の確証は採用前に別途確認する。

| crate | 観測 version | declared MSRV | license | 依存（宣言） | 構造 |
| --- | --- | --- | --- | --- | --- |
| `moxcms` | 0.9.1（2026-09 更新・活発） | 1.89（宣言 — workspace MSRV と一致） | BSD-3-Clause OR Apache-2.0 | `num-traits` + `pxfm` | pure Rust・SIMD 経路は feature 選択式 |
| `lcms2` | 6.2.0 | 未確認 | MIT | `lcms2-sys` + `foreign-types` + `bytemuck` | Little CMS（C ライブラリ）の FFI wrapper |
| `qcms` | 0.3.0（2024-01・更新停滞気味） | 未宣言（`rust_version` 未確認） | MIT | `libc`（optional・c_bindings のみ） | pure Rust（Firefox 由来） |
| `icc-profile` | 0.0.6（初期版） | 未確認 | MIT OR Apache-2.0 | `bin-rs` | pure Rust・f32 コア・Gray/RGB のみ（CMYK/MPE 非対応） |
| `tpt-colour` / `tpt-colour-icc` | 新興（採用実績少） | 1.85（宣言・simd feature は 1.95） | 要確認 | — | pure Rust no_std・ICC v2/v4 + gamut/tone-map を含む |

## Alternatives and Comparison

### 軸 1: ICC エンジン（採否は design で確定 — 本書は比較のみ）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) `moxcms` | pure Rust（unsafe/FFI 方針と整合）・declared MSRV 1.89 で workspace と一致・デュアルライセンス適合・依存 2 件で小さい・LUT/shaper 最適化が feature 選択式・RGBX↔RGBX に加え CMYK/LAB/GRAY・Display class 広範 | 比較的新しいクレート（2025-02 初版）・実効 MSRV と SIMD feature 既定集合の精査が要 | candidate |
| (b) `lcms2`（Little CMS FFI） | 実績最大・機能完全 | C 依存 + FFI `unsafe` が「unsafe 原則禁止」暫定規約と衝突・C ツールチェイン導入が必要 | no（構造不整合） |
| (c) `qcms` | pure Rust・Firefox 実績 | 更新が 2024-01 で停滞・intent/LUT カバレッジが狭い | maybe |
| (d) `icc-profile` | pure Rust・f32 処理コア・checked parser | 0.0.x・Gray/RGB のみで LUT 網羅性に欠ける | maybe |
| (e) `tpt-colour(-icc)` | pure Rust no_std・ICC + gamut mapping + tone map を包含 | 新興・採用実績が乏しく依存追加規約の「メンテナンス状況」証明が弱い | maybe |
| (f) 自前の最小 ICC パーサ + 行列/TRC 変換 | 外部依存ゼロ維持 | ICC v4 構造・LUT(mft1/mft2/mAB)・tetrahedral 補間の実装・検証コストが本 Issue の責務を超える | no |

### 軸 2: ICC 依存の分離形（Issue 本文の「opt-in 分離方針」— 形の確定は design）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) `ruviform-video-proc` の Cargo feature（optional dep） | 単一クレートでカーネル規約を共有・feature off で依存ゼロ維持 | feature 有効時に依存が実クレートに入る — 既定 feature 最小規約の運用を design で定義する必要 | candidate（仮説） |
| (b) 別クレート分離（`ruviform-video-proc` とは別の ICC アダプタ） | ベース crate の依存ゼロを構造的に保証・audio-device の adapter パターンに類似 | クレート新設コスト・層構造（同順位の処理系）への位置づけが要確認 | candidate（仮説） |
| (c) 直接の恒常依存 | 最も単純 | 「default feature 最小」規約・既存依存構造（video-core + media-core のみ）に反する | no |

### 軸 3: working space の保持形

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) `PixelFormat` に f32 packed 拡張（例: `RgbaF32`） | working space が `VideoFrame`/`ColorInfo` 規約に乗りパイプライン間を受け渡せる | video-core の公開 enum 拡張（`#[non_exhaustive]` により non-breaking）・plane 幅計算の波及確認が要 | candidate（仮説） |
| (b) カーネル内部の非公開 f32 scratch | video-core 非侵襲 | working space が frame をまたげず「f32 linear 上で連結処理」の設計意図を満たさない恐れ | maybe |
| (c) `RgbaF16` を working space 保持に流用 | 型追加なし | f32 linear 前提の精度・範囲（PQ 10000 nits）に f16 精度不足 | no |

## Tentative Conclusions and Hypotheses

- **confirmed**: カーネル API 形状（二相 prepare/process・`RtContext` なし・`Processor` 非実装）・エラーモデル（`media_core::Error`）・依存方向・`#[non_exhaustive]`・`rt_` 検証規約は accepted ADR で確定済み — 本 briefing で新規 ADR は不要
- **confirmed**: f32 ピクセル表現の欠如は最初に前提が崩れる地点 — design の事前課題（video-core 拡張か内部表現か）
- **confirmed**: ADR broad-color-management D2-a は `moxcms` を「有力候補」としつつ採否を「実装 Issue の依存追加規約」に委任 — Issue 本文も「採否は本実装 Issue の設計で確定」。よって ICC エンジン選定は **design フェーズの決定事項**であり briefing 時点の ADR ではない
- **hypothesis**: (a) `moxcms` + (a/b) opt-in 分離が規約（pure Rust・MSRV・依存最小）との整合が最も高い — 確定は design
- **hypothesis**: `RgbaF32` 級の `PixelFormat` 拡張が working space 保持の自然形 — `#[non_exhaustive]` により non-breaking、確定は design
- **deferred（Issue スコープ外として確認済み）**: OCIO 設定消費・ACES フル管（ADR D2-b 保留・D3-b 後続）・GPU カーネル（ruviform-gpu Issue）・planar YUV への直接適用

## Open Questions

> すべて design フェーズが答える論点 — architecture 未決事項ではなく実装設計の確定対象。

- working space の primaries 選択（Bt2020 広色域か P3D65 か）と `ColorInfo` への写像・記述子の要否
- HDR→SDR トーンマップ個別方式（設計 deferred 表の当該項目 — `HdrMetadata`（MaxCLL/MaxFALL/mastering display）を参照するか含む）
- ICC エンジン採否 + opt-in 分離の形（feature vs 別クレート）+ intent/黒点補償の初期範囲
- f32 ピクセル表現の形（`PixelFormat` 拡張 vs 内部表現）と対象入力フォーマット範囲
- `Unspecified` primaries/transfer の既定解釈規則
- gamut/transfer カーネルの初期演算集合（リニア化→ primaries 行列→再符号化の stage 分割か単一合成か）と等価性契約の ulp bound

## Promotion Candidates

- [x] spec-doc: `../specs/color-conversion-kernels.md`（**created — proposed**。本探索の issue framing・scope 境界・ギャップを spec として formalize）
- [x] adr-doc: **不要と判断** — 構造的决定（working space・ICC 経路採用方針・ACES/OCIO 段階・API 形状・エラーモデル）は accepted ADR `broad-color-management` + `video-proc-kernel-api-and-color-hdr-scope` で網羅済み。唯一の open な選択（ICC エンジン採否）は ADR D2-a が「実装 Issue の依存追加規約」へ明示委任しており design フェーズの責務 — briefing 時点で決定していない選択を ADR に記録しない（先例: ライブラリ選定は採否確定時に ADR 化 — `select-audio-device-backend` 等）
- [ ] design-doc: 次ノード（design）で起票 — Open Questions の確定が責務

## Sources and Relations

- Issue [#99](https://github.com/scarecrow173/ruviform/issues/99)・親 Issue [#1](https://github.com/scarecrow173/ruviform/issues/1)・設計 Issue #32・関連 Issue #96（edit 射影の色変換需要側）・#97（linear 合成を本 Issue に deferred）・#98（OFX — 色変換を #99 として除外）
- 一次文書: [second-target-design.md](../designs/second-target-design.md)（色管理行・責務分割・後続 Issue 分解・deferred 表）・[broad-color-management.md](../adr/broad-color-management.md)（accepted）・[video-proc-kernel-api-and-color-hdr-scope.md](../adr/video-proc-kernel-api-and-color-hdr-scope.md)（accepted）
- ローカル一次証拠: `crates/ruviform-video-core/src/color.rs`（`ColorInfo`/`ColorPrimaries`/`TransferCharacteristics`/`HdrMetadata`）・`pixel.rs`（`PixelFormat` — f32 不在）・`frame.rs`（`color`/`set_color`）・`crates/ruviform-video-proc/src/lib.rs` + `yuv_to_rgb.rs`（既存カーネル規約・`ColorInfo` 更新規則の先例）
- ICC 候補クレート観測: crates.io `moxcms` 0.9.1・`lcms2` 6.2.0・`qcms` 0.3.0・`icc-profile` 0.0.6・tpt-colour プロジェクトページ（上記 `relations.source`）
