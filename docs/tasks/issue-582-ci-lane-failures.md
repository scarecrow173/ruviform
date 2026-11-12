---
id: "TASK-vioQrXcHCVk7TkuPqAIPKc"
type: "task"
status: "done"
title: "Issue #582 T1 — ci 3 lane 失敗の修正・doc-chain・workflow_dispatch 検証"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/582"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-582-ci-lane-failures.md"
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
    - "docs/test-specs/issue-582-ci-lane-failures.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-582-ci-lane-failures-plan.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-582-ci-lane-failures.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #582 T1 — ci 3 lane 失敗の修正・doc-chain・workflow_dispatch 検証

## Objective

run 43270011255 の 3 系統失敗を修正し、対象ブランチの `ci` workflow run で ui-gpui-verify ×2 / ui-gpui-offscreen-verify / icc-verify が success になることを確認する。

## Checklist

- [x] 失敗 run(43270011255)の 3 失敗を原因確定(gpui-verify 冷キャッシュ metadata / libc 0.2.191 ENOATTR / moxcms 0.9.2 intent fallback)
- [x] `Cargo.lock` の libc を `cargo update -p libc --precise 0.2.189` で pin 復元(drift 経緯 `4bcf1402` を確認)
- [x] moxcms 0.9.1→0.9.2 差分確認(fallback 追加・encoder 不変)に基づき `icc.rs` 境界 check + dst-side test を実装
- [x] `ci.yml` ui-gpui-verify に `mise exec -- cargo fetch --locked` step 追加
- [x] `mise run test-video-proc-icc` / `lint-video-proc-icc` / `check-ui-driver-boundary` / `cargo fetch --locked` がローカルで成功
- [x] doc-chain(spec/design/plan/task/test-spec + IR)作成
- [x] dispatch run で顕在化した既存破損を修正: `rvf_view_native_slot_*` 5 test + 専用 helper を `#[cfg(target_os = "windows")]` で gate(`BACKEND_AVAILABLE = cfg!(windows)` 由来。coordinator 承認の範囲拡張で `crates/ruviform-ui-gpui/` を編集)
- [x] `git merge origin/main` → 触った docs dir の管理 index を doc-maintenance で再生成
- [x] commit → push → PR(`Refs #582`)
- [ ] `gh workflow run ci.yml --ref bugfix/582-ci-lanes-failures` で対象 4 job success を確認し URL を PR に記載
