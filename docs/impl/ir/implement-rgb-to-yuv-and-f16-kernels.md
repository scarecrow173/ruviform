---
id: "IMPL-034JgwwQXOaYpCHRnN2X1e"
type: "impl"
status: "completed"
title: "Implement rgb to yuv and f16 kernels"
created: "2026-10-20"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/26"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-video-proc/src/rgb_to_yuv.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/src/rgba_f16.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-video-proc/src/coef.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/src/lib.rs"
      - type: "file"
        path: "docs/tasks/implement-rgb-to-yuv-and-f16-kernels.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/implement-rgb-to-yuv-and-f16-kernels.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/implement-issue-26-video-foundation.md"
    - "docs/specs/implement-video-foundation-api.md"
    - "docs/adr/video-proc-kernel-api-and-color-hdr-scope.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/video-foundation-api.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Implement rgb to yuv and f16 kernels

## Summary

task `TASK-034JcD2Xk1x6RZjHn779an`（plan `PLAN-034JcCFGe6ILSTWLgUIjAn` Phase D）として、`ruviform-video-proc` に `RgbToYuv`（`Rgba8` → `Yuv420p`/`Yuv422p`/`Yuv444p`）と `Rgba8ToRgbaF16`/`RgbaF16ToRgba8` の 3 カーネルを実装した（spec AC7 後半・AC8・AC9）。係数導出は T3a の `coef.rs` を拡張（`RgbCoef` + `rgb_to_yuv_table` + slot→enum 逆写像 `RESOLVED_MATRIX`/`RESOLVED_RANGE`）して共有。plan D-2 の primitive `f16` 仮定は MSVC 系ターゲットで unstable のため成立せず、IEEE 754 binary16 のソフトウェアコーデック（`f16::from_f64` とビット一致の RNE エンコード + exact デコード）を crate 内部に実装した。chroma downsample（Q16 ドメイン平均）・出力 `ColorInfo` 更新規則・等価性契約を kernel doc に明記し、golden vector・エッジケース・`rt_` 契約テスト・doctest で検証した。

## Context

- 実施タスク: [../../tasks/implement-rgb-to-yuv-and-f16-kernels.md](../../tasks/implement-rgb-to-yuv-and-f16-kernels.md)
- 上流: [plan](../../plans/implement-issue-26-video-foundation.md) Phase D（D-1/D-2）と検証 V-4、Open Question Q2/Q3/Q4、Risk Register 4/5/8/9、[spec](../../specs/implement-video-foundation-api.md) AC6–AC9、[ADR](../../adr/video-proc-kernel-api-and-color-hdr-scope.md) D1-b/D2-a/D3-a/D6/D7-a、[設計ベースライン](../../designs/ruviform-architecture-design-baseline.md)（RT 契約・DSP/SIMD 決定）
- 契約保証の正本: [test-spec](../../test-specs/video-foundation-api.md)
- 前提: T3a（[implement-yuv-to-rgb-kernel.md](implement-yuv-to-rgb-kernel.md)）の `coef.rs` 共有ロジックと `YuvToRgb` 参照実装。外部依存ゼロ・`unsafe` 不使用

## Implementation

### Changes

- `src/coef.rs` — `RgbCoef`（`y_base` + 9 本の Q16 係数と `luma`/`chroma` 演算;`chroma` は `[i32; 2]` の Q16 ドメイン値を返し downsample の量子化前平均に対応）+ `rgb_to_yuv_table()`（`MATRIX_WEIGHTS`/`RANGE_SCALE` を `yuv_to_rgb_table` と共有、encode 側は逆ゲイン `1/luma_gain`・`1/chroma_gain` + `1/(2·(1−K))` 正規化を畳込）+ `RESOLVED_MATRIX`/`RESOLVED_RANGE`（適用係数 slot → 出力 `ColorInfo` 記録値の逆写像）。`clip8` を `pub(crate)` 化して rgb_to_yuv が再利用
- `src/rgb_to_yuv.rs`（新規）— `RgbToYuv { coef: RgbCoef, matrix, range }`。`new(matrix, range) -> Result<Self>`（Q2: 係数は構築引数、`matrix_slot`/`range_slot` で解決+検証+索引を一元化、`Unspecified`→Bt601/Limited、`Identity`/未知 variant → `Err(InvalidValue)`、`_` arm）。`process` は `**RT contract:**` RT-suitable — 全検証（src=`Rgba8`・dst∈{420,422,444}・寸法一致・`plane_mut` の排他/存在）を書込前に完了。chroma downsample: 420=2×2 セル・422=水平ペア・444=画素単位、Q16 値を `i32` 和で平均（round-half-up 付き）してから `clip8`、奇数寸法は存在画素のみ（siting 前提: 水平 co-sited・垂直 center = T3a と対の規約）。出力 `ColorInfo`（matrix/range→適用値・残り伝播）+ `pts`/`duration` 伝播
- `src/rgba_f16.rs`（新規）— `Rgba8ToRgbaF16`/`RgbaF16ToRgba8`（unit struct、`new()` は失敗し得ない + `Default` impl）。**binary16 はソフトウェアコーデック**: `u8_to_f16_bits`（`f64::from(v)/255.0` → RNE の `f64_to_f16_bits` — `f16::from_f64` と全入力ビット一致を検証済み）と `f16_bits_to_u8`（exact デコード `f16_bits_to_f64` → NaN fold → `clamp(0,1)` → `·255` → `f64::round`）。`ColorInfo`・`pts`/`duration` は不変伝播
- `src/lib.rs` — `RgbToYuv`/`Rgba8ToRgbaF16`/`RgbaF16ToRgba8` の再エクスポート + crate doc「Kernels」節追記
- `docs/tasks/implement-rgb-to-yuv-and-f16-kernels.md` — Work/Done When/Verification 記入・`status: done`・`implemented-by` 設定

### Decisions During Implementation

- **primitive `f16` は使用不可（plan D-2 の仮定を代替）**: Rust 1.82 で stabilize したのは一部ターゲットのみで、`*-pc-windows-msvc`（= 本ホスト・CI matrix の windows レーン）では Rust 1.99.0 時点でも `#![feature(f16)]` ゲートのまま unstable（rust-lang/rust#116909）。nightly 化・外部クレート導入・`unsafe` のいずれも制約違反なので、契約の本体である LE binary16 バイト列をソフトウェアで encode/decode する実装を選択 — `f64_to_f16_bits` は IEEE RNE（tying-to-even）で `f16::from_f64` とビット一致、`f16_bits_to_f64` は exact。仕様・ワイヤ互換に差分なし
- **`RgbToYuv` の係数は構築引数（Q2 既定どおり）**: `new(matrix, range)` が `matrix_slot`/`range_slot` で `Result` 解決し、適用した値を `RESOLVED_MATRIX`/`RESOLVED_RANGE` 経由で出力 `ColorInfo` に記録 — src の `matrix`（`Identity`/`Unspecified` が自然）に依存しない
- **chroma downsample は Q16 ドメインで平均してから `clip8`**: per-pixel chroma を `RgbCoef::chroma`（`i32` Q16 値、`CHROMA_CENTER·2¹⁶` 加算済み）で得て、セル内 `i32` 和 → 個数除算 → 半分の端数は `+count/2` の round-half-up → `clip8`。先に u8 量子化してから byte 平均する経路とは結果が異なり得る（識別テスト `chroma_downsample_420_averages_in_q16_domain` で固定）。和は `i32` で不敗（|Q16 chroma| ≲ 3×10⁷/画素 × セル ≤4 画素）。平均自体は整数演算なので等価性契約はビット完全一致を維持
- **siting 前提は T3a と対の規約**: 水平 co-sited・垂直 center（up 側の最近傍複写と一致する前提）を kernel doc に明記 — 既知リスクは spec/plan Risk 8 で受容済み
- **f16 の等価性契約**: forward（u8→binary16）は正準値（`v/255` の最近傍）に対し **1 ulp（binary16 ドメイン）**、inverse（binary16→u8）は **ビット完全一致** — 入力ビットパターンから出力 byte が一意に決まる経路
- **NaN 規則は明示的 fold**: `f64::clamp` は NaN を伝播する（`as u8` 飽和キャストで最終的に 0 になるが）規則の可読性のため `is_nan` → `0.0` を明示（clippy `manual_clamp` 回避にも整合）
- **エラーモデル・共有 dst 拒否・検証順序は T3a と同一規約**: `Error::InvalidValue(&'static str)` の field-ish payload、`dst.plane_mut(..)==None` → `Err("dst plane")`、全検証を書込前に完了
- **テスト命名**: `rt_` 部分文字列が非 RT テスト名に混入しないよう命名（Risk 5）。clippy `similar_names` 対応で `cb_*`/`cr_*` の平行 binding は `blue_*`/`red_*`（係数）・配列インライン化（テスト期待値）に変更
- **roundtrip テストのセル構成**: 「4×2 で `(y*4+x)/2` 配色」は chroma cell が 2 色混在になる誤りだったため、4×4・`y/2*2+x/2`（真の 2×2 均一セル×4 色）に修正 — `YuvToRgb` 復号との往復で pixel-exact を検証

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

実施環境: Windows x64（`x86_64-pc-windows-msvc`、rustc 1.99.0）・Git Bash。task の `## Verification` 全項目を実行:

| Check | コマンド | 結果 |
| --- | --- | --- |
| テスト | `mise exec -- cargo test -p ruviform-video-proc` | 41 unit + 9 doctest 全合格 |
| RT レーン | `mise exec -- cargo test -p ruviform-video-proc rt_ -- --list` | rgb_to_yuv 2 + rgba_f16 4 + yuv_to_rgb 2 = 8 件のみ選択・合格 |
| lint | `mise exec -- cargo clippy -p ruviform-video-proc --all-targets -- -D warnings` | 0 warnings（`similar_names`/`doc_markdown`/`manual_clamp`/`cast_sign_loss`/`cast_possible_truncation` を修正して収束） |
| 整形 | `mise run fmt` / `fmt-check` | 差分なし |
| 型チェック | `mise run check` | exit 0 |
| ワークスペース | `mise run lint`・`mise run test`・`mise run test-rt` | 全 PASS |
| 配線 | `mise exec -- cargo metadata --format-version 1 --no-deps` | video-proc deps = media-core + video-core のみ・`publish: []` |
| docs 監査 | `node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` | 0 findings |
| IR 監査 | `node .agents/skills/impl-doc/scripts/audit_impl_record.js --json` | 0 findings |

golden vector カバレッジ: 3 matrix × 2 range × 6 画素（neutral・3 原色・黒・白）。期待値は H.273 重みから Python で独立導出し導出式をコメント記載（Risk 9）。chroma downsample は 420 奇数寸法（5×3）・422 水平ペア+奇数列（3×2）・444 画素単位・Q16 平均識別テストで網羅。f16 は 0/128/255・NaN・±inf・負・>1・±0・subnormal 端と全 256 コード往復で検証。binary16 ソフトウェアコーデック自体は実装前に Python `struct 'e'` 対比で全 65536 パターン + ランダム/エッジ値をビット一致確認済み。

## Risks

- plan D-2 が前提とした primitive `f16` は windows-msvc で unstable — ソフトウェアコーデックで代替済み（出力ビット列は非依存）。upstream で f16 が stabilize されても差替不要（契約はバイト列）だが、`f64_to_f16_bits` の独自実装は長期的な保守負担になり得る（全入力検証済みなのでリスクは低い）
- siting 前提（水平 co-sited・垂直 center）は暗黙仮定 — spec/plan Risk 8 で受容済み、up/down 両方向で doc 明記
- `process` の alloc-free は `rt_` テスト（反復成功・src 不変）とコードレビューで担保 — アロケータ観測は `unsafe` 禁止のため非対象
- Q16 chroma 平均の `i32` 和: セル画素数 ≤ 4（422 は 2、444 は 1）で |Q16 chroma| ≲ 3×10⁷/画素 → 合計 ≲ 1.2×10⁸ ≪ `i32::MAX` でオーバーフロー不敗（係数の有界性は `coef::q16` 側の doc で保証）

## Follow-ups

- T4（[sync-docs-issue-26.md](../../tasks/sync-docs-issue-26.md)）: baseline 採否表（色変換・HDR 行）・`AGENTS.md`・`README.md` の同期と総合検証 — 本タスクの完了が前提
- spec/plan へのフィードバック（T4 の検討対象）: plan D-2・Q3 の「primitive `f16`（MSRV 1.85 で可）」は windows-msvc で成立しない — Q3 の値規則（`v/255`・round-half-up）自体は維持されたが、記述の修正または「binary16 コーデック実装」の選択肢記載が望ましい
