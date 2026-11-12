---
id: "IMPL-IVBg1yR3uF3Zq1zXqfrIYj"
type: "impl"
status: "draft"
title: "Issue #582 ci lane failures implementation record"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/582"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-582-ci-lane-failures.md"
      - type: "file"
        path: "docs/specs/issue-582-ci-lane-failures.md"
      - type: "file"
        path: "docs/designs/issue-582-ci-lane-failures-design.md"
      - type: "file"
        path: "docs/plans/issue-582-ci-lane-failures-plan.md"
      - type: "file"
        path: "docs/tasks/issue-582-ci-lane-failures.md"
      - type: "file"
        path: "docs/test-specs/issue-582-ci-lane-failures.md"
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "crates/ruviform-video-proc/src/icc.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-582-ci-lane-failures.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/gt-05-ci-offscreen-lanes-slice.md"
    - "docs/specs/issue-125-icc-lut-fixture-coverage.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/issue-582-ci-lane-failures.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/actions/runs/43270011255"
    - "https://github.com/scarecrow173/ruviform/pull/572"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #582 ci lane failures implementation record

## Summary

run 43270011255(sha `246e364e`)の 3 系統 ci 失敗を修正した。(1) `ui-gpui-verify` に `mise exec -- cargo fetch --locked` step を追加し、冷キャッシュ時の `cargo metadata --offline` 失敗を解消。(2) `Cargo.lock` の `libc` を 0.2.189 に pin 復元し xattr 0.2.3 の `ENOATTR` 参照を回復。(3) moxcms 0.9.2 の ICC.1:2022 §8.10.2 c) perceptual fallback に対し、`IccTransform::new` が Issue #125 AC3 の missing-intent-tag reject を境界で再強制。(4) dispatch run 43318635475 で顕在化した既存破損 — `BACKEND_AVAILABLE = cfg!(windows)` により Linux では `declare_slot` が常に `Unsupported` を返すのに `rvf_view_native_slot_*` 5 test が成功を仮定 — を `#[cfg(target_os = "windows")]` で gate(coordinator 承認の範囲拡張)。

## Changes

### `.github/workflows/ci.yml`

- `ui-gpui-verify` の「Restore cargo registry/git index cache」直後に `Fetch locked cargo dependencies` step(`run: mise exec -- cargo fetch --locked`)を追加。matrix(ubuntu/windows)両脚を 1 箇所でカバー。consumer-verify.yml 472-481 行と同型の対処。

### `Cargo.lock`

- `libc` 0.2.191 → 0.2.189(`cargo update -p libc --precise 0.2.189`)。version + checksum のみの最小 diff。0.2.191 への drift は `4bcf1402`(PR #572)の lockfile 再生成で発生 — gt-05 の 0.2.189 pin が同 commit で潰れていた。moxcms 0.9.1→0.2 drift も同 commit。

### `crates/ruviform-video-proc/src/icc.rs`

- `IccIntent` に `lacks_a_to_b_tag`/`lacks_b_to_a_tag` を追加 — profile が方向別 LUT tag を持つのに要求 intent の tag を欠く場合 `true`。`rendering_intent()` と同じ exhaustive match で将来 variant 追加時に compile error → 意図的決定を強制。
- `IccTransform::new` で RGB 判定後・executor bake 前に `intent.lacks_a_to_b_tag(&src) || intent.lacks_b_to_a_tag(&dst)` → `InvalidValue("icc transform")`。LUT tag を全く持たない profile は engine に委譲(engine の `has_*_lut` gate と同条件)。
- `Errors` doc に missing intent tag の reject を追記。
- test `icc_transform_missing_intent_b2a_tag_rejects` を追加 — in-memory `lut_rgb_profile` で B2A0-only profile を生成し dst-side 対称契約を固定(committed fixture 不増)。既存 `icc_transform_missing_intent_lut_tag_rejects` は無変更で pass。

### `crates/ruviform-ui-gpui/src/render.rs`

- `rvf_view_native_slot_*` 5 test と同 test のみが使う helper `tabs_with_native_slot`/`declare_and_bind` に `#[cfg(target_os = "windows")]` を付与。native slot の container backend は Windows のみ存在(`surface::BACKEND_AVAILABLE = cfg!(target_os = "windows")`)し、非 Windows では `declare_slot` が常に `SlotError::Unsupported` を返すため、成功を仮定するこれらの test は Linux で決定論的に失敗する既存破損だった。#566 で `ui-gpui-verify` lane が追加されて以降、guard/libc の先行失敗に隠れて未顕在化。lane・guard・検証条件は変更せず、backend 非依存の test は全て Linux で継続実行される。

## Decisions During Implementation

- **moxcms pin ではなく 0.9.2 適応を選択**: 0.9.2 の fallback は ICC.1:2022 §8.10.2 c) の規格準拠実装であり engine 退行ではない。pin は lockfile 再生成で失われやすく(libc pin 喪失が同一 failure mode)、`IccIntent` の missing-tag reject は `IccTransform::new` が所有する境界の不変条件として再強制するのが責務的に正しい。
- **encoder workaround は維持**: `write_lut_entry` は 0.9.1/0.9.2 で同一のため `fix_mft1_tags` は 0.9.2 でも必要・有効。committed `.icc` fixture は変更なし。
- **fetch step は ci.yml 側**: mise task に入れず consumer-verify と同型の CI step とした — guard task は `--offline` 検査であり、全依存の fetch 前提を CI が明示する方が責務が明確。
- **dst-side test は in-memory 生成**: committed fixture を増やさず `lut_rgb_profile` helper で B2A0-only profile を構築。
- **native_slot test は cfg gate(coordinator 承認の範囲拡張)**: 当初の編集範囲外だった `crates/ruviform-ui-gpui/` への変更は、同一 lane の green 化に必要な既存破損の修正として `orca orchestration ask` で拡張許可を取得。test を `Unsupported` 期待に書き換える代わりに platform gate を選んだ — これらの test が検証するのは Windows container backend 上の slot 投影挙動であり、非 Windows での honest `Unsupported` は別 test(`slot_clip_reports_only_real_restrictions` 等)が既に担保している。

## Verification

- `mise run test-video-proc-icc` → exit 0(196 passed / 0 failed / 1 ignored + doctest 24 passed)。`icc_transform_missing_intent_lut_tag_rejects`・`icc_transform_missing_intent_b2a_tag_rejects` とも pass。
- `mise run lint-video-proc-icc` → exit 0(`-D warnings`)。
- `mise run check-ui-driver-boundary` → verdict: pass(dep-graph/signature/allowlist)。
- `mise exec -- cargo fetch --locked` → exit 0。
- `mise run test-ui-gpui`(Windows) → exit 0(77 lib tests + 統合 test 全 pass。gate 対象 5 test は Windows で継続実行・pass)。
- `cargo fmt -p ruviform-ui-gpui -- --check` → exit 0。
- CI: `gh workflow run ci.yml --ref bugfix/582-ci-lane-failures` の結果は PR 本文に記載。

## Related Experiments

### Adopted

- moxcms 0.9.2 + 境界 check — spec 準拠 engine を維持しつつ ruviform 契約を保持。

### Rejected

- `cargo update -p moxcms --precise 0.9.1` pin — lockfile 再生成で容易に失われる(libc pin 喪失と同型)。0.9.2 適応を別 Issue に先送りするより、境界で契約を再強制する方が恒久的。
- xattr 側の更新待ち — upstream の ENOATTR 依存解消が未定であり、libc pin が最小修正。
