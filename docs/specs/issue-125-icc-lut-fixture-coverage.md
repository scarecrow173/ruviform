---
id: "SPEC-034N3OBemoWwFEpqzWlf7r"
type: "spec"
status: "implemented"
title: "Issue #125 ICC LUT fixture coverage（intent 区別 pair + mft1/mft2/mAB 経路）"
created: "2026-10-27"
updated: "2026-10-27"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/125"
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
  implements: []
  implemented-by:
    - "../plans/implement-issue-125-icc-lut-fixture-coverage.md"
  depends-on:
    - "color-conversion-kernels.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../test-specs/icc-transform-contract-tests.md"
    - "../impl/ir/issue-99-icc-transform-task-4-implementation.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/color-conversion-kernels.md"
  derived-by:
    - "../plans/implement-issue-125-icc-lut-fixture-coverage.md"
  verifies: []
  verified-by:
    - "../test-specs/icc-transform-contract-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/99"
  defers: []
  deferred-by: []
---

# Issue #125 ICC LUT fixture coverage（intent 区別 pair + mft1/mft2/mAB 経路）

## What Should Be Built

Issue #99 Task 4（`docs/impl/ir/issue-99-icc-transform-task-4-implementation.md`）の独立レビューで「後続 Issue 管轄」として記録された 2 件の coverage 拡張を、`ruviform-video-proc` の feature `icc` テスト群へ追加する。

- **intent-distinguishing LUT fixture pair** — A2B0（Perceptual）と A2B1（RelativeColorimetric）、および B2A0/B2A1 が異なる写像を持つ合成 ICC fixture を作り、`IccIntent` が executor の bake に使う LUT タグ選択を変え、観測可能に出力が変わることを実証する。現行 `icc_transform_intents_build_and_run` は matrix-shaper sRGB fixture しか持たず、両 intent が同一バイトを返すこと＝焼付けの受理/決定性しか検証していない（テスト内コメントにも「NOT that the intent changes the mapping」と記録済み）。
- **`lut` feature 経路 fixture** — moxcms の `lut` feature が有効化している mft1（lut8Type）/mft2（lut16Type）/mAB（`LutMultidimensionalType`）タグを持つ合成 ICC fixture を追加し、A2B（device→PCS）・B2A（PCS→device）両方向で LUT 変換パイプラインを行使する。現行 3 fixture（`srgb.icc`・`display_p3.icc`・`gray_gamma22.icc`）は全て matrix-shaper profile で、`create_lut3x3`/`prepare_mab_3x3`/`prepare_mba_3x3` 経路は未行使。

## Why Now

- Issue #99 reviewer A の P2 ×2 が「intent が mapping を変える実証なし」「LUT 変換パス未行使」の 2 点を指名し、task-4 IR の Follow-ups 節に後続 Issue として記録・起票済み（Issue #125）。契約の穴を放置すると、`IccIntent` 写像の誤配線や `lut` feature 依存コードの退行を現行テストは検出できない。
- `Cargo.toml` は `lut` feature を理由付きで有効化済み（mft1/mft2/mAB profile 対応のため）— 有効化した feature の経路を fixture が行使しない状態は「依存の正当化」と「検証」の不整合。

## Users and Value

- **レビュアー**: #99 で棚上げされた coverage gap が実テストとして閉じ、レビュー指摘の追跡が完結する。
- **将来の `IccTransform` メンテナ**: intent → engine `RenderingIntent` 写像の退行（例: 両 variant を同じ intent に写してしまう誤り）や、moxcms バージョン更新時の LUT パイプライン退行を、CI `icc-verify` lane で検出できる。
- **fixture 再生成の実施者**: `#[ignore]`d `generate_fixtures` が全 fixture の正本生成手順であり続け、新規 LUT fixture の合成パラメータがコードとして残る。

## Scope

### In Scope

- `crates/ruviform-video-proc/tests/fixture/` への合成 ICC fixture 追加 — 手作り `ColorProfile`（`lut_a_to_b_*`/`lut_b_to_a_*` タグに `LutWarehouse::Lut`/`Multidimensional`）を pin 版 moxcms `encode()` で出力する方式（既存 fixture と同一の再生成経路）
- `generate_fixtures`（`#[ignore]`d 再生成 test）の拡張 — 全 fixture の one-shot 再生成 recipe として維持
- `icc.rs` inline tests へのテスト追加 — intent 区別（src 方向・dst 方向の 2 系統）・mft1/mft2/mAB の A2B/B2A 両方向・fixture のタグ構造検証（engine-observable）・未提供 intent タグの拒否挙動・LUT 経路の RT 反復テスト
- `docs/test-specs/icc-transform-contract-tests.md` への新 coverage 規則の追記（契約の正本として同期）

### Out of Scope

- `IccTransform`/`IccIntent` 公開 API・`TransformOptions` 契約値・依存 feature 集合の変更（feature `icc` の定義は据え置き）
- intent 拡張集合（Saturation/AbsoluteColorimetric）・黒点補償の API 露出 — #99 plan non-goal 据え置き
- golden vector の新規 pin 追加（literal golden は既存の x86_64 非 FMA baseline 規約のまま増やさない — 新テストは oracle 等価性 + 行為断言で検証し、全 build 構成で動く形に統一）
- moxcms 側の LUT 実装正しさの検証（エンジン内部契約 — 本 spec が検証するのは ruviform 側の fixture/テスト配線）
- `any_to_any`/`options`/SIMD 系 moxcms feature の有効化（must-stay-off 据え置き）

## Requirements

- 追加 fixture は pin 版 moxcms（0.9.1・`default-features = false` + `lut` + `extended_range`）で `encode()` し `new_from_slice` が受理するバイト列であり、`include_bytes!` でテストが消費する。再生成は `cargo test -p ruviform-video-proc --features icc generate_fixtures -- --ignored` 一発で全 fixture を書き直せる
- intent 区別 fixture は A2B0 ≠ A2B1 かつ B2A0 ≠ B2A1 の写像差を持ち、`IccIntent::Perceptual`/`RelativeColorimetric` で bake した executor の出力が `Rgba8` 量子化レベルで区別できる差（last-ulp 差ではない観測可能な差）を生じる
- LUT fixture は `DataColorSpace::Rgb` + `pcs = Xyz` の合成 profile で、mft1（`LutStore::Store8`）・mft2（`Store16`）・mAB（`LutMultidimensionalType`・a/m/b curves + CLUT）を網羅し、A2B・B2A 両方向で executor 構築に成功する
- 新テストは全 build 構成で動く（literal golden を `cfg` ゲートせず、同一 build 構成の raw executor との bit-exact 比較 + 行為断言で構成 — 既存 `icc_transform_rgba8_output_matches_engine_bit_exact` の oracle パターン踏襲）
- `.gitattributes` の `*.icc binary` 規約を新 fixture へも適用（既存行でカバー済みか確認）

## Acceptance Criteria

- [x] **AC1**: `IccIntent::Perceptual` と `RelativeColorimetric` で bake した `IccTransform` が、intent 区別 fixture に対して `Rgba8` 出力で異なるバイト列を返すテストが src 方向（A2B 選択差）・dst 方向（B2A 選択差）の両方で存在し、`mise run test-video-proc-icc` で pass する
- [x] **AC2**: mft1/mft2/mAB の各 LUT tag 型について、fixture を parse した時点で期待する `LutWarehouse` variant/`LutType` が engine から観測でき、A2B・B2A 両方向の transform が同一 build 構成の raw executor 出力に bit-exact 一致するテストが `mise run test-video-proc-icc` で pass する
- [x] **AC3**: 未提供の intent タグを持つ profile（A2B0 のみ）が `RelativeColorimetric` で `InvalidValue("icc transform")` を返し、Perceptual では受理されるテストが存在する（intent が LUT タグ選択を実際に制御している裏付け）
- [x] **AC4**: `generate_fixtures` が全 fixture（既存 3 + 新規）を再生成し、再生成後に `mise run test-video-proc-icc` が green のままであることを実コマンドで確認済み（IR に実 log 記録）
- [x] **AC5**: `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` / `test-video-proc-icc` / `lint-video-proc-icc` が全て exit 0（feature-off ベース経路無退行を含む）

## Deferred Design Concerns

| 論点 | 本 spec での扱い | 移管先 | 無視した場合のリスク |
| --- | --- | --- | --- |
| Saturation/AbsoluteColorimetric intent・黒点補償 API | fixture 追加なし（`IccIntent` 集合据え置き） | #99 plan non-goal 継続 | intent 拡張時に対応 fixture が追従しない |
| moxcms LUT 内部処理の正しさ（trilinear/mAB stage 順序等） | エンジン契約として委託 — ruviform 側は oracle 等価性のみ検証 | moxcms upstream | エンジン bug は pin 版の動作として fixture/golden に取り込まれる |
| 実世界 ICC profile（OS 同梱・カメラ由来）での検証 | 合成 fixture のみ（依存なし・決定的） | 将来の media-io/アプリ層 Issue | 実 profile のみが示す形式差（v2 タグ・異常 CLUT）の取りこぼし |

## Sources and Relations

- 一次ソース: GitHub Issue [#125 [impl] Issue #99 フォローアップ: ICC 変換の LUT fixture coverage](https://github.com/scarecrow173/ruviform/issues/125)（`relations.source`）。Issue #99 reviewer 指摘の coverage 拡張 2 件と検証 lane を規定。
- 上流文書（`relations.derives-from`）: [color-conversion-kernels design](../designs/color-conversion-kernels.md)（`IccTransform`/`IccIntent` 契約と moxcms 依存/feature 選択の正本）。
- 前提（`relations.depends-on`）: [color-conversion-kernels spec](color-conversion-kernels.md)（#99 spec・implemented）。変更対象の契約面を確定した先行 spec。
- 関連（`relations.related`）: [icc-transform-contract-tests test-spec](../test-specs/icc-transform-contract-tests.md)（本 spec が規則を追加する契約正本）、[task-4 IR](../impl/ir/issue-99-icc-transform-task-4-implementation.md)（P2 follow-up の記録元）。
- 派生（`relations.derived-by`/`implemented-by`）: [実装 plan](../plans/implement-issue-125-icc-lut-fixture-coverage.md)。
- 参考（`relations.references`）: [Issue #99](https://github.com/scarecrow173/ruviform/issues/99)（親 Issue）。
