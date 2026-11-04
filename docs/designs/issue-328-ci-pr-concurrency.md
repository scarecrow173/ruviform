---
id: "DESIGN-yMdXfkOVFpXfZGKfVH7LjG"
type: "design"
status: "approved"
title: "Issue #328 CI PR concurrency 設計 — group 解決・cancel 意味論・required checks 保護"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/328"
    - "docs/specs/issue-328-ci-pr-concurrency.md"
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
    - "docs/specs/issue-328-ci-pr-concurrency.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-328-ci-pr-concurrency.md"
    - "docs/adr/issue-328-ci-concurrency-grouping.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-328-ci-pr-concurrency.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-328-ci-pr-concurrency.md"
    - "docs/adr/issue-328-ci-concurrency-grouping.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://docs.github.com/en/actions/how-tos/writing-workflows/choosing-what-your-workflow-does/control-the-concurrency-of-workflows-and-jobs"
  defers: []
  deferred-by: []
---

# Issue #328 CI PR concurrency 設計 — group 解決・cancel 意味論・required checks 保護

## Overview

`ci.yml` / `consumer-impact-check.yml` の workflow トップレベル(`permissions:` 直後・`jobs:` 前)に `concurrency:` block を追加する。job・step・job 名には一切触れない。

```yaml
concurrency:
  # Same-PR runs supersede each other: a new run for the same PR cancels
  # the in-progress/pending run. Non-PR triggers (push to main,
  # workflow_dispatch) get a unique group per run_id and are never
  # cancelled — and different PRs/workflows never share a group.
  group: ${{ github.workflow }}-${{ github.event_name == 'pull_request' && github.ref || github.run_id }}
  cancel-in-progress: ${{ github.event_name == 'pull_request' }}
```

## Group 解決表(event 別)

GitHub 式評価: `&&` は `||` より強く結合するため `(event_name == 'pull_request' && github.ref) || github.run_id` と等価。

| event | `github.event_name` | 式の解決 | 生成 group 例 | 同 group になり得る run | cancel-in-progress |
| --- | --- | --- | --- | --- | --- |
| `pull_request`(opened/synchronize/reopened/ready_for_review) | `pull_request` | `github.ref` = `refs/pull/<N>/merge` | `ci-refs/pull/328/merge` | 同じ PR の run のみ | `true` |
| `push` to main | `push` | `github.run_id` | `ci-19876543210` | なし(run ごと一意) | `false` |
| `workflow_dispatch` | `workflow_dispatch` | `github.run_id` | `ci-19876543211` | なし | `false` |
| `merge_group`(将来・未使用) | `merge_group` | `github.run_id` | `ci-19876543212` | なし | `false` |

- PR group キー `refs/pull/<N>/merge` は PR 番号で一意 → fork PR・別 base・reopen でも別 PR と衝突しない。
- `github.workflow` 接頭辞で workflow 間 isolation(concurrency group はリポジトリ共有空間)。`ci` と `consumer-impact-check` は名前が違うため同式で衝突しない。
- workflow 名・ref・run_id は小文字/数字のみで、group 名の case-insensitive 性の影響なし。

## Cancel 意味論(公式仕様より)

- group 内で同時に running になれるのは 1 run のみ。新規 run が queue されると、既存の pending run は **既定で cancel されて置き換えられる**(`queue: single` 既定)。`cancel-in-progress: true` はさらに running 中の run も cancel する。
- したがって連続 push A→B→C では: A running + B pending → C 到着時に pending の B が cancel・A も cancel → C のみ完走。**最新 revision のみが検証される**形になる。
- 非 PR event は group が run_id 一意のため、cancel-in-progress の有無に関係なく他 run と競合しない(現在と同一の並列性)。

## Required checks / check run 保護

- check run は workflow run の **head SHA** に紐づく check suite として記録される。cancel された run の conclusion は `cancelled` であり、required status check の合格条件(success または skipped)を満たさない。
- ruleset/branch protection の required checks は **PR の最新 head SHA** の check を評価する。旧 SHA の cancelled 結果が新 head の合格証拠になる経路は存在しない。
- `skipped` が合格扱いになる ruleset の既知挙動とも無関係: 本変更は check run を skipped にしない。cancel は run の実行経路ではなく「走らなかった」conclusion。
- auto-merge は最新 head の required checks を再評価するため、旧 run cancel で誤マージは起きない。

## Edge case 検証

| case | 挙動 | 評価 |
| --- | --- | --- |
| 同一 PR 連続 push | 最新 run が pending/running の旧 run を cancel して完走 | 目的挙動。AC3 で実測 |
| 別 PR 同時進行 | group が `refs/pull/N` で別物 | cancel なし(AC4) |
| main へ連続 push | 各 run が一意 group | 従来通り全実行(AC4) |
| workflow_dispatch 連続実行 | 各 run が一意 group | 従来通り全実行(AC4) |
| 古い run の手動 re-run | 同じ run_id・同 group に再入 → **実行中の最新 run を cancel し得る** | 既知 edge。re-run は自分の元 SHA で走るため、最新 head の check は cancelled 方向(=fail-safe・マージ不可)に倒れる。復旧 = 最新 run の再実行 or 再 push。人間の明示操作のみが引き金で、静かに起きるものではないため許容 |
| 同一 SHA での再 trigger(ready_for_review/reopened) | 同 group で最新 run が権威ある check を生成 | 同一 SHA のため新旧で検証対象は同一。害なし |
| fork PR | `github.ref` は upstream 側 `refs/pull/N/merge` | 通常 PR と同一の分離。`contents: read` permissions 不変(concurrency は token 権限を使わない platform 機構) |
| merge queue 将来導入 | `merge_group` は event_name≠pull_request → run_id 一意 | queue 内 run が相互 cancel しない安全側の既定。導入時に group 設計を再検討 |
| `queue` property | 未指定 = `single` | `queue: max` は `cancel-in-progress` と併用不可・本件不要 |

## consumer-impact-check.yml への適用

同一 block を適用する。`edited`/`synchronize` 等で同一 PR・同一/別 SHA の stale 実行が最新実行より後に完了し stale な本文検査結果を「最新」として残す競合を、group cancel で構造的に排除する。必須 check ではないため required-check リスクはなく、実行コストの削減効果は小さいが「最新結果の権威性」という同じ不変条件を守る。

## consumer-verify.yml への非適用

`workflow_dispatch` 専用。同一式を置いても group は run_id 一意で no-op。加えて dispatch は意図的な診断実行であり相互に cancel させる意味がない。変更しない(設計上の判断として記録)。

## AGENTS.md 同期

CI workflow 記述(「ci.yml は 3 trigger・12 required checks」)に「同一 PR の旧 revision run は concurrency group で新 run 開始時に cancel される」旨の一文を追加する。job 構成・trigger・required checks の記述は不変。

## 検証方針

- 静的: `mise exec -- actionlint .github/workflows/ci.yml .github/workflows/consumer-impact-check.yml`、式の全 event 分岐の机上評価(本書の group 解決表)、`git diff` で job 構成非変更を確認。
- 実測(本 PR の CI で観測・coordinator 実施): 同一 PR 連続 2 push → 旧 run cancel/最新 run 完走の Run URL を IR と PR に記録。
- worker 環境では push を伴う実験は行わない(共通規約: push/PR 操作は coordinator 担当)。worker は設定正当性・actionlint・docs を担当。
