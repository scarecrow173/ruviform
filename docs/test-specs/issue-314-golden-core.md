---
id: "TSPEC-e1fQqoSXueHMyLvDmFV5Gz"
type: "test-spec"
status: "proposed"
title: "Issue #314 slice 1 — Golden Utility コアのテスト仕様"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/314"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-314-golden-core.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/tests/golden.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/golden/key.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/golden/image.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/golden/diff.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/golden/store.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/golden/report.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-314-golden-core.md"
    - "docs/designs/issue-314-golden-core.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-314-golden-core.md"
  verifies:
    - "docs/specs/issue-314-golden-core.md"
    - "docs/designs/issue-314-golden-core.md"
---

# Issue #314 slice 1 — Golden Utility コアのテスト仕様

全て合成画像・tempdir による headless 検証。実描画・実 OS・GPU・
フォント環境には依存しない。

## Feature: GoldenKey / EnvironmentMeta（key.rs）

### Rule: key の全要素が同一性に効く

- Example: story/theme/state/driver/os/gpu/scale/font/revision のいずれか
  1 要素だけを変えた 2 つの `GoldenKey` は `canonical()`・`file_stem()`
  が異なる。同一値の key は serde round-trip 後も一致する。
- 検証: `golden_key_*` unit tests。

### Rule: file_stem は fs 安全で決定的

- Example: 空白・`/`・非 ASCII を含むフィールド値が `_` に正規化され、
  `scale_factor = 2.0` が `"2.0"` として現れる（`{:?}` format）。
- 検証: `golden_key_file_stem_is_fs_safe`。

### Rule: 未取得の環境値は None のまま

- Example: `gpu_class`/`font_class`/`os_version` 等の `Option` が
  `None` で serialize され、捏造値が入らない。
- 検証: serde round-trip の unit test + `skip_serializing_if` 確認。

## Feature: RgbaImage / codec（image.rs）

### Rule: PNG round-trip は RGBA8 を保存する

- Example: 既知の RGBA8 画素列 → `to_png` → `from_png` で画素・次元が
  一致。RGB 入力 PNG は alpha=255 で RGBA8 に正規化される。
- 検証: `golden_image_png_round_trip`・`golden_image_png_rgb_expands`。

### Rule: Screenshot wire 型を decode できる

- Example: `format="png"` は base64 PNG を decode、`format="rgba-raw"` は
  base64 生 RGBA（width/height は wire フィールド）を decode。
  未知 format・壊れた base64・次元不整合は `GoldenError`。
- 検証: `golden_image_from_screenshot_*` unit tests。

### Rule: 異常入力を拒否する

- Example: `pixels.len() != w*h*4`、0 次元、上限超過の PNG 次元
  （`MAX_DIMENSION`/`MAX_PIXELS`）はエラー。
- 検証: `golden_image_rejects_*` unit tests。

## Feature: diff（diff.rs）

### Rule: per-channel 許容差の境界

- Example: tolerance `(2,0,0,0)` で R 差 2 は一致・差 3 は差分。
  G/B/A は差 1 でも差分。全 channel 0 の tolerance は厳密一致。
- 検証: `golden_diff_tolerance_boundary`。

### Rule: mask 領域は比較対象外

- Example: 差分を含む rect を mask すると `diff_pixels=0`・
  `masked_pixels` に計上され `Pass`。mask は画像端で clip される。
- 検証: `golden_diff_mask_excludes_region`・`golden_diff_mask_clips`。

### Rule: alpha=0 の両側画素は一致扱い

- Example: expected/actual とも alpha=0 で RGB が異なっても差分に
  ならない。片側のみ alpha=0 なら alpha channel 差分として評価。
- 検証: `golden_diff_transparent_pixels`。

### Rule: 統計と差分画像が正しい

- Example: 既知の差分配置から `diff_pixels`/`compared_pixels`/
  `masked_pixels`/channel 別 `max_delta`/bbox が計算され、差分画像は
  差分画素=赤・masked=減光・その他=actual を保持する。
- 検証: `golden_diff_stats_and_image`。

## Feature: BaselineStore / compare（store.rs）

### Rule: layout と sidecar 検証

- Example: `write_candidate` が `<story>/<theme>/<state>/<stem>.png|.json`
  を生成し、`promote` で baseline path へ配置される。sidecar の key が
  要求 key と異なる baseline は `GoldenError::Store` で拒否される。
- 検証: `golden_store_round_trip`・`golden_store_rejects_key_mismatch`（tempdir）。

### Rule: verdict 写像が正直

- Example: baseline 一致→`Pass`、許容差超過→`Fail`、サイズ不一致→
  `Fail`(reason=`size_mismatch`)、baseline 不在→`Blocked`
  (reason=`baseline_missing`)、全面 mask→`Blocked`
  (reason=`empty_comparison`)。
- 検証: `golden_compare_*` tests（`tests/golden.rs`）。

### Rule: 自動更新経路がない

- Example: `UPDATE_*` 系 env が存在せず、`compare` は baseline を
  一切書き換えない（baseline mtime/内容が比較前後で不変）。
- 検証: `golden_compare_does_not_mutate_baseline`。

## Feature: evidence（report.rs）

### Rule: JSONL evidence が schema どおり append される

- Example: `RUVIFORM_GOLDEN_EVIDENCE` 指定時に `rvf.golden.evidence/v1`
  行（key/environment/verdict/reason/stats/baseline/artifacts）が
  追記され、未指定時はファイル副作用がない。
- 検証: `golden_evidence_appends_jsonl`（tempdir path を明示指定する
  API で検証 + env 未設定の no-op 確認）。

## Feature: 公開面の境界

### Rule: 公開 signature に toolkit/GPUI 型が現れない

- 検証: `mise run check-ui-driver-boundary`（exit 0）。
