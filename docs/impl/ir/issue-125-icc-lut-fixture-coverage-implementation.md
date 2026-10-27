---
id: "IMPL-034N3SIoHoHOf6TdLjhDna"
type: "impl"
status: "completed"
title: "Issue #125 ICC LUT fixture coverage implementation"
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
      - type: "file"
        path: "docs/specs/issue-125-icc-lut-fixture-coverage.md"
      - type: "file"
        path: "docs/plans/implement-issue-125-icc-lut-fixture-coverage.md"
      - type: "file"
        path: "docs/tasks/icc-lut-fixture-coverage.md"
      - type: "file"
        path: "docs/impl/ir/issue-125-icc-lut-fixture-coverage-implementation.md"
    modified:
      - type: "file"
        path: "crates/ruviform-video-proc/src/icc.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/tests/fixture/srgb.icc"
      - type: "file"
        path: "crates/ruviform-video-proc/tests/fixture/display_p3.icc"
      - type: "file"
        path: "crates/ruviform-video-proc/tests/fixture/gray_gamma22.icc"
      - type: "file"
        path: "docs/test-specs/icc-transform-contract-tests.md"
      - type: "file"
        path: "docs/designs/color-conversion-kernels.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/icc-lut-fixture-coverage.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #125 ICC LUT fixture coverage implementation

## Summary

Issue #125 完了 — Issue #99 Task 4 独立レビューの P2 ×2（intent 非区別・`lut` feature 経路未行使）を閉じる 5 合成 LUT fixture + 8 テストを `icc.rs` `#[cfg(test)] mod tests` に追加。公開 API・依存・feature 定義は不変（src 非テストコード・`Cargo.toml`・`mise.toml`・CI 定義に diff なし）。`generate_fixtures` を全 8 fixture の正本再生成器へ拡張し、生成器自身が `new_from_slice` 読戻しで受理確認する構造を維持。moxcms 0.9.1 encoder の `lut8Type` 非対称 bug（`lut16Type` 専用 4-byte entry-count を mft1 に混入）を発見し、generator 内局所 workaround（`fix_mft1_tags`）でコミット済み fixture byte を spec 準拠に修正。4 gate + icc lane 2 + `test-rt` + 再生成列 + doc audit 全 green 実 log。

## Context

approved plan `docs/plans/implement-issue-125-icc-lut-fixture-coverage.md`・spec `docs/specs/issue-125-icc-lut-fixture-coverage.md`・task `docs/tasks/icc-lut-fixture-coverage.md`（T1–T3 = S1–S3 ordered substeps）。upstream は Issue #99 Task 4 IR（`docs/impl/ir/issue-99-icc-transform-task-4-implementation.md` — reviewer A P2 指摘の記録元）と test-spec `icc-transform-contract-tests.md`（契約正本 — 本実装で LUT coverage 規則を追記）。前提となる moxcms 0.9.1 実検証事実（`LutWarehouse` variant → mft1/mft2/mAB エンコード・intent→tag 固定マップ・欠落 tag は `UnsupportedProfileConnection` → `"icc transform"`）は plan「Design Inputs」節に記録済みで、実装で全て実測確認した。

## Implementation

### Changes

- `crates/ruviform-video-proc/src/icc.rs`（`#[cfg(test)] mod tests` のみ — src 公開/非テストコードは無変更）:
  - fixture const 5 件（`include_bytes!` — L439–443）+ 各 fixture の写像意味を doc コメントで明記（L423–438）。
  - 合成 profile builder 群（L454–649）: `lut_u8`/`lut_u16`（`Store8`/`Store16` 量子化 — `round(v·255)`/`round(v·65535)`、clamp + round で cast が lossless であることを SAFETY 式 `#[allow(clippy::cast_*)]` コメントで明記）・`clut2`（2×2×2 trilinear CLUT — engine `Cube` の `x·4+y·2+z` node 順・3ch interleave）・`mft_lut`（`LutDataType` 組立 — mft1 は format 固定の 256-entry `Store8` in/out table、mft2 は最小 2-entry `Store16` linear ramp — `has_same_kind` 制約で全 Store 同一型）・`mab_lut`（`LutMultidimensionalType` — a/b/m 全 linear `ToneReprCurve::Lut(vec![])` + `grid_points[..3]=[2,2,2]` `Store16` CLUT + 非単位 `Matrix3d`）・`lut_rgb_profile`（`ColorProfile::default()` + `DisplayDevice`/`Rgb`/`Xyz`/D50 white・per-intent `Option<LutWarehouse>` 装着 → `encode()` → `fix_mft1_tags`）。
  - `fix_mft1_tags`（L580–616）— **moxcms 0.9.1 encoder の既知非対称 workaround**: `write_lut_entry` が `lut8Type` に `lut16Type` 専用の u16×2 entry-count 4 byte を混入し、spec 準拠 reader（moxcms 自身を含む）が table 開始を 4 byte 早く誤読する。ICC layout（128-byte header + tag count + 12-byte directory）を直接走査し、各 `mft1` body を `body[..48] ++ body[52..]` に圧縮 → directory offset/size・header total size を再配置。committed fixture byte が正しい CMM の出力と一致するよう generator に閉じ込めた（`lut_mft1.icc` は 6724 → 6708 byte）。
  - 写像 map 関数: `halve3`（×0.5 — intent pair Perceptual / mft2 A2B / a2b0_only）・`scale34_3`（×0.75 — mft2 B2A）・`ident3`（intent pair colorimetric）・`swap_rb3`（R↔B — mft1）・`rotate3`（`(x,y,z)→(z,x,y)` rotation — mAB CLUT）・`mab_matrix`（非単位 M-stage matrix — identity+zero-bias+全 linear m-curves の skip 条件を外して M stage を行使）。
  - `generate_fixtures` 拡張（L656–729）— `[(&str, Vec<u8>); 8]` で built-in 3 + 合成 5 を一括再生成。各 fixture を `new_from_slice` で読戻し（`lut_` prefix は `DataColorSpace::Rgb` 断言 — `gray_gamma22` は Gray なので合成 LUT のみに限定）。再生成コマンドは従来どおり `--ignored` 明示実行。
  - oracle 基盤: `run_rgba8`（kernel `process` → `dst` byte 列）・`oracle_rgba8`（同一 profile/options/intent の raw `create_transform_f32` executor + 同一 `quantize8`）・`LUT_PIXELS`（全 LUT fixture で byte-observable な mid-gamut 4 pixel）。
  - 新テスト 8 件:
    - `lut_fixtures_parse_with_intended_tag_shapes`（L1393）— 構造検証: mft1=`Lut`/`Lut8`/`Store8`/256-entry/2³ CLUT・mft2=`Lut16`/`Store16`/2-entry・mAB/mBA=`Multidimensional`/2³ `Store16` CLUT/3 b-curves・intent pair の A2B0≠A2B1・B2A0≠B2A1・a2b0_only は A2B0 単タグ。
    - `icc_transform_intent_selects_distinct_a2b_luts`（L1489）— `lut_intent_pair → srgb`: Perceptual(×0.5 PCS) と RelCol(identity PCS) の `Rgba8` 出力が `assert_ne` + channel-wise `<=` & 画素毎 strict 差（clamp 飽和画素を許容）+ 各 intent が同 intent raw executor に bit-exact。**`IccIntent` が実写像を変えることの A2B 側証明**。
    - `icc_transform_intent_selects_distinct_b2a_luts`（L1533）— `srgb → lut_intent_pair` の鏡像（B2A0=×0.5 / B2A1=identity — 全 channel strict `<` が成立）。
    - `icc_transform_mft1_lut_path_swaps_channels`（L1573）— mft1 A2B/B2A 両方向 oracle bit-exact + 純赤 → 青優位の swap 行為断言 + `lut_mft1 → lut_mft1`（swap∘swap≈identity ±2）往復。
    - `icc_transform_mft2_lut_path_scales_channels`（L1617）— mft2 両方向 oracle bit-exact + `lut_mft2 → lut_mft2` で A2B×0.5 ∘ B2A×0.75 ≈ ×0.375（±3）。
    - `icc_transform_mab_lut_path_executes_clut`（L1648）— mAB/mBA 両方向 oracle bit-exact + 純緑 → 青優位（rotation CLUT）・`srgb→srgb` baseline は緑のまま（matrix-shaper との区別）。
    - `icc_transform_missing_intent_lut_tag_rejects`（L1684）— `lut_a2b0_only → srgb` Perceptual 受理・RelCol `InvalidValue("icc transform")`（A2B1 欠落）・`srgb → lut_a2b0_only` 両 intent 同拒否（B2A 全欠落）— engine の intent→tag 選択が効く裏付け。
    - `rt_process_icc_lut_path_is_deterministic_and_pure`（L1707）— mAB kernel で 64 回 `process` 反復 byte-identical + `src` 非破壊（既存 `rt_process_icc_*` パターン）。
  - `icc_transform_intents_build_and_run`（L1151）コメント更新 — matrix-shaper sRGB での両 intent 同一出力は「焼付け受理/決定性の証明」であり「intent が写像を変える証明ではない」旨を明記し、実写像差の証明は上記 intent pair テストへ参照。
- `crates/ruviform-video-proc/tests/fixture/`（新規 5 + 再生成 3）:
  - `lut_intent_pair.icc`（786 B）— mft2 ×4 tag。A2B0/B2A0=×0.5・A2B1/B2A1=identity。
  - `lut_mft1.icc`（6708 B）— mft1/lut8Type ×4 tag（全 intent/direction R↔B swap・256-entry `Store8` table + 2³ CLUT）。`fix_mft1_tags` 適用後の spec 準拠バイト列。
  - `lut_mft2.icc`（772 B）— mft2/lut16Type ×4 tag（A2B=×0.5・B2A=×0.75・2-entry `Store16`）。
  - `lut_mab.icc`（1298 B）— mAB+mBA `LutMultidimensionalType` ×4 tag（linear a/b/m + 非単位 matrix + rotation CLUT）。
  - `lut_a2b0_only.icc`（374 B）— A2B0 mft2 ×0.5 単タグのみ（A2B1/B2A0/B2A1 全欠落）。
  - built-in 3 fixture（`srgb.icc` 612 B・`display_p3.icc` 598 B・`gray_gamma22.icc` 312 B）は generator が header 内 creation datetime のみを更新して再書込み（同サイズ・payload 等価 — `cmp` で diff byte が datetime 6-field のみと実確認）。
- `docs/test-specs/icc-transform-contract-tests.md`: Rules に intent 区別 fixture・LUT tag 型網羅・欠落 tag 拒否・再生成規約（`fix_mft1_tags` workaround 明記）の 4 規則を追記・Examples に 3 例追加・header status note を Issue #125 拡張済みに更新・`updated` = 2026-10-27。
- `docs/designs/color-conversion-kernels.md`: `derived-by` に本 spec/plan の 2 行を追加（doc graph reciprocal 整合 — audit warning 解消）。
- doc index: `docs/specs/README.md`・`docs/plans/README.md`・`docs/tasks/README.md`（生成時に自動追加された行の status を最終値へ更新）+ `docs/impl/ir/README.md`（本 IR の行を手動追加）。

### Decisions During Implementation

- **mft1 encoder bug の fixture 側局所修正**（deviation・deliberate）— moxcms 0.9.1 `encode()` の `lut8Type` 出力が spec 非準拠（余分な 4 byte）で、エンジン自身の spec 準拠 reader が自製出力を誤読する（table が 4 byte ずれて CLUT 値が化ける）。upstream 修正は依存 bump を伴うため scope 外と裁定し、`fix_mft1_tags` を `lut_rgb_profile` の encode 後段に局所封じ込め — committed fixture byte は正しい CMM が書く byte と等しい。workaround の存在・理由・対象 version は関数 doc + test-spec の再生成規約に明記（pin 版 bump 時に除去判断を再評価）。
- **写像差は全て「大きい」線形写像** — intent 区別・tag 型別の観測差を `Rgba8` 量子化後に確実に残すため、×0.5/×0.75 scale・R↔B swap・channel rotation を選択（plan Risk Register どおり）。2³ CLUT + linear in/out table は trilinear で map を忠実再現し store 量子化のみの誤差に留まる。
- **Perceptual-only BPC 副次効果は区別根拠にしない** — plan 記録どおり Perceptual 時のみ `detect_black_point` 補償が掛かるが、intent 差の主根拠は LUT 内容差（×0.5 vs identity）。BPC 依存の差は engine 内部契約なので oracle 等価性の比較対象には使わない。
- **mft1/mft2 builder の統合** — plan/task は `mft1_lut`/`mft2_lut` 別 helper を想定していたが、差分は `LutType` + table entry 数のみなので単一 `mft_lut(map, lut_type)` に集約（DRY — task S1 に「実装では統合」と記録）。
- **構造検証と行為検証の分離** — `lut_fixtures_parse_with_intended_tag_shapes` は engine-observable な parse 結果（`LutWarehouse` variant・`LutType`・entry 数・Store 型・CLUT grid）のみを断言し、写像の意味は各 transform テストの行為断言に残す — tag が意図どおり encode されたこと（構造）と executor が意図どおり走ること（行為）を別 assertion で固定。
- **clamp 飽和を許容する intent 差断言** — A2B 方向は ×0.5 PCS vs identity で channel-wise `<=` が理論保証だが、飽和 channel は両者 255 に clamp され得るため per-channel strict `<` ではなく `<=` + 画素単位 `p[..3] != c[..3]` で「観測可能な差」を固定（`[255,0,134]` vs `[255,0,185]` のような clamp 等値を偽失敗させない）。B2A 方向は device 空間差が全 channel strict になる実測を確認し `<` を維持。
- **literal golden 不増** — spec out-of-scope どおり新規 golden vector は追加せず、全 build 構成で動く oracle bit-exact + 行為断言に統一（x86_64 非 FMA baseline pin は既存 4 件のみ維持）。

## Related Experiments

### Adopted

なし — approved plan の確定実装のみ（実験ログなし）。

### Rejected

なし — mft1 encoder bug の対処方針（upstream 修正 vs fixture 側 workaround）は実装中に評価し、依存 bump を伴う upstream 修正は scope 外と裁定（experiment 文書化までは至らない裁定 — Decisions 節に記録）。

## Validation

- gate（実 log・全 exit 0）:
  - `mise run fmt-check` — exit 0（`cargo fmt --all -- --check`）
  - `mise run check` — exit 0（`cargo check --workspace --all-targets` — feature-off ベース）
  - `mise run lint` — exit 0（`cargo clippy --workspace --all-targets -- -D warnings` clean）
  - `mise run test` — exit 0（`cargo test --workspace` — feature-off ベース無退行）
  - `mise run test-rt` — exit 0（`cargo test --workspace rt_` — `rt_process_icc_lut_path_is_deterministic_and_pure` は feature-on レーン側で選択 — test-spec 規約どおり）
  - `mise run test-video-proc-icc` — exit 0（feature-on — lib tests **195 passed / 0 failed / 1 ignored**（ignored = `generate_fixtures`）・doctests **24/24** — 新規 8 テスト全 pass を実確認）
  - `mise run lint-video-proc-icc` — exit 0（feature-on clippy clean — 初回実行で doc_markdown ×4 + cast_possible_truncation ×3 を検出し修正 → 再実行 green。feature-off `lint` では icc test 配下がコンパイル対象外のため本 lane が必須）
  - `mise exec -- cargo test -p ruviform-video-proc --features icc generate_fixtures -- --ignored` — exit 0（全 8 fixture 再生成 + 読戻し検証 → 再実行後 `test-video-proc-icc` 再 green — spec AC4）
- 構造確認（実コマンド）:
  - `git check-attr binary -- 'crates/ruviform-video-proc/tests/fixture/lut_*.icc'` — 新規 5 fixture 全て `binary: set`（`.gitattributes` の `*.icc binary` が網羅 — spec 要求どおり確認済み）
  - `cmp` diff（`srgb.icc` HEAD vs 再生成）— diff byte は header creation datetime のみ（同サイズ 612 B）
  - `ColorProfile::new_from_slice` 読戻し — 全 8 fixture 受理・合成 5 は `DataColorSpace::Rgb`（generator 内断言）
- doc audit: `node .agents/skills/doc-status/scripts/doc_status.js audit --json` — blocking 0。初回検出の warning 4 件（design `derived-by` 未連携 ×2・test-spec `verifies` 未連携 ×1・task index status 不整合 ×1）は reciprocal relation 追記 + index 更新で全解消済み（再実行 clean — findings 0）。

## Non-Goals

- `IccTransform`/`IccIntent` 公開 API・`TransformOptions` 契約値・`Cargo.toml`/feature 定義・`mise.toml`/CI 定義の変更 — plan 境界どおり src 非テストコードに diff なし。
- intent 拡張集合（Saturation/AbsoluteColorimetric）・黒点補償 API — #99 plan non-goal 継続。
- moxcms 側 `lut8Type` encoder bug の upstream 修正 — 依存 bump を伴うため scope 外（`fix_mft1_tags` に局所封じ込め・pin 版 bump 時の除去判断は test-spec に記録）。
- 実世界 ICC profile（OS 同梱・カメラ由来）での検証 — spec Deferred どおり合成 fixture のみ（将来の media-io/アプリ層 Issue）。
- literal golden vector の新規 pin — spec out-of-scope（全テスト oracle 等価性 + 行為断言で全 build 構成動作）。

## Risks

- `fix_mft1_tags` は moxcms 0.9.1 encoder の観測された出力レイアウトに pin — 将来の moxcms 更新で encoder が修正された場合、workaround が spec 準拠出力を壊さないか要確認（body[..48] 抽出が冪等なのは現行バグ形のみ）。関数 doc に前提明記・pin bump 時は再生成 + `lut_fixtures_parse_with_intended_tag_shapes` が検出。
- 合成 fixture は CLUT 2³ の最小 grid — trilinear 補間の角だけを行使しており、密 grid・非線形 curve・v2 タグなど実 profile 形状の網羅ではない（spec Deferred — エンジン内部網羅は moxcms 側責務）。
- intent pair の Perceptual 差には engine BPC 補償の副次効果が含まれ得る — LUT 内容差が主根拠なので影響なしだが、moxcms bump で BPC 判定が変わると出力 byte が変わり得る（oracle 比較なので検出可能・再生成規約どおり）。
- 新テストは全て `#[cfg(test)]` 内 — feature `icc` off の `cargo test --workspace` は icc 配下を選択しない構造的事実は変わらず、`test-video-proc-icc` lane / CI `icc-verify` が唯一の regress 検出経路（test-spec 規約どおり既存構造を維持）。

## Follow-ups

- moxcms version bump 時: `fix_mft1_tags` の要否再評価（upstream `lut8Type` encoder が修正されていれば除去）+ `generate_fixtures` 再実行 — 手順は test-spec 再生成規約に記録済み。
- Issue #99 Task 4 IR の Follow-ups 節に記録された P2 ×2（intent-distinguishing LUT fixture pair・`lut` feature 経路 fixture coverage）は本実装で解消 — 残る #99 系 follow-up はなし（Task 6/7 は先行完了済み）。
- `assert_no_alloc` による `process` alloc 機械検出は #97/#99 共通の後続 Issue 管轄で据え置き（本実装の `rt_process_icc_lut_path_is_deterministic_and_pure` も repeatable/deterministic + src 非破壊の観測検証に留まる）。
