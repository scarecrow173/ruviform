---
id: "IR-mfG5TUv6nTb9bSBIpPKVbE"
type: "impl"
status: "completed"
title: "Issue #328 実装記録 — CI PR concurrency(cancel stale PR runs)"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/328"
  changes:
    added: []
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: ".github/workflows/consumer-impact-check.yml"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-328-t1-ci-pr-concurrency.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-328-t1-ci-pr-concurrency.md"
    - "docs/plans/issue-328-ci-pr-concurrency.md"
    - "docs/test-specs/issue-328-ci-pr-concurrency.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-328-ci-pr-concurrency.md"
    - "docs/adr/issue-328-ci-concurrency-grouping.md"
    - "docs/designs/issue-328-ci-pr-concurrency.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/issue-328-t1-ci-pr-concurrency.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-328-ci-pr-concurrency.md"
  verified-by: []
  references:
    - "https://docs.github.com/en/actions/how-tos/writing-workflows/choosing-what-your-workflow-does/control-the-concurrency-of-workflows-and-jobs"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #328 実装記録 — CI PR concurrency(cancel stale PR runs)

## Summary

`.github/workflows/ci.yml` と `.github/workflows/consumer-impact-check.yml` の `permissions:` 直後に Issue #328 推奨形の workflow-level `concurrency:` block を追加した。`pull_request` では group が `${{ github.workflow }}-refs/pull/<N>/merge` で `cancel-in-progress: true`、それ以外の event では `${{ github.workflow }}-<run_id>` で cancel 無効。job 名・trigger・permissions・step の変更はゼロ。`AGENTS.md` の ci.yml 記述に同旨の一文を追加。

## Changes

| file | diff |
| --- | --- |
| `.github/workflows/ci.yml` | `concurrency:` block + 説明コメント 9 行追加(+9) |
| `.github/workflows/consumer-impact-check.yml` | `concurrency:` block + 説明コメント 7 行追加(+7) |
| `AGENTS.md` | ci.yml bullet 内に concurrency 説明 1 文を挿入 |

## Design verification(静的・机上)

test-spec の INV/Cases に対する評価:

- INV-1: `pull_request` の `github.ref` は `refs/pull/<N>/merge`(fork/reopen/別 base でも PR 番号で一意)。group = `<workflow>-refs/pull/<N>/merge`。
- INV-2: `push`/`workflow_dispatch`/`merge_group` は `github.run_id` で run ごと一意。どの run も他 run と group を共有しない。
- INV-3: group 内 pending 置換は既定動作(`queue: single`)、`cancel-in-progress: true` で running run も cancel — GitHub 公式ドキュメントの記述で確認。連続 push A→B→C で完走し得るのは C のみ。
- INV-4: cancel run の check run conclusion は `cancelled` で元 head SHA に紐づく。required checks は最新 head SHA を評価。`cancelled` は合格条件(success/skipped)を満たさない → 偽装経路なし。
- INV-5: `git diff` は両 workflow で `concurrency:` + コメントのみ(job 名・job 数・trigger・permissions・step 非変更)。
- 式の合法性: `concurrency` で許可される expression context は `github`/`inputs`/`vars`/`needs`/`strategy`/`matrix`。`github.workflow`/`github.event_name`/`github.ref`/`github.run_id` は全て `github` context 内。`cancel-in-progress` の条件式は公式ドキュメントに例示あり。
- 既知 edge(ADR 記録): 古い run の手動 re-run は同 group に再入し最新 run を cancel し得る。cancel 方向は fail-safe(古い revision の check が欠ける=merge 不可、誤マージは起きない)。復旧 = 最新 run の再実行 or 再 push。

## Verification(実測、Windows 11 x64 worktree / Git Bash)

| 検証 | コマンド | 結果 |
| --- | --- | --- |
| actionlint(変更 2 ファイル) | `mise exec -- actionlint .github/workflows/ci.yml .github/workflows/consumer-impact-check.yml` | PASS(exit 0・出力なし) |
| actionlint(全 workflow) | `mise exec -- actionlint` | PASS(exit 0) |
| 非破壊 diff | `git diff` | PASS(concurrency + comment のみ) |
| doc chain gate | `route_graph.js --focus docs/specs/issue-328-*.md` で probe→briefing→design→planning→task-graph→implementation(tasks-active) | PASS(全 edge 選択・blocker なし) |
| doc index 再生成 | `doc_maintenance.js apply` | PASS(7 README 再生成・errors なし) |
| doc audit | `doc_status.js audit` | blocking 155→148(全て pre-existing baseline。新規 blocking 0) |

toolchain: mise 2026.10.1 / actionlint 1.7.12。

### NotRun(後続検証が必要)

- **AC3 / test-spec C3(同一 PR 連続 push で旧 run cancel の実測)**: worker 環境では push・PR 操作を行わない規約のため未実施。本 PR の CI で 2 push 実験を行い、旧 run(cancelled)・新 run(completed)両方の Run URL を記録すること — coordinator 側で実施・記録を引き継ぐ。
- **別 PR/main push/dispatch の非 cancel 実測**: 同上。式の静的評価(INV-2)で構造的に保証されるが、実機観測は本 PR の運用で確認推奨。

## Review(review-protocol)

- 客観チェック: actionlint・diff・doc audit・graph gate — 全 PASS(上表)。
- Reviewer A(cooperative): spec R1–R6 の充足を設計・実装・検証表でトレース — 全項充足。
- Reviewer B(adversarial): ref 意味論・trigger types・same-SHA re-trigger・merge queue・cross-workflow 衝突・式 context 合法性・artifact/cache 副作用を攻撃 — 唯一の実質 edge(古い run の手動 re-run → 最新 run cancel)は ADR/design に既記録・fail-safe・復旧手順あり。
- Judge: CONFIRMED P0/P1 なし → **ACCEPT**(非 blocking 観測のみ)。

## Followups

- `consumer-verify.yml` は dispatch-only で本変更なし(設計で no-op 判断を記録)。将来 `pull_request` trigger を持つ workflow を追加する場合は同じ group 式を適用し、`github.workflow` 接頭辞で isolation を維持すること。
- merge queue(`merge_group`)導入時は concurrency group 設計を再検討する(現行式は run_id 一意で安全側だが、queue 効率のため専用 group が望ましい可能性)。
- doc audit の残存警告(spec `traceability-missing-upstream`・test-spec `inconsistent-reciprocal-relation`)は issue-238/312 と同型の慣例レベル — spec の upstream は Issue 本文自身が source であり upstream doc が存在しないため。
