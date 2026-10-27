---
id: "TASK-034N3OJyIb0GPezXrvD4MM"
type: "task"
status: "done"
title: "ICC LUT fixture coverage（intent pair + mft1/mft2/mAB）"
created: "2026-10-27"
updated: "2026-10-27"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-video-proc/tests/fixture/lut_intent_pair.icc"
      - type: "file"
        path: "crates/ruviform-video-proc/tests/fixture/lut_mft1.icc"
      - type: "file"
        path: "crates/ruviform-video-proc/tests/fixture/lut_mft2.icc"
      - type: "file"
        path: "crates/ruviform-video-proc/tests/fixture/lut_mab.icc"
      - type: "file"
        path: "crates/ruviform-video-proc/tests/fixture/lut_a2b0_only.icc"
    modified:
      - type: "file"
        path: "crates/ruviform-video-proc/src/icc.rs"
      - type: "file"
        path: "docs/test-specs/icc-transform-contract-tests.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-125-icc-lut-fixture-coverage.md"
  implemented-by:
    - "docs/impl/ir/issue-125-icc-lut-fixture-coverage-implementation.md"
  depends-on:
    - "docs/plans/implement-issue-125-icc-lut-fixture-coverage.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "icc-transform-feature.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/icc-transform-contract-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/125"
  defers: []
  deferred-by: []
---

# ICC LUT fixture coverage（intent pair + mft1/mft2/mAB）

## Work

approved plan `implement-issue-125-icc-lut-fixture-coverage.md` の T1–T3 を 1 task の ordered substeps として実行 — fixture 合成器拡張 → fixture 生成 → テスト追加 → test-spec 同期 + gate。

**先行確認**: 合成 `ColorProfile`（`ColorProfile::default()` + pub field 設定 + `lut_*` tag 装着）が pin 版 moxcms 0.9.1 で `encode()` → `new_from_slice` round-trip し、`create_transform_f32` が LUT 経路を選択することを `generate_fixtures` の再生成実行で実確認する（fake success なし — 読み戻し検証を生成器に組込む）。

### S1 — fixture 合成器 + `generate_fixtures` 拡張

- [x] `icc.rs` `mod tests` に builder 追加: `lut_rgb_profile(desc, a2b0, a2b1, b2a0, b2a1) -> Vec<u8>`（`color_space=Rgb`・`pcs=Xyz`・`class=DisplayDevice`・D50 `white_point`/`media_white_point`・description → `encode()`）・`mft_lut`/`mab_lut`（2³ CLUT・`x·4+y·2+z` index）・`clut2` 生成 helper — 実装では mft1/mft2 を単一 `mft_lut(map, LutType)` に統合
- [x] `generate_fixtures` を拡張 — 既存 3 built-in に加えて 5 合成 fixture を書き出し、生成直後に `new_from_slice` で読み戻し検証（生成器が自ら受理確認する構造）
- [x] fixture 生成実行: `mise exec -- cargo test -p ruviform-video-proc --features icc generate_fixtures -- --ignored`
- [x] 各 fixture の写像内容（plan File Map どおり）: `lut_intent_pair` = mft2（A2B0=×0.5/A2B1=id・B2A0=×0.5/B2A1=id）、`lut_mft1` = mft1（両 intent R↔B swap・256 entry）、`lut_mft2` = mft2（A2B=×0.5・B2A=×0.75・2 entry）、`lut_mab` = mAB+mBA（linear a/b/m curves + 非単位 matrix + rotation CLUT）、`lut_a2b0_only` = A2B0 mft2 ×0.5 単タグ

### S2 — テスト追加（`icc.rs` inline tests・`include_bytes!` 消費）

- [x] fixture const 5 件追加 + `lut_fixtures_parse_with_intended_tag_shapes` — `new_from_slice` で `color_space == Rgb` と `LutWarehouse` variant/`LutType`（`Lut`/`Lut8`・`Lut16`・`Multidimensional`）を断言
- [x] `icc_transform_intent_selects_distinct_a2b_luts` — `lut_intent_pair → srgb` Perceptual/RelCol の `Rgba8` 出力が一致しない + 各 intent 出力が同 intent raw executor に bit-exact
- [x] `icc_transform_intent_selects_distinct_b2a_luts` — `srgb → lut_intent_pair` で同型断言（B2A 選択差）
- [x] `icc_transform_mft1_lut_path_swaps_channels` / `icc_transform_mft2_lut_path_scales_channels` / `icc_transform_mab_lut_path_executes_clut` — 各 tag 型で `lut → srgb`（A2B）と `srgb → lut`（B2A）が raw executor oracle に bit-exact 一致 + 写像行使の行為断言
- [x] `icc_transform_missing_intent_lut_tag_rejects` — `lut_a2b0_only → srgb` が Perceptual 受理・RelCol `InvalidValue("icc transform")` + `srgb → lut_a2b0_only` 両 intent で拒否
- [x] `rt_process_icc_lut_path_is_deterministic_and_pure` — LUT 経路 `process` の反復決定性・`src` 非破壊（既存 `rt_process_icc_*` パターン）
- [x] `icc_transform_intents_build_and_run` の「follow-up」コメントを本実装への参照へ更新

### S3 — test-spec 同期 + gate

- [x] `docs/test-specs/icc-transform-contract-tests.md` に新 coverage 規則を追記（Rules/Examples）+ `updated` 更新
- [x] gate 全実行（下記 Verification）

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-125-icc-lut-fixture-coverage.md](../plans/implement-issue-125-icc-lut-fixture-coverage.md) — `PLAN-034N3OJ2PTwjfEeCLuuvmQ`（approved）。実装手順の正本
- [../specs/issue-125-icc-lut-fixture-coverage.md](../specs/issue-125-icc-lut-fixture-coverage.md) — `SPEC-034N3OBemoWwFEpqzWlf7r`。受け入れ基準の正本
- [../test-specs/icc-transform-contract-tests.md](../test-specs/icc-transform-contract-tests.md) — 検証契約の正本（本 task で追記）
- [icc-transform-feature.md](icc-transform-feature.md) — Issue #99 Task 4（`IccTransform`/`IccIntent` 実装・fixture 再生成規約の元 task）

## Dependencies

- 前提: `implement-issue-99-color-conversion-kernels.md`（completed — `IccTransform` API・fixture 規約）
- S2 は S1 の fixture 生成に依存（`include_bytes!` が生成済みバイト列を消費）
- 外部クレート依存・`unsafe` なし（moxcms は feature `icc` 配下の既存 optional dep）

## Affected Files

- `crates/ruviform-video-proc/src/icc.rs` — `#[cfg(test)] mod tests` のみ（builder・`generate_fixtures`・const・新テスト・コメント更新）
- `crates/ruviform-video-proc/tests/fixture/lut_{intent_pair,mft1,mft2,mab,a2b0_only}.icc` — 新規生成（`.gitattributes` `*.icc binary` 適用済み確認）
- `docs/test-specs/icc-transform-contract-tests.md` — coverage 規則追記

## Done When

- [x] Perceptual/RelCol で bake した `IccTransform` が intent 区別 fixture で `Rgba8` 出力を変えるテストが src/dst 両方向で存在し pass
- [x] mft1/mft2/mAB 各 tag 型で A2B・B2A 両方向 transform が raw executor oracle に bit-exact 一致し、tag 構造が engine-observable に検証される
- [x] A2B0-only profile が RelCol で `"icc transform"` 拒否・Perceptual で受理される
- [x] `generate_fixtures` が全 fixture（既存+新規）を再生成し、再生成後の `test-video-proc-icc` が green

## Verification

- [x] `mise run fmt-check` — exit 0
- [x] `mise run check` / `lint` / `test` / `test-rt` — exit 0（feature-off ベース経路無退行）
- [x] `mise run test-video-proc-icc` — exit 0（新テスト全 pass・`generate_fixtures` は `--ignored` で非選択）
- [x] `mise run lint-video-proc-icc` — exit 0
- [x] `mise exec -- cargo test -p ruviform-video-proc --features icc generate_fixtures -- --ignored` → 再生成後 `test-video-proc-icc` 再実行 green
- [x] `git check-attr binary -- 'crates/ruviform-video-proc/tests/fixture/lut_mft1.icc'` で `binary: set`
- [x] `node` doc-status audit clean
