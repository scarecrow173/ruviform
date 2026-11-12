---
id: "SPEC-QDhe8xTiwyoX9UufhbVtoS"
type: "spec"
status: "approved"
title: "Issue #582 ci 3 lane 失敗(gpui-verify 冷キャッシュ / libc pin 喪失 / moxcms 0.9.2 drift)を修正する"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/582"
    - "https://github.com/scarecrow173/ruviform/actions/runs/43270011255"
  changes:
    added:
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
      - type: "file"
        path: "docs/impl/ir/issue-582-ci-lane-failures.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
  implements: []
  implemented-by:
    - "docs/plans/issue-582-ci-lane-failures-plan.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-582-ci-lane-failures-design.md"
    - "docs/impl/ir/gt-05-ci-offscreen-lanes-slice.md"
    - "docs/specs/issue-125-icc-lut-fixture-coverage.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-582-ci-lane-failures.md"
  references:
    - "https://github.com/scarecrow173/ruviform/pull/572"
  defers: []
  deferred-by: []
---

# Issue #582 ci 3 lane 失敗(gpui-verify 冷キャッシュ / libc pin 喪失 / moxcms 0.9.2 drift)を修正する

## Problem

2026-11-12T17:29Z の `ci` workflow run([run 43270011255](https://github.com/scarecrow173/ruviform/actions/runs/43270011255)、sha `246e364e`)で 3 系統の lane が失敗した。直前の全レーン倒壊(run 43140254535、mise `mr_boxington` 起因)は Issue #583/#584 で解消済みであり、本 Issue は別原因の残存失敗を扱う。

1. **ui-gpui-verify (ubuntu-latest / windows-latest)**: `mise run check-ui-driver-boundary` が内部で実行する `cargo metadata --offline` が、lane 固有キャッシュ冷時に `failed to download 'aho-corasick v1.1.5'` で exit 2/1。`consumer-verify.yml` が同一実障害(run 42348131128)を文書化し `mise exec -- cargo fetch --locked` を guard 前に置く対処を済ませているが、`ci.yml` の ui-gpui-verify には同型の前処理がなかった。
2. **ui-gpui-offscreen-verify**: `cargo build` が `error[E0425]: cannot find value 'ENOATTR' in crate '::libc'`(xattr 0.2.3 経由)で失敗。gt-05 で `cargo update -p libc --precise 0.2.189` により pin されていた `libc` が、その後の lockfile 再生成で 0.2.191 に drift していた(`libc::ENOATTR` は libc 0.2.190 で Linux から削除済み)。
3. **icc-verify**: `mise run test-video-proc-icc` が `icc::tests::icc_transform_missing_intent_lut_tag_rejects` で 1 件失敗(194 passed / 1 failed)。`moxcms = "0.9"` の spec に対し lock が 0.9.2 に drift(0.9.1→0.9.2 で intent 未指定時の perceptual LUT への fallback が追加された)。

libc / moxcms の drift は同一 commit `4bcf1402`(PR #572、Issue #313 S2)の lockfile 再生成で同時に発生している。

## Diagnosis(変更前に診断する)

- **gpui-verify**: guard(`gui-boundary-guard --offline`)は `cargo metadata --offline` を実行し、lockfile 全依存がローカル registry に存在することを前提とする。lane cache(actions/cache の `~/.cargo/registry/{index,cache}` + `git/db`)が冷えていると、consumer clone 側の部分 fetch では workspace 全 dev-dep をカバーできず offline metadata が欠落 crate で失敗する。consumer-verify.yml 472-481 行が同根因をコメント済み。
- **offscreen-verify**: xattr 0.2.3(gpui の `zed-async-tar` 依存経由)が `libc::ENOATTR` を参照するが、libc 0.2.190 で同定数が Linux cfg から削除された。gt-05 IR(`docs/impl/ir/gt-05-ci-offscreen-lanes-slice.md`)が 0.2.189 pin を記録しているが、`4bcf1402` の `Cargo.lock` 再生成で pin が潰れ 0.2.191 になっていた。
- **icc-verify**: moxcms 0.9.2 の `ColorProfile::get_device_to_pcs`/`get_pcs_to_device` に ICC.1:2022 §8.10.2 c) の fallback(`lut.or(self.lut_a_to_b_perceptual.as_ref())`)が追加され、intent 専用 LUT タグ欠落時に perceptual(tag 0)テーブルへ暗黙 fallback するようになった。0.9.1 は intent タグ欠落で `None` → transform 生成失敗 → ruviform が `InvalidValue("icc transform")` に写像していた。Issue #125 spec AC3 が「未提供 intent タグは reject」を契約として固定しており、0.9.2 の fallback はこの契約を迂回する。encoder 側の `write_lut_entry` は 0.9.1/0.9.2 で同一であり、`fix_mft1_tags` の fixture workaround は 0.9.2 でも有効(再生成時も 4 byte 補正が必要)。

## Requirements

1. ui-gpui-verify 両脚で `check-ui-driver-boundary` 前に lockfile 全依存を取得し、冷キャッシュでも `cargo metadata --offline` が成功すること。
2. `Cargo.lock` の `libc` を 0.2.189 に pin し直し、xattr 0.2.3 が Linux でコンパイルできること。
3. moxcms 0.9.2 下で Issue #125 AC3 の契約(missing intent LUT tag → `InvalidValue("icc transform")`)を維持すること。pin で止める場合はその判断根拠と 0.9.2 適応の別 Issue 候補を記録する。
4. 対象ブランチの `ci` workflow run で ui-gpui-verify ×2 / ui-gpui-offscreen-verify / icc-verify が全て success になり、新規失敗 job を作らないこと。
5. lane 削除・skip、guard 検査の緩和、テストの検証意図を損なう書き換えを行わないこと。
6. 最新 main を取り込み、doc-chain と管理 index を再生成してコミットすること。

## Acceptance Criteria

- AC1: `ci.yml` の ui-gpui-verify に `mise exec -- cargo fetch --locked` 前処理が追加され、ブランチ run で両脚 job が success。
- AC2: `Cargo.lock` の `libc` が 0.2.189 で、`4bcf1402` で 0.2.191 に drift した経緯が PR に記載され、ブランチ run の ui-gpui-offscreen-verify が success。
- AC3: `mise run test-video-proc-icc` がローカルで success(196+ tests、missing intent tag reject を含む)し、ブランチ run の icc-verify が success。0.9.1→0.9.2 の実差分に基づく選択根拠が design/PR に記載される。
- AC4: `gh workflow run ci.yml --ref bugfix/582-ci-lane-failures` の run で対象 4 job が success(実行 URL を PR 本文に記載)。
- AC5: `git diff` で lane 削除・guard 緩和・テスト意図の弱体化がないこと。

## Non-goals

- `consumer-verify.yml`(#294 管轄)や ci-issue-tracker 系ワークフローの管理ロジック変更。
- xattr / gpui / moxcms の upstream 修正や version 要件の恒久的 pin 仕様化(今回は lockfile pin + 境界側契約維持)。
- 他 lane(verify / rt-verify / compression-verify 等)の挙動変更。
- 製品コードの ICC 変換意味論の変更(IccIntent の公開 API・variant 集合は据え置き)。
