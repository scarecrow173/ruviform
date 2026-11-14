---
id: "TSPEC-cFTjIZpnYGLaA5BcC74ahh"
type: "test-spec"
status: "proposed"
title: "Consumer-neutral Workflow/CLI orchestration契約検証（Issue #553 RF-AUTX-01）"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/553"
  changes:
    added:
      - type: "doc"
        path: "docs/test-specs/issue-553-workflow-orchestration.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-553-workflow-orchestration-design.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-553-workflow-orchestration-design.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-553-workflow-orchestration.md"
    - "docs/designs/issue-553-workflow-orchestration-design.md"
    - "docs/adr/issue-553-workflow-cli-contract.md"
  verified-by: []
  references:
    - "docs/designs/issue-553-workflow-orchestration-design.md"
    - "evidence/rf553/verification.md"
  defers: []
  deferred-by: []
---

# Consumer-neutral Workflow/CLI orchestration契約検証（Issue #553 RF-AUTX-01）

## Purpose

`docs/designs/issue-553-workflow-orchestration-design.md` が定義する Consumer-neutral orchestration 契約の各条項が、将来の共通実装・Consumer 統合で「検証可能な保証」として保持されることを記録する。契約が変更・再設計されても、ここに列挙する Rule/Guarantee が黙って失われないことを目的とする。実装が存在しない現時点では実行テストはなく、Rule/Example は将来の検証タスクが従うべき仕様である。実装が契約と乖離した場合、または契約自体が supersede された場合に本 test-spec を見直す。

## Feature

Consumer-neutral Workflow/CLI orchestration 契約が、Versioned Bounded Plan・Fail-Closed PlanValidator・transport-neutral StepExecutionPort・Run/Step State・Cancel Requested/Confirmed・Indeterminate・Idempotency/Digest・Budget・Non-RT・Grant/Trust/Job ownership 境界・CLI Result 境界を満たすこと。

## Rules

### R-PLAN: Versioned Bounded Plan は authority を持たない

- Rule: `OrchestrationDefinition`/`ExecutablePlan` は `schema_version`/`definition_id`/`definition_version`/`inputs`/`steps`/`outputs`/`limits`/`definition_digest` を持ち、`principal`/`grant`/`approval`/`secret`/`os_credential` を fields に含めない。Step は `CallRef`（versioned）と `StepOutputRef` で結合し、暗黙 Target を禁止する。
- Rule: `PlanLimits`（`max_steps`/`max_depth`/`max_iterations`/`max_runtime`/`max_output_bytes`）を静的に持ち、無界 loop・無制限再帰・外部 I/O の無制限待機を Plan 段階で拒否する。

### R-VALID: PlanValidator は Fail-Closed

- Rule: `PlanValidator` は未知/非対応 `schema_version`・未知 Step kind・未知 `CallRef`・循環参照・未来 Step 参照・未定義 `StepOutputRef`・予算超過・Effect/DataAccess 上限の under-approximation・暗黙 Target を型付き `ValidationError` で拒否する。暗黙フォールバック・最近似 Capability 選択を行わない。
- Rule: Validator の規則内容は Consumer の Capability/Schema/Effect 意味論に依存し、generic 契約はこれらを型として保持しない。

### R-PORT: StepExecutionPort は transport-neutral で第二正本を作らない

- Rule: `StepExecutionPort` は in-process/CLI/IPC/MCP を区別せず、GUI/GPUI・OS Process/FS/Network/Shell の型を公開面に出さない。`StepHandle` は orchestration 側の参照であり、Consumer の Job/Operation/Grant/Project の第二正本を作らない。`reconcile` は Consumer/Provider の authoritative state を照会する経路である。
- Rule: `cancel` は `CancelRequested` を返し、`Cancelled`/`Completed`/`Indeterminate` の確定は `reconcile` または Consumer の Job truth に委ねる。

### R-STATE: Run/Step State は coordinator のみが所有し Indeterminate を昇格しない

- Rule: `OrchestrationRun`/`StepCheckpoint` は `run_id`/`plan_digest`/`definition_version`/`input_digest`/`caller_ref`/`steps`/`cursor`/`run_state` を持ち、`operation_id`/`job_ref`/`idempotency_key`/`expected_revision`/`output_digest`/`verdict`/`retry_policy` を checkpoint に記録する。Secret/Authority は serialize しない。
- Rule: Run state は `Created → Validated → Ready → Running → {AwaitingApproval | WaitingForJob | Paused | CancelRequested | Completed | Failed | Indeterminate}` を遷移し、`Indeterminate` は外部副作用の結果が不明な状態として Success に昇格させない。Resume は authoritative state を照会してから次 Step へ進む。
- Rule: VM stack/coroutine・Script 内部状態を checkpoint に含めない。

### R-CANCEL: Cancel Requested ≠ Confirmed

- Rule: Cancellation の `Requested` と `Confirmed` を区別する。`ruviform-exec` の `JobState.Cancelling`（request 受理・unwind 未了）と `Finished`（終端）に整合し、generic は Requested のみを記録し Confirmed を Consumer/Provider truth に委ねる。

### R-IDEM: Idempotency/Digest は orchestration が発行・記録し Consumer が正本解釈する

- Rule: `IdempotencyKey`/`OperationId`/`JobRef`/`OutputDigest`/`DefinitionDigest`/`InputDigest` は orchestration が発行・記録する参照であり、Consumer の Commit/Job/Provider が冪等性・照合の正本として解釈する。`Outcome` は `Completed | Failed | Cancelled | Indeterminate | Unsupported` の coarse 分類 + Consumer detail 参照であり、generic は detail を解釈しない。

### R-NONRT: Orchestration は non-RT only

- Rule: Plan 解釈・Step dispatch・journal・Resume はすべて `non-RT only`。RT callback での orchestration・alloc・block・IO を禁止する。

### R-GRANT: Grant/Trust/Job ownership は Consumer が所有する

- Rule: generic 契約は Principal/Grant/Approval/AdapterSeal/Trusted Issuer を型として持たない。認可は Consumer の Gateway/Policy が各 effect/commit boundary で再評価する。analyzer/plan 検証の成功は実行権限を意味しない。Workflow Run を `JobKind`/`Job` と偽装せず、子 Job の状態・Cancel 真偽は Consumer の JobPort/Provider が正本。

### R-CLI: CLI Entry/Result Encoding Adapter は Consumer 所有

- Rule: CLI Entry（引数・file 解決・envelope・exit code）は Consumer 所有。generic が提供し得るのは `CliEnvelope` 相当の結果 encoding の形（`schema_version`/`status`/`result`/`error`/`references`）と Exit Code 分類の語彙のみであり、実装は Consumer。結果 encoding/exit code の語彙は外部 Job・filesystem 副作用の成否を予測・保証せず、dry-run/preview の範囲は Consumer が定義する。#155 の「CLI framework 非昇格」方針と整合する。

### R-TYPE: Consumer 型の流入を禁止する

- Rule: generic 公開契約は `synvelis-project`/`synvelis-gateway`（`CapabilityRef`・`Grant`/`Approval`/`Effect`/`CommitGate`・`JobPort`・`Composite`/`CAS`/`Undo`）・GPUI・AudioLens 製品型・独自 Grant Policy へ直接依存しない。公開 enum は `#[non_exhaustive]`。

### R-PRECISION: 両 Consumer の精度/性能を犠牲にしない

- Rule: AudioLens f64/NaN/gap/PTS/長尺/Export semantics・Synvelis RT/DAW semantics を generic orchestration が変更・制約しない。片側 Consumer の成功だけで共通化を採用しない。

## Examples

### EX-SV-01: Synvelis Workflow run（正例）

- Given: `synvelis-cli workflow run --file render-chain.yaml --project mix.svproj`
- When: `Query(track list) → AtomicMacro(batch normalize) → JobSubmit(render) → AwaitJob → Result → If/BoundedLoop` を実行する
- Then: generic `OrchestrationDefinition`/`PlanValidator`/`StepExecutionPort`/`OrchestrationRun` の語彙で表現可能。`CallRef` は `synvelis.operation.*` CapabilityRef、`StepExecutionPort` は `synvelis-gateway`/`synvelis-render` JobPort への adapter、journal は `synvelis-automation` Coordinator が所有する。Cancel は `CancelRequested` → Gateway/JobPort 伝播 → `Cancelled`/`Indeterminate` は provider 照合。Resume は `operation_id`/`job_ref` で再照会し二重 Commit/Job を防ぐ。

### EX-SV-02: Synvelis 反例（契約違反として reject）

- Plan/Definition に `principal`/`grant`/`approval`/`os_credential` を serialize する → **R-GRANT 違反**。
- VM stack/coroutine を checkpoint に保存して Resume する → **R-STATE 違反**（rf#558 契約違反）。
- Workflow 全体を単一 atomic transaction として扱い外部副作用の rollback を generic が保証する → **R-STATE/R-GRANT 違反**（非 Atomic 性違反）。
- Script 生成 Definition を VM 内で直接 Gateway invoke する（Plan Compiler を通さない）→ **R-PORT 違反**（第二実行経路）。

### EX-AL-01: AudioLens stage pipeline（正例 — consumer-only 継続）

- Given: `al-cli run --resume <manifest>` で stage 列（decode→analyze→export）を逐次実行する
- When: 既存 `al-job` Stage resume/cancel・Manifest・f64/Dataset が充足している
- Then: 分岐・非同期 Job 待機・外部 I/O 混在・Condition を持たないため、generic `OrchestrationDefinition` の語彙で「versioned bounded plan + step refs + run cursor」に写像**可能**だが、現要件では新規依存を正当化しない。consumer-only 継続。

### EX-AL-02: AudioLens 反例

- generic 契約が `CapabilityRef`/`Grant`/`Project Revision`/`GPUI`/Synvelis 型を要求する → **R-TYPE 違反**。
- AudioLens の f64/NaN/gap/PTS/長尺/Export semantics を generic orchestration が変更・制約する → **R-PRECISION 違反**。
- Script Workflow を AudioLens に強制導入する → **対象外**（#127 で Script 非要求）。

### EX-CANCEL-01: Blocking provider work 中の Cancel（反例）

- Given: `JobSubmit(render)` 後 `AwaitJob` 中に `synvelis-cli` が SIGINT を受ける
- When: Coordinator が `CancelRequested` を記録し Gateway/JobPort へ伝播する
- Then: generic は `CancelRequested` のみを記録し、`Cancelled`/`Indeterminate` は provider 照合に委ねる。cooperative または indeterminate であり、Confirmed を待たない。**R-CANCEL/R-PORT に適合**。

### EX-CANCEL-02: CLI disconnect / Window close（反例）

- Given: `synvelis-cli` process が Workflow run 中に exit する、または GUI Window が close される
- When: Run/Job ownership は Consumer 側に残る
- Then: orchestration journal は consumer が durable に保持し、Resume/CancelRequest/Result 再照会が所有者と Status を正しく返す。generic は ownership を移さない。**R-STATE/R-GRANT に適合**。

### EX-RESUME-01: Commit/Job 完了後 Journal 前 Crash（反例）

- Given: Project Commit 後 Journal 前、または Job 完了後 Record 前に Crash する
- When: Resume 時に `operation_id`/`job_ref`/`idempotency_key` で authoritative state を照会する
- Then: 重複 Project Commit/Job 副作用 0・Unknown を Success に昇格 0。**R-STATE/R-IDEM に適合**。

## Guarantees

- generic orchestration 契約は、Versioned Bounded Plan・Fail-Closed PlanValidator・transport-neutral StepExecutionPort・Run/Step State・Cancel Requested/Confirmed・Indeterminate・Idempotency/Digest・Budget・Non-RT・Grant/Trust/Job ownership 境界・CLI Result 境界を満たす。
- Consumer 型（`synvelis-*`/GPUI/AudioLens 製品型/独自 Grant Policy）の流入を禁止する。
- 両 Consumer の精度/性能（AudioLens f64/NaN/gap/PTS/長尺/Export、Synvelis RT/DAW semantics）を犠牲にしない。
- 第二正本（Capability Registry/Policy/Project Writer/Commit engine/Job truth/Approval store/Transaction engine）を generic orchestration が複製しない。

## Non-goals

- Workflow Engine・共通 CLI crate・`synvelis-automation`/`synvelis-cli`/`al-cli` の実装。
- Synvelis Capability Gateway・Grant/Approval・Project/CommitGate/Composite・JobPort の共通基盤への移動。
- AudioLens への Workflow/Script 機能の強制導入。
- GPUI 依存の導入、RT callback での orchestration、OS/FS/Network/Shell の汎用権限の付与。
- #558〜#560 の Script Runtime/Analyzer PoC の実施またはそれらを Blocker とすること。
- 実装・両 Consumer 移行・OS Native 実走の Pass 主張。

## Risk

- 契約が文書に留まり実装がないため「共通化が本当に機能するか」は未実証。Synvelis/AudioLens が契約から乖離するリスクは、test-spec の Rule/Example と consumer-applicability の判定で監視する。
- Synvelis AUTX-05/06 の具象設計が未確定な段階で generic を固定すると、Consumer 側の発見を取り込めず手戻りが大きい。再評価 Trigger（ADR）で Synvelis 実装完了後に generic を再検討する。
- AudioLens が現要件で orchestration を必要としないため、片側成功での共通化採用を防ぐ。AudioLens または第三 Consumer が独立した必要性を示した時点で再評価する。
