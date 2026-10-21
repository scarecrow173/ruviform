---
id: "IMPL-034JfTIFdaBP0PDUkO9yLm"
type: "impl"
status: "completed"
title: "Implement yuv to rgb kernel"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/26"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-video-proc/src/coef.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/src/yuv_to_rgb.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-video-proc/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "docs/tasks/implement-yuv-to-rgb-kernel.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/implement-yuv-to-rgb-kernel.md"
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

# Implement yuv to rgb kernel

## Summary

task `TASK-034JcD1UTgLlW1lRoWI1Ng`（plan `PLAN-034JcCFGe6ILSTWLgUIjAn` Phase C）として、`ruviform-video-proc` に二相カーネル API の最初の実装例となる `YuvToRgb` カーネル（`Yuv420p`/`Yuv422p`/`Yuv444p` → `Rgba8`）を実装した（spec AC6・AC7 前半・AC8・AC9）。`ruviform-media-core` の `Error`/`Result` path 依存を追加し（ADR D7-a）、係数導出は crate 内部の `coef.rs` に集約（Q16 固定小数点、T3b が再利用）。chroma upsample・出力 `ColorInfo` 更新規則・等価性契約（ビット完全一致）を kernel doc に明記し、golden vector・エッジケース・`rt_` 契約テスト・doctest で検証した。

## Context

- 実施タスク: [../../tasks/implement-yuv-to-rgb-kernel.md](../../tasks/implement-yuv-to-rgb-kernel.md)
- 上流: [plan](../../plans/implement-issue-26-video-foundation.md) Phase C（C-1〜C-5）と検証 V-3、Risk Register 4/5/8/9、Open Question Q4、[spec](../../specs/implement-video-foundation-api.md) AC6–AC9、[ADR](../../adr/video-proc-kernel-api-and-color-hdr-scope.md) D1-b/D2-a/D3-a/D7-a、[設計ベースライン](../../designs/ruviform-architecture-design-baseline.md)（RT 契約・DSP/SIMD 決定・層内順序）
- 契約保証の正本: [test-spec](../../test-specs/video-foundation-api.md)
- 前提: T1（video-core 契約文書化）・T2（video-proc 骨格）完了済み。外部依存ゼロ・`unsafe` 不使用

## Implementation

### Changes

- `crates/ruviform-video-proc/Cargo.toml` — `ruviform-media-core = { path = "../ruviform-media-core" }` 追加（`Error`/`Result` 使用の実需が生じたため ADR D7-a どおり。`Cargo.lock` 自動更新）
- `src/coef.rs`（新規・crate 内部）— `matrix_slot`/`range_slot`（`Unspecified`→Bt601/Limited 既定規則、`Identity`/未知 variant → `Err(InvalidValue)`、`_` arm）+ `YuvCoef`（Q16 係数 6 本 + `apply` 画素演算）+ `yuv_to_rgb_table`（`MATRIX_WEIGHTS`/`RANGE_SCALE` 定数テーブルから 6 組を導出、`std::array::from_fn`）
- `src/yuv_to_rgb.rs`（新規）— `YuvToRgb`（`new()` で 6 係数を bake、non-RT only。`process` は `**RT contract:**` RT-suitable — alloc-free・non-blocking・`width`×`height` 有界。全検証は書込前、行借用は構造上不敗）。chroma upsample = 最近傍複写（siting 前提: 水平 co-sited・垂直 center を doc 明記）、`row()`/`row_mut()` 単位処理、alpha=255、出力 `ColorInfo`（matrix→Identity・range→Full・残り伝播）+ `pts`/`duration` 伝播
- `src/lib.rs` — crate doc に「Kernels」節追加（`YuvToRgb`・スカラー参照実装 + 等価性契約の原則・T3b の後続カーネル）、依存層記述を media-core 実使用に同期、「Skeleton」文言削除、`pub use yuv_to_rgb::YuvToRgb` + `mod` 宣言
- `docs/tasks/implement-yuv-to-rgb-kernel.md` — Work/Done When/Verification 記入・`status: done`・`implemented-by` 設定

### Decisions During Implementation

- **係数は `new()` で全 6 組（3 matrix × 2 range）を bake し、`process` が `src.color()` を毎回解決して選択**。spec「変換は入力フレームの `ColorInfo` を参照」の文言どおりにしつつ、f64 係数導出は prepare 側に留める（二相形状と per-frame メタデータの両立）。`Unspecified`/`Identity`/未知 variant の解決は `coef::matrix_slot`/`range_slot`（slot index を返す＝解決+検証+索引を一元化）に集約し T3b が再利用する形
- **Q16 固定小数点の整数演算のみ**（`y_scale`/`cr_to_r`/`cb_to_g`/`cr_to_g`/`cb_to_b` + round-half-up + clamp）。等価性契約は「ビット完全一致」（浮動小数点を含まないため baseline L375 どおり）。係数は `Kr`/`Kb` から f64 で導出して `round(x·2¹⁶)` — 規格値の由来をコード上に残す
- **chroma upsample は最近傍複写**（chroma cell (cx,cy) が luma `2cx..=2cx+1`・420 は `2cy..=2cy+1` をカバー）。siting 前提=水平 co-sited・垂直 center と整合（各行が等距離）し、奇数寸法は `div_ceil` グリッド上の存在セルのみ使用 — `x/2`・`y/2` インデックスが自然に端数規則を満たす
- **出力 `range` は常に `Full`** — RGB 出力は全コード空間を使うため実値を記録（spec「range 変換を行った場合は実値に更新」の解釈を doc 明記）
- **Q4 解決**: `process` は `dst` の `pts`/`duration` を `src` から伝播（色変換は時刻を変えない既定どおり、kernel doc 明記）
- **エラーモデルは media-core `Error::InvalidValue(&'static str)` のみ**（variant 追加なし — 非破壊性保つため。payload は `"src format"`/`"dst format"`/`"dimensions"`/`"color matrix"`/`"color range"`/`"src planes"`/`"dst plane"` の field-ish 文字列、video-core 既存規約どおり）
- **共有 dst 拒否**: `dst.plane_mut(0) == None` を `Err(InvalidValue("dst plane"))` へ写像（T1 の書込ゲート観測点をそのまま利用。部分書き込みなしは「全検証を書込前に実施」で保証）
- **`new()` は no-arg・`Self` 返し + `Default` impl**（全組合せを bake するため失敗し得ない。`clippy::new_without_default` 対応）
- **テスト命名**: `rt_` 部分文字列が非 RT テスト名に混入しないよう命名（`assert_`/`convert_`/`start_` 系を避ける Risk 5 対応）。clippy `similar_names` 対応で chroma 変数は `blue_*`/`red_*`（Cb=青差分・Cr=赤差分の semantics に整合）

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

実施環境: Windows x64・Git Bash。task の `## Verification` 全項目を実行:

| Check | コマンド | 結果 |
| --- | --- | --- |
| テスト | `mise exec -- cargo test -p ruviform-video-proc` | 15 unit + 3 doctest 全合格 |
| RT レーン | `mise exec -- cargo test -p ruviform-video-proc rt_` | `rt_process_is_repeatable_and_deterministic`・`rt_process_never_mutates_source` の 2 件のみ選択・合格（13 filtered） |
| lint | `mise exec -- cargo clippy -p ruviform-video-proc --all-targets -- -D warnings` | 0 warnings（`similar_names`/`match_same_arms`/`cast_possible_truncation`/`used_underscore_binding` を修正して収束） |
| 型チェック | `mise run check` | exit 0 |
| 整形 | `mise run fmt` / `fmt-check` | 差分なし |
| ワークスペース | `mise run lint`・`mise run test`・`mise run test-rt` | 全 PASS（rt_ レーンは意図した集合のみ） |
| 配線 | `mise exec -- cargo metadata --format-version 1 --no-deps` | video-proc deps = media-core + video-core のみ・`publish: []` |
| docs 監査 | `node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` | 0 findings |
| IR 監査 | `node .agents/skills/impl-doc/scripts/audit_impl_record.js --json` | 0 findings |

golden vector カバレッジ: 3 matrix（Bt601/Bt709/Bt2020）× 2 range（Limited/Full）= 6 組 × 4 画素（neutral・赤系・緑系・コード空間端 (255,255,255)）。Bt709/Bt2020 Limited の bar 色で R/B のクランプ経路をカバー。期待値は H.273 重みから独立導出し導出式をコメント記載（Risk 9 対応）。

## Risks

- siting 前提（水平 co-sited・垂直 center）は暗黙仮定 — 相違コンテンツで半画素ずれ（既知リスク・spec/計画 Risk 8 で受容済み、doc 明記）
- `process` の alloc-free は `rt_` テスト（反復成功・src 不変）とコードレビューで担保 — 実行時のアロケータ観測は `unsafe` 禁止のため非対象（global allocator 差替は `unsafe impl` が必要）
- 行借用の `ok_or` は構造上不敗だが graceful に `Err` 化（panic-free 契約維持）

## Follow-ups

- T3b（[implement-rgb-to-yuv-and-f16-kernels.md](../../tasks/implement-rgb-to-yuv-and-f16-kernels.md)）: `RgbToYuv`（係数は構築引数 — plan Q2 既定、slot→enum の逆写像を `coef.rs` に追加する余地あり）と `Rgba8 ↔ RgbaF16` カーネル
- T4（[sync-docs-issue-26.md](../../tasks/sync-docs-issue-26.md)）: baseline 採否表・`AGENTS.md`・`README.md` の同期と総合検証
