---
id: "ADR-ieVahO2wRjuTcKd43fT59I"
type: "adr"
status: "accepted"
title: "Issue #328 CI concurrency — group キーと cancel-in-progress の event 条件化"
created: "2026-11-04"
updated: "2026-11-06"
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
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-328-ci-pr-concurrency.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-328-ci-pr-concurrency.md"
  refines: []
  refined-by:
    - "docs/adr/issue-326-ci-yml-dispatch-only.md"
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://docs.github.com/en/actions/how-tos/writing-workflows/choosing-what-your-workflow-does/control-the-concurrency-of-workflows-and-jobs"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# Issue #328 CI concurrency — group キーと cancel-in-progress の event 条件化

## Context and Problem Statement

`ci.yml` に PR 単位の重複実行抑制を入れたい。要求は「同一 PR の旧 run は新 run で cancel」「main push・workflow_dispatch・別 PR は絶対に cancel しない」の両立であり、concurrency group キーの選び方が isolation 境界を構造的に決める。GitHub Actions の concurrency group は **リポジトリ全体で共有** され workflow をまたいで衝突する点、group 名は case-insensitive な点、pending run の置換は `cancel-in-progress` なしでも既定で起きる点が公式仕様。

## Decision Drivers

- 別 PR・main push・手動 dispatch の巻き添え cancel は許容できない — group キーが「PR ごと一意・非 PR は run ごと一意」を満たす必要がある。
- concurrency group はリポジトリ共有空間のため、複数 workflow が同じ式を使う場合は `github.workflow` 接頭辞が必須(公式ドキュメントも推奨)。
- `pull_request` では `github.ref` が `refs/pull/<N>/merge` で PR 番号ごと一意。fork PR・reopen・synchronize でも不変。
- `cancel-in-progress` を非 PR で有効にする必要がない(一意 group では vacuous)が、条件式で明示した方が意図が読み取れる。
- 追加依存・追加 permission は避けたい(concurrency は platform 側の scheduling 機構で `contents: read` のままで動く)。

## Considered Options

- **Option A(採用)**: `group: ${{ github.workflow }}-${{ github.event_name == 'pull_request' && github.ref || github.run_id }}`、`cancel-in-progress: ${{ github.event_name == 'pull_request' }}`。Issue #328 本文の推奨形。PR → `ci-refs/pull/N/merge`、非 PR → `ci-<run_id>`(run ごと一意)。
- **Option B**: GitHub 公式 canonical 形 `group: ${{ github.head_ref || github.run_id }}`、`cancel-in-progress: true`。**却下** — `github.head_ref` は PR の head ブランチ名であり、同一ブランチから複数 PR(別 base 向け・close→再作成)が存在すると別 PR 同士が group を共有して相互 cancel する。`github.ref`(`refs/pull/N`)は PR 番号で一意となりこの衝突がない。また workflow 接頭辞を含まないため consumer-impact-check と ci が同 PR で group 衝突し得る。
- **Option C**: `group: ${{ github.workflow }}-${{ github.ref }}`、`cancel-in-progress: true`(無条件)。**却下** — `push` to main では `github.ref = refs/heads/main` で全 main push が 1 group を共有し、新しい main push が実行中の main CI を cancel してしまう。「main push を cancel しない」要件に違反。
- **Option D**: job-level `concurrency`(各 job に同式を記述)。**却下** — pending supersession は run 単位の semantics が自然で、job 単位だと run 途中で部分的に cancel が進む中途半端な状態を作り得る。記述量も増える。
- **Option E**: 外部 cancel アクション(styfle/cancel-workflow-action 等)で旧 run を API cancel。**却下** — 追加依存・`actions: write` permission・Step 追加が要り、native 機能で要件を満たせる(YAGNI)。

## Decision Outcome

Chosen option: **Option A**。

```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.event_name == 'pull_request' && github.ref || github.run_id }}
  cancel-in-progress: ${{ github.event_name == 'pull_request' }}
```

- `ci.yml` と `consumer-impact-check.yml` に同 block を workflow トップレベルへ置く(`github.workflow` が workflow 名で isolation を保証するため同式でよい)。
- `consumer-verify.yml` には置かない — dispatch-only で group が run_id 一意となる no-op であり、意図的な診断実行同士の干渉を増やさない。

### Consequences

- Good: PR 連続 push で最新 run のみが残る(pending の置換は既定動作、in-progress は `cancel-in-progress` が cancel)。main push/dispatch は `run_id` 一意 group で現状と同一の並列性。workflow 接頭辞で将来 `pull_request` workflow が増えても衝突しない。permission・依存は不変。
- Bad: 同一 group に対する既知の edge — 古い run を手動 **re-run** すると、その run が同 group に再入して最新 run を cancel し得る(古い revision の check が cancelled になる方向 = fail-safe、且つ人間の明示操作のみが引き金)。復旧は最新 run の再実行または再 push。設計書に既知挙動として記録する。
- Neutral: group 名は case-insensitive だが `refs/pull/N/merge`・`run_id`・workflow 名はいずれも大文字を含まず影響なし。`queue: max` は `cancel-in-progress` と併用不可かつ不要のため使用しない。

---

> **2026-11-06 補足（後続 ADR への pointer — 本 ADR の決定は不変）**:
> `docs/adr/issue-326-ci-yml-dispatch-only.md`（ADR-SYd9kRt5mEssMKoPG8hnSG）により、`ci.yml` の `pull_request`/`push` trigger は Issue #365 決定 1 / #326 Stage 4 の下で撤去された。`pull_request` event が存在しなくなったため、本 ADR が採用した event 条件式（`github.event_name == 'pull_request' && github.ref || github.run_id`）は常に非 PR 分岐に解決する dead 機構となり、concurrency は `ci-<run_id>` 一意 group・`cancel-in-progress: false` の単純形に簡素化された。本 ADR の核心不変条件（非 PR run は他 run を cancel しない・run ごと一意 group・workflow 接頭辞で衝突しない）は単純形でも同一に満たされる。trigger 復活時に per-PR cancel 機構が「仕様として」戻ることを意図しない — 復活する場合は改めて設計すること。
