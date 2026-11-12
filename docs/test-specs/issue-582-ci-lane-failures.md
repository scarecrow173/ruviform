---
id: "TSPEC-7QVSRiKeJAQ2bbzvImiekm"
type: "test-spec"
status: "approved"
title: "Issue #582 テスト仕様 — ci lane 修復・missing intent tag reject 維持・検証意図の保持"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/582"
    - "docs/specs/issue-582-ci-lane-failures.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-582-ci-lane-failures.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-582-ci-lane-failures-plan.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-582-ci-lane-failures-plan.md"
    - "docs/specs/issue-582-ci-lane-failures.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-582-ci-lane-failures-design.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-582-ci-lane-failures-plan.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-582-ci-lane-failures.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #582 テスト仕様 — ci lane 修復・missing intent tag reject 維持・検証意図の保持

## Purpose

3 系統の ci lane 失敗が修正され、既存の検証意図(offline boundary guard・Linux pixel golden・ICC intent 契約)を弱めずに回復することを保証する。

## Feature

ci lane 修復 — `ui-gpui-verify` の offline metadata 前提、`ui-gpui-offscreen-verify` の Linux compile、`icc-verify` の moxcms 0.9.2 下での Issue #125 AC3 契約。

## Rules

- INV-1(offline metadata 前提): `check-ui-driver-boundary` の `cargo metadata --offline` は lockfile 全依存( dev-dep 含む)が registry に存在することを前提とし、ui-gpui-verify はその前提を `cargo fetch --locked` で明示的に作る。
- INV-2(libc pin): `Cargo.lock` の `libc` は xattr 0.2.3 が要求する `libc::ENOATTR` を持つ版(≤0.2.189 系)に pin され、lockfile 再生成での再 drift を防ぐ記録が残る。
- INV-3(missing intent tag reject): moxcms ≥0.9.2 の perceptual fallback があっても、LUT tag を持つ profile が要求 intent の tag を欠く場合 `IccTransform::new` は `InvalidValue("icc transform")` を返す(Issue #125 AC3)。A2B/B2A 両方向で対称。
- INV-4(LUT 非保有 profile の委譲): A2B/B2A tag を全く持たない profile(matrix-shaper 等)は engine の既存経路(matrix-shaper or hard reject)に委ね、境界 check で過剰 reject しない。
- INV-5(検証意図の維持): lane 削除・skip、guard の検査緩和、既存テストの検証意図を損なう書き換えを行わない。

## Cases

| case | 検証方法 | 期待 |
| --- | --- | --- |
| icc feature test | `mise run test-video-proc-icc` | exit 0。`icc_transform_missing_intent_lut_tag_rejects`(A2B0-only src)と `icc_transform_missing_intent_b2a_tag_rejects`(B2A0-only dst)が pass |
| icc lint | `mise run lint-video-proc-icc` | exit 0(`-D warnings`) |
| boundary guard | `mise run check-ui-driver-boundary` | verdict: pass |
| dep fetch | `mise exec -- cargo fetch --locked` | exit 0 |
| libc pin | `git diff origin/main -- Cargo.lock` | libc 0.2.191→0.2.189 のみ |
| guard 前処理 | `git diff origin/main -- .github/workflows/ci.yml` | ui-gpui-verify に fetch step 追加のみ、lane/guard 不変 |
| 実 CI | `gh workflow run ci.yml --ref bugfix/582-ci-lanes-failures` → `gh run watch` | ui-gpui-verify×2 / offscreen / icc-verify = success |
| index 整合 | `doc_maintenance.js index apply` | exit 0 |

## Notes

- offscreen lane の Linux compile(xattr + libc)はローカル Windows で再現不能のため、CI run の job success をもって検証とする。
- `icc_transform_missing_intent_lut_tag_rejects` は実装側修正により**テスト無変更で pass** する — 検証意図の書き換えは発生していない。
- dst-side test は committed fixture を増やさず `lut_rgb_profile` で in-memory 生成する(既存 fixture 群の byte を変更しない)。
