---
id: "IMPL-034Z0uHvTxRFpgNslvN5XM"
type: "impl"
status: "completed"
title: "Issue #727 bounded non-RT job executor placement decision"
created: "2026-11-17"
updated: "2026-11-17"
owners:
  - "devin-worker"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/727"
    - "https://github.com/scarecrow173/synvelis/issues/518"
  changes:
    added:
      - type: "file"
        path: "evidence/rf727/"
      - type: "doc"
        path: "docs/specs/issue-727-job-executor.md"
      - type: "doc"
        path: "docs/designs/issue-727-bounded-nonrt-job-executor.md"
      - type: "doc"
        path: "docs/adr/issue-727-bounded-nonrt-job-executor-placement.md"
      - type: "doc"
        path: "docs/plans/issue-727-job-executor.md"
      - type: "doc"
        path: "docs/tasks/issue-727-job-executor.md"
      - type: "doc"
        path: "docs/test-specs/issue-727-job-executor.md"
      - type: "doc"
        path: "docs/impl/ir/issue-727-job-executor.md"
      - type: "doc"
        path: "docs/impl/exp/issue-727-job-executor.md"
      - type: "file"
        path: "docs/impl/exp/issue-727-job-executor-exp.jsonl"
    modified:
      - type: "file"
        path: "docs/designs/shared-foundations-and-modularization-policy.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-727-job-executor.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/exp/issue-727-job-executor.md"
    - "evidence/rf727/verification.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/test-specs/issue-727-job-executor.md"
  verified-by:
    - "evidence/rf727/verification.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/727"
    - "https://github.com/scarecrow173/synvelis/issues/518"
    - "docs/specs/issue-727-job-executor.md"
    - "docs/designs/issue-727-bounded-nonrt-job-executor.md"
    - "docs/adr/issue-727-bounded-nonrt-job-executor-placement.md"
    - "docs/test-specs/issue-727-job-executor.md"
    - "evidence/rf727/verification.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-727-job-executor-exp.jsonl"
    rejected: []
---

# Issue #727 bounded non-RT job executor placement decision

## Summary

`scarecrow173/ruviform#727` の責務判断 slice。bounded 非 RT
job executor primitive を **Ruviform `ruviform-exec` への
module 追加**で決裁し、
`docs/adr/issue-727-bounded-nonrt-job-executor-placement.md`
に ADR（status: accepted）として記録した。API 契約は
`docs/designs/issue-727-bounded-nonrt-job-executor.md` に
定義し、`docs/designs/shared-foundations-and-modularization-
policy.md` L282 の stale 行番号参照を最小修正した。
executor の実装コードは作成していない（docs-only slice）。

## Context

- spec: `docs/specs/issue-727-job-executor.md`（approved）
- design: `docs/designs/issue-727-bounded-nonrt-job-executor.md`
  （approved）
- ADR: `docs/adr/issue-727-bounded-nonrt-job-executor-placement.md`
  （accepted — Option 1: ruviform-exec module 追加）
- plan: `docs/plans/issue-727-job-executor.md`（approved）
- test-spec: `docs/test-specs/issue-727-job-executor.md`
  （approved）
- task: `docs/tasks/issue-727-job-executor.md`
- Issue: `scarecrow173/ruviform#727`（bounded 非 RT job
  executor primitive の責務判断 — consumer は synvelis#518）

## Implementation

### Changes

- `docs/specs|designs|adr|plans|tasks|test-specs/
  issue-727-*.md` — Issue #727 の内部文書チェーン（要求・
  API 設計・責務判断 ADR・計画・task・検証規範）。
- `docs/designs/issue-727-bounded-nonrt-job-executor.md` —
  `ExecutorConfig`/`JobExecutor`/`QueuedJob`/`SubmitError`/
  `ExecutorError` の API 契約、bounded 性（worker 数・queue
  容量・in-flight 上限）・協調 cancel/preemption・結果
  join・RT 分離の設計。
- `docs/adr/issue-727-bounded-nonrt-job-executor-placement.md` —
  責務判断の決裁記録。`relations.source` に sv#518 URL を
  記載し stable URL での相互参照を成立させた。
- `docs/designs/shared-foundations-and-modularization-policy.md`
  — L282 の stale 行番号参照（L240/L352/L392 → L262/L375/
  L415）を修正し、本 ADR への参照を追加。`updated` を
  2026-11-17 に更新。
- `evidence/rf727/` — baseline/事後の doc_status lint・
  route_graph・fmt/prek の実行証拠と検証照合記録。
- `docs/impl/{ir,exp}/issue-727-job-executor.*` — 本記録と
  実験ログ。

### Decisions During Implementation

- **Ruviform `ruviform-exec` module 追加を採用**: 汎用実行
  契約は synvelis-driven 実行境界 ADR Decision 6 で Ruviform
  責務と確定済み。sv#518 が最初の実需として実在し、
  shared-foundations L336-337 の「汎用利用者の実在」
  トリガを満たす。依存方向・RT contract・consumer 依存が
  既存 `ruviform-exec` と一致するため module 追加で十分。
- **新規 crate を却下**: 独立 compile/OS/配布境界の必要が
  なく module-first 原則に反する。`OfflineJob` pool 化も
  「1 job = 1 thread」契約を壊すため却下。consumer 側実装は
  汎用実行契約の所在を誤り、将来の重複実装リスクがある。
- **preemption = 協調 cancel + budget checkpoint**: 任意
  Rust コードの強制中断は安全に提供不能。強制 preemption は
  Deferred（別途 ADR 要）。
- **design doc + ADR の併用**: Issue が「設計 doc または
  ADR」を許可し、責務判断の決裁記録として ADR（accepted）を
  必須成果物とし、API 契約詳細は design doc に分離した。
- **shared-foundations L282 の最小修正**: stale 行番号参照を
  現在の正しい行番号（L262・L375・L415）に修正し、ADR への
  参照を追加。baseline 本文の改訂は範囲外のため、改訂要否の
  結論のみを ADR に記録した。
- **docs index 未同期のまま**: `docs/*/README.md` は
  Coordinator が統合時に再生成する運用のため本 slice では
  一切触れない。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-727-job-executor-exp.jsonl`（+
  `issue-727-job-executor.md`）— consumer 照合・既存
  primitive 確認・配置判断・stale 表記評価の観察/判断/
  検証記録。

### Rejected

- 新規 crate 追加・`OfflineJob` pool 化・consumer 側実装 —
  ADR の Considered Options で却下理由を記録。
- executor の実装コード — Issue #727「対象外」明記、採用時は
  別 Issue。

## Validation

canonical 検証（`evidence/rf727/verification.md` に一覧）:

- `node .agents/skills/doc-status/scripts/doc_status.js lint
  --dir docs/<type>`（adr/designs/specs/plans/tasks/
  discovery/test-specs）— Pass（exit 0。baseline 採取済み、
  新規文書由来の finding は index 未同期由来の
  `index-missing-entry`/`orphan-index`/`orphan-navigation`
  warning のみ — Coordinator が統合時に index 再生成で解消）
- `node .agents/skills/doc-driven-dev-graph/scripts/
  route_graph.js --json` — baseline blocked（既存
  broken-relation・focus-required）を記録。新規文書追加後も
  同等の blocked で、新規 finding の増加なし
- `node .agents/skills/adr-doc/scripts/review_adr.js
  --dir docs/adr`・`audit_adr.js`・`check_code_links.js` —
  実行・結果記録
- `mise run fmt-check` — Pass（exit 0・docs-only）
- `mise exec -- prek run --files <変更ファイル>` — Pass
- review-protocol self-review — blocker なし

NotRun: `*-msrv`・`*-full`・3 OS 横断・cargo 系 lane
（`check`/`lint`/`test`/`build`）— docs-only slice のため
POLICIES.md §5 により対象外。Unsupported: なし。

## Risks

- 責務判断が文書間で drift すると sv#518 が誤った前提で
  実装を進める — spec/design/ADR/test-spec で一貫した配置・
  API 契約を記録し、test-spec R9 で文書間整合を規範化。
- API 契約が `OfflineJob` の既存 surface と矛盾すると実装
  時に再設計が必要になる — design doc で `OfflineJob` と
  同一の observe surface を契約化し、差分（pool worker
  実行・shutdown semantics）のみを明示。
- `JobState` queued 表現の未確定事項 — 実装 Issue で確定、
  契約上は「queued と実行中を区別可能」であることのみ要求。
- route_graph baseline blocked が新規文書の問題と誤認される
  — baseline を `evidence/rf727/` に記録し、前後差分で
  新規 finding がないことを確認。

## Follow-ups

- Coordinator による docs index（`docs/*/README.md`）再生成。
- PR merge 後の Issue #727 close は Coordinator が実施。
- executor の実装は採用時の別 Issue（ADR `defers`・
  spec/design Deferred Design Concerns に記録）。
- baseline 本文の「ワーカースレッドプール」表記改訂は
  baseline 改訂 Issue に委ねる（ADR「baseline stale 表記の
  扱い」節に結論を記録）。
- sv#518 側で `RenderExecutor` port の実装として
  `JobExecutor` を注入する consumer 側配線（consumer gate）。
