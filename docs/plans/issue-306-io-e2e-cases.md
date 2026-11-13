---
id: "PLAN-dS6gH1jK4lM9nB2vC8xZ3q"
type: "plan"
status: "approved"
title: "Issue #306 IO E2E — OS-L1 既存 CI 証拠検査と不足 Case 補完 実施計画"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/306"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-306-io-e2e-cases.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/designs/issue-306-io-e2e-cases.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-306-io-e2e-cases.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-306-io-composition-e2e.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-306-io-e2e-cases.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-306-io-e2e-cases.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #306 IO E2E — OS-L1 既存 CI 証拠検査と不足 Case 補完 実施計画

## Scope Split

単一 slice（Slice V+T）。V（証拠検査）と T（Case 補完）は
独立しているが同一 PR で完結させる — どちらも Issue #306
の同一受入条件群の一部であり、分割すると証拠と Case の
対応関係が追いにくくなるため。

## File Map

| ファイル | 変更 |
| --- | --- |
| `evidence/rf306/ci-os-l1-report.md` | 新規 — CI 証拠・Case 補完・判定材料・NotRun の正本 |
| `crates/ruviform-io-e2e/tests/staged_lifecycle.rs` | 変更 — 3 Case 追加・module doc 更新 |
| `crates/ruviform-io-e2e/README.md` | 変更 — test 数・Case 一覧の更新 |
| `docs/{specs,designs,plans,tasks,test-specs,impl/ir}/issue-306-io-e2e-cases.md` | 新規 — 本 doc チェーン |
| `docs/{specs,designs,plans,tasks,test-specs,impl/ir}/README.md` | 再生成 — managed index |

変更禁止領域（再掲）: `ruviform-io-e2e` 以外の crate、
`.github/`、`mise.toml`、`ci/`、`POLICIES.md`、
`evidence/rf288/`、その他 `docs/`。

## Dependency Graph

```text
spec → design → plan → task → test-spec
         └──────────→ impl/ir（実装記録）
Slice V（証拠抽出）─ 独立 ─┐
Slice T（test 追加）──────┴→ 検証（mise 4 task）→ index 再生成 → review
```

- V は `gh` CLI と run `43428172820` の完了に依存。
- T は `ruviform-io-file` の公開 API（`temp_path()`・
  `take_recovery_handle()`・`RecoveryHandle::cleanup()`）
  のみに依存し、production 変更を必要としない。

## Task Breakdown

1. **V-1**: run/job メタデータ取得（`gh run view`・
   `gh api jobs`）。— 完了
2. **V-2**: io 系 job の生ログ取得と crate/binary 単位の
   test 集計（補助 parser 使用・section 対応付け）。— 完了
3. **V-3**: `ci-os-l1-report.md` 作成（feature lane・
   MSRV・RT・NotRun を含む）。— 完了
4. **T-1**: `staged_lifecycle.rs` に 3 Case 追加。— 完了
5. **T-2**: `mise run fmt-check`/`check`/`lint`/`test
   ruviform-io-e2e` 実行。— 完了（80 tests Pass）
6. **D-1**: doc チェーン 6 文書作成。— 本 plan と同時
7. **D-2**: `doc-maintenance index plan/apply` で
   specs/designs/plans/tasks/test-specs/impl/ir の index
   再生成 + `doc-status lint` 確認。
8. **R-1**: `review-protocol` レビュー → blocker 解消。
9. **G-1**: `git fetch origin main && git merge
   origin/main` → 競合時は生成物は再生成で解消。
10. **C-1**: Conventional Commits で論理単位 commit →
    push → `gh pr create`（日本語本文）→ worker_done。

## Risk Register

| Risk | 対応 |
| --- | --- |
| CI run が未完了で証拠不足 | 同 SHA 最新 completed run を採用。今回は dispatch run が success で確定 |
| ログ集計の誤対応（doctest/複数 cargo 呼出し） | section 対応付け・対象 crate 値の安定性確認・生ログ URL を証跡に記録 |
| tamper test が OS 差で不安定 | 実 FS 操作のみ・rename 先行で Windows delete-pending 回避・3 OS CI で最終確認 |
| docs index 競合（他 worker） | merge 後に `index apply` で再生成（手マージ禁止） |
| 範囲外変更の混入 | `git status` で編集範囲を最終確認 |

## Verification Matrix

| 検証 | コマンド | 判定 |
| --- | --- | --- |
| format | `mise run fmt-check` | exit 0 |
| compile | `mise run check ruviform-io-e2e` | exit 0 |
| lint | `mise run lint ruviform-io-e2e` | exit 0（-D warnings） |
| test | `mise run test ruviform-io-e2e` | 80 tests / 0 failed |
| docs | `doc-maintenance index apply` + `doc-status lint` | index in-sync・finding なし |
| review | `review-protocol` | blocker 0 |

`*-full`/`verify-full`/`*-msrv` はタスク指定によりローカル
必須検証に含めない（Daily/手動 Full CI・msrv lane に委譲）。

## Rollback

- test 追加・evidence・doc のみの変更で production 影響
  なし。revert はコミット単位で可能。
