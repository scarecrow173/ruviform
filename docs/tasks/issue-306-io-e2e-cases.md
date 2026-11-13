---
id: "TASK-pL5wE8rT1yU4iO7pA2sD6f"
type: "task"
status: "done"
title: "Issue #306 OS-L1 既存 CI 証拠検査・不足 Case 補完 (Slice V+T)"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/306"
  changes:
    added:
      - type: "file"
        path: "evidence/rf306/ci-os-l1-report.md"
    modified:
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/staged_lifecycle.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-306-io-e2e-cases.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-306-io-e2e-cases.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-306-io-e2e-cases.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-306-io-e2e-cases.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #306 OS-L1 既存 CI 証拠検査・不足 Case 補完 (Slice V+T)

## 概要

Issue #306 の OS-L1 証拠 slice。対象 SHA
`e93c438adc4af5b498e10926fe859ac1338ac390` の Full CI run
`43428172820` の生ログを検査して io 系 crate の実走証拠を
`evidence/rf306/ci-os-l1-report.md` に記録し、
`ruviform-io-e2e` の staged publish に不足していた
外部干渉・recovery retry の L1 Case を 3 件補完した。

## 実施内容

- [x] V-1〜V-3: run/job 取得・生ログ集計・evidence report
      作成（`evidence/rf306/ci-os-l1-report.md`）。
- [x] T-1: `staged_lifecycle.rs` に 3 Case 追加
      （post-seal temp rewrite / delete / recovery cleanup
      failure→retry）。io-e2e 77 → 80 tests。
- [x] T-2: `mise run fmt-check` / `check` / `lint` / `test
      ruviform-io-e2e` — 全て exit 0、80 tests Pass。
- [x] README 検証節の更新。
- [x] doc チェーン 6 文書 + managed index 再生成。
- [x] `review-protocol` レビュー（blocker なし）。

## 検証結果

| コマンド | 結果 |
| --- | --- |
| `mise run fmt-check` | Passed (exit 0) |
| `mise run check ruviform-io-e2e` | Passed (exit 0) |
| `mise run lint ruviform-io-e2e` | Passed (exit 0, -D warnings) |
| `mise run test ruviform-io-e2e` | Passed (80 tests / 0 failed / 0 ignored, Windows 11) |

統合受入は merge 後の対象 SHA の CI 証拠（Coordinator 確認）。
ローカル検証は変更範囲の検証。

## 対象外（記録のみ）

- G07 public fault seam の実装/公開判定（別 slice）。
- Indeterminate/TempState::Unknown の公開 API 再現
  （Unsupported — mock 正本が canonical）。
- 実 4 GiB・実機 macOS/Linux・Network/Cloud FS・Windows
  L2/L3 実 FS 特殊条件・#294/#288 cross 集約（Deferred）。
