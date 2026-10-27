---
id: "PLAN-034N3OJ2PTwjfEeCLuuvmQ"
type: "plan"
status: "completed"
title: "Issue #125 実装計画 — ICC LUT fixture coverage"
created: "2026-10-27"
updated: "2026-10-27"
owners: []
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
      - type: "file"
        path: "docs/tasks/icc-lut-fixture-coverage.md"
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
    - "docs/specs/issue-125-icc-lut-fixture-coverage.md"
  implemented-by:
    - "docs/tasks/icc-lut-fixture-coverage.md"
  depends-on:
    - "docs/plans/implement-issue-99-color-conversion-kernels.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-125-icc-lut-fixture-coverage.md"
    - "docs/designs/color-conversion-kernels.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/icc-transform-contract-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #125 実装計画 — ICC LUT fixture coverage

## Goal

Issue #99 後続の 2 coverage gap を閉じる — intent 区別 LUT fixture pair で `IccIntent` が写像を変えることを実証し、mft1/mft2/mAB LUT tag を持つ合成 fixture で feature `lut` 配線の LUT 変換パイプライン（A2B/B2A 両方向）を行使する。実装は `icc.rs` test 配下と `tests/fixture/` のみで、公開 API・依存・feature 定義は不変。

## Scope

単一実装ストリーム — fixture 生成器拡張 + fixture 生成 + テスト追加 + test-spec 同期で 1 task。境界: `crates/ruviform-video-proc` の `#[cfg(test)]`/`tests/fixture/` と `docs/test-specs/icc-transform-contract-tests.md` のみ。`src/` 非テストコード・`Cargo.toml`・mise.toml・CI 定義は無変更。

## Upstream Documents

- [../specs/issue-125-icc-lut-fixture-coverage.md](../specs/issue-125-icc-lut-fixture-coverage.md) — `SPEC-034N3OBemoWwFEpqzWlf7r`（proposed→implemented）。スコープ・受け入れ基準の正本
- [GitHub Issue #125](https://github.com/scarecrow173/ruviform/issues/125) — 一次ソース。#99 reviewer 指摘の 2 件を規定
- [../impl/ir/issue-99-icc-transform-task-4-implementation.md](../impl/ir/issue-99-icc-transform-task-4-implementation.md) — follow-up の記録元（reviewer A P2 ×2）

## Design Inputs

- [../designs/color-conversion-kernels.md](../designs/color-conversion-kernels.md) — `IccTransform`/`IccIntent`・moxcms 依存/feature 集合・等価性契約の正本
- [../test-specs/icc-transform-contract-tests.md](../test-specs/icc-transform-contract-tests.md) — 契約テスト規則の正本（本 plan で coverage 規則を追記）
- moxcms 0.9.1（`Cargo.lock` pin・`default-features = false` + `lut` + `extended_range`）の実装検証済み事実:
  - `ColorProfile` は全 pub field の `Default` 構造体 — `lut_a_to_b_perceptual`/`lut_a_to_b_colorimetric`/`lut_b_to_a_perceptual`/`lut_b_to_a_colorimetric` に `Option<LutWarehouse>` を設定して `encode()` 可能
  - `LutWarehouse::Lut(LutDataType)` は `Store8` → mft1・`Store16` → mft2 として書き出される（`write_lut_entry` は `has_same_kind` を要求 — input/clut/output 全 Store が同一型）
  - `LutWarehouse::Multidimensional(LutMultidimensionalType)` は A2B タグでは mAB・B2A タグでは mBA として書き出される（`write_mab_entry`）
  - executor の intent → tag 選択は `get_device_to_pcs`/`get_pcs_to_device` で `Perceptual → A2B0/B2A0`・`RelativeColorimetric|AbsoluteColorimetric → A2B1/B2A1`・`Saturation → A2B2/B2A2` に固定マップ — 要求 intent のタグ未提供は `UnsupportedProfileConnection` → `InvalidValue("icc transform")`（フォールバックなし）
  - RGB→RGB の LUT 経路は `has_device_to_pcs_lut`/`has_pcs_to_device_lut` で選択され `create_lut3x3`（mft）/`prepare_mab_3x3`・`prepare_mba_3x3`（mAB/mBA）を行使 — `is_matrix_shaper` ではない profile では LUT 必須
  - mft1 は in/out table が 256 entry/ch 固定（reader 側が Lut8 を 256 強制）・mft2 は 2..=4096 entry・CLUT grid ≥ 2（trilinear `Cube` は `index = x·g² + y·g + z`・3 ch interleave）
  - mAB は `b_curves` 3 本必須（欠くと `InvalidAtoBLut`）・CLUT は `a_curves` 3 本 + `grid_points[0..3]` ≥ 2 と対で必要・`m_curves` + 非単位 matrix で M stage が走る
  - Perceptual 時のみ `detect_black_point` の BPC 補償が掛かる（`DEFAULT_V4_BLACK_POINT`・確定的）— intent 差の副次効果だが区別根拠には使わず LUT 内容差を主根拠とする

## File Map

- `crates/ruviform-video-proc/src/icc.rs`（`#[cfg(test)] mod tests` のみ変更）— LUT fixture 合成 helper（`lut_rgb_profile`/`mft1_lut`/`mft2_lut`/`mab_lut` + CLUT 生成）・`generate_fixtures` 拡張・新 fixture const・新テスト群
- `crates/ruviform-video-proc/tests/fixture/lut_intent_pair.icc` — mft2 ×4 tag（A2B0=×0.5 scale / A2B1=identity・B2A0=×0.5 / B2A1=identity）— intent 区別 pair
- `crates/ruviform-video-proc/tests/fixture/lut_mft1.icc` — mft1 ×4 tag（A2B0=A2B1=R↔B swap・B2A0=B2A1=R↔B swap・256 entry table）— lut8Type 経路
- `crates/ruviform-video-proc/tests/fixture/lut_mft2.icc` — mft2 ×4 tag（A2B=×0.5・B2A=×0.75 scale・2 entry table）— lut16Type 経路
- `crates/ruviform-video-proc/tests/fixture/lut_mab.icc` — mAB + mBA tag（a/b/m curves=linear + 非単位 matrix + 2³ CLUT channel rotation）— `LutMultidimensionalType` 経路
- `crates/ruviform-video-proc/tests/fixture/lut_a2b0_only.icc` — A2B0（mft2 ×0.5）のみの単タグ profile — 未提供 intent tag の拒否挙動用
- `docs/test-specs/icc-transform-contract-tests.md` — LUT coverage 規則の追記

## Implementation Sequence

### T1: fixture 合成器と `generate_fixtures` 拡張（task: icc-lut-fixture-coverage）

- [x] `mod tests` に合成 profile builder を追加 — `ColorProfile::default()` + pub field 設定（`color_space=Rgb`・`pcs=Xyz`・`class=DisplayDevice`・`white_point`/`media_white_point`=D50・description）→ `encode()`。`LutDataType`/`LutMultidimensionalType`/`LutStore`/`ToneReprCurve` の組立 helper（2³ CLUT は `x·4+y·2+z` index 順）
- [x] `generate_fixtures` を拡張 — 既存 3 built-in encode に加え、5 合成 fixture（intent_pair・mft1・mft2・mab・a2b0_only）を書き出す。`#[ignore]` + doc コメントの再生成手順は維持
- [x] `mise exec -- cargo test -p ruviform-video-proc --features icc generate_fixtures -- --ignored` で fixture を生成・コミット（`.gitattributes` の `*.icc binary` が全 `*.icc` に効くことを確認）

### T2: テスト追加（task: icc-lut-fixture-coverage・T1 と同一 task 内の ordered step）

- [x] fixture const 追加（`include_bytes!`）+ 構造検証テスト — 各 LUT fixture を `new_from_slice` で parse し、`color_space == Rgb` と期待する `LutWarehouse` variant/`LutType`（mft1/mft2/mAB）を engine から観測断言
- [x] intent 区別テスト ×2 — `lut_intent_pair → srgb`（A2B 選択差）と `srgb → lut_intent_pair`（B2A 選択差）で Perceptual/RelCol 各 bake の `Rgba8` 出力が一致しないことを断言 + 各 intent の出力が同 intent の raw executor に bit-exact 一致（oracle パターン踏襲・全 build 構成で動く）
- [x] LUT tag 型別の両方向テスト ×3（mft1・mft2・mAB 各 1）— `lut → srgb`（A2B）と `srgb → lut`（B2A）の transform が raw executor oracle に bit-exact 一致 + 写像が行使された行為断言（swap/scale/rotation の観測差）
- [x] 未提供 intent tag 拒否テスト — `lut_a2b0_only → srgb` が Perceptual で受理・RelCol で `InvalidValue("icc transform")`（engine の intent→tag 選択が実際に効く裏付け）
- [x] `rt_process_icc_lut_repeatable` — LUT 経路 `process` の反復決定性（既存 `rt_process_icc_*` パターン）
- [x] 既存 `icc_transform_intents_build_and_run` のコメント更新 — 「intent-distinguishing LUT fixture pair は follow-up」の記述を本実装への参照に更新

### T3: test-spec 同期 + 検証（plan Verification 節・task 内手順どおり）

- [x] `docs/test-specs/icc-transform-contract-tests.md` に新 coverage 規則を追記（intent 区別 pair・LUT tag 型網羅・再生成規約の更新）+ `updated` 更新
- [x] gate 全実行（下記 Verification）

## Dependencies and Risks

### Dependency Graph

| 依存元 | 依存先 | 内容 |
| --- | --- | --- |
| T2 | T1 | テストは生成済み fixture バイト列を `include_bytes!` で消費 — fixture 生成が先行 |
| T3 | T1・T2 | test-spec 追記と gate は実装確定後 |
| 本 plan | `implement-issue-99-color-conversion-kernels.md`（completed） | `IccTransform` API・fixture 再生成規約・等価性契約の前提 |

### Risk Register

| リスク | 影響 | 確率 | 対応 |
| --- | --- | --- | --- |
| 合成 profile が moxcms の encode→parse で round-trip しない | fixture 生成不能 | 低 | `generate_fixtures` 実行時に `new_from_slice` で読み戻し検証を組込み（生成器自身が受理確認する構造） |
| 合成 LUT の写像が弱く intent/型差が量子化後に消える | AC1/AC2 を満たす区別が出ない | 低 | ×0.5 scale・channel swap/rotation 等の大きい写像差を選択 — `Rgba8` でも確実に分かれる |
| golden 値が target-feature 依存で揺れる | CI 以外の構成で fail | 低 | literal golden を増やさない — raw executor oracle 比較 + 行為断言（規約どおり） |
| moxcms バージョン bump で fixture 意味が変わる | 再生成手順と同梱バイト列が乖離 | 中 | `generate_fixtures` が全 fixture の正本 — bump 時は再生成（既存規約どおり・手順は test-spec に記録） |

## Verification

- [x] `mise run fmt-check` — exit 0
- [x] `mise run check` / `lint` / `test` / `test-rt` — exit 0（feature-off ベース経路無退行）
- [x] `mise run test-video-proc-icc` — exit 0（feature-on・新テスト全 pass）
- [x] `mise run lint-video-proc-icc` — exit 0
- [x] `mise exec -- cargo test -p ruviform-video-proc --features icc generate_fixtures -- --ignored` → 再生成後に `test-video-proc-icc` 再実行 green（fixture 再生成経路の実検証）
- [x] `git check-attr binary -- 'crates/ruviform-video-proc/tests/fixture/*.icc'` で `binary: set` 確認
- [x] doc-status audit（spec/plan/task/test-spec/impl）clean
