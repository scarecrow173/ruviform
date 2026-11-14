---
id: "DESIGN-sh006tWu0ht5WmSqG0V85w"
type: "design"
status: "approved"
title: "Consumer-neutral Workflow/CLI orchestration契約設計（Issue #553 RF-AUTX-01）"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/553"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-553-workflow-orchestration-design.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-553-workflow-orchestration.md"
  implemented-by: []
  depends-on:
    - "docs/adr/issue-553-workflow-cli-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
    - "docs/designs/issue-155-headless-cli-foundation.md"
    - "docs/designs/issue-558-script-contract-design.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-553-workflow-orchestration.md"
    - "docs/adr/issue-553-workflow-cli-contract.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-553-workflow-orchestration.md"
  references:
    - "docs/adr/issue-553-workflow-cli-contract.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
    - "docs/designs/issue-155-headless-cli-foundation.md"
    - "docs/designs/issue-558-script-contract-design.md"
    - "evidence/rf553/contract-trace.md"
    - "evidence/rf553/consumer-applicability.md"
  defers: []
  deferred-by: []
---

# Consumer-neutral Workflow/CLI orchestration契約設計（Issue #553 RF-AUTX-01）

本書は Issue #553 の DESIGN-ONLY 契約を定義する。実装 crate・公開 Rust API・Consumer コードは含まず、将来の共通化実装が従うべき型・意味論・Fail-Closed 規則・Owner 境界を固定する。

## Context

- Synvelis AUTX-00（synvelis#259、`docs/automation/` PR #420 マージ済み）は Action=Capability 投影、Atomic Macro=既存 Batch/Composite、Workflow=非 RT orchestration、CLI=独立 Adapter とする統合契約を持つ。AUTX-05（#264）が `WorkflowDefinition`/`PlanCompiler`/`ExecutablePlan`、AUTX-06（#265）が Coordinator/Run journal/Resume/Cancel、AUTX-03（#262）が `synvelis-cli` と `CliEnvelope`/Exit Code を Owner とする。
- AudioLens（audio-lens#127/#189）は `al-cli`/`al-job`/`al-dataset`/`al-analysis` で Stage Resume・Cancel・Manifest・f64 解析・Portable Export を既に持ち、Workflow DSL・分岐・非同期 Job 待機・外部 I/O 混在を要求していない。Script 導入も明示的に非要求。
- Ruviform 既存: `ruviform-exec` は `CancellationToken`/`Progress`/`OfflineDriver`/`OfflineJob`（`JobState{Running,Cancelling,Finished}`・`JobOutcome{Completed,Cancelled,Failed}`・`JobEvent`・`JobError`）を提供するが、Plan/Step/Run journal/Resume/Idempotency/CLI Envelope は持たない。`ruviform-cli-sample` は #155 で「利用例であり Ruviform API に昇格しない」と確定済み。
- rf#558〜#560 は Script Runtime/Analyzer の言語非依存契約・PoC であり、本契約とは非重複（Script が生成する Definition は Consumer の Plan Compiler へ lower される入力であって、orchestration 契約の一部ではない）。

## Decision overview

**契約語彙は採用、実装は Deferred（条件付き Go）。** Consumer-neutral な orchestration の「形」（Versioned Bounded Plan・Step Port・Run/Step State・Cancel Requested/Confirmed・Indeterminate・Idempotency/Digest・Budget・CLI Result 境界）を本書で契約として固定するが、Ruviform 共通 Engine/CLI crate は作らない。Synvelis は `synvelis-automation`/`synvelis-cli` が具象 Owner のまま進め、AudioLens は既存 stage-job モデルを維持する。共通実装は、(a) Synvelis 側実装で契約形が安定し、(b) AudioLens または第三 Consumer が独立した有界 Step orchestration の必要性を示した時点で再評価する。

## C1. 棚卸し結果（実在 API の対応表）

| 既存資産 | 実在 API/契約 | 本契約での扱い |
| --- | --- | --- |
| `ruviform-exec` | `CancellationToken`/`Progress`/`OfflineDriver::run{,_parallel}`/`OfflineJob{spawn,state,is_finished,cancel,cancel_token,progress,events,try_wait,wait,detach}`/`JobScope`/`JobState`/`JobOutcome`/`JobEvent`/`JobError` | **Reuse** — Cancel Requested≠Confirmed・進捗・job handle の primitive。Consumer orchestration の下位 substrate であり変更不要 |
| `ruviform-cli-sample` | `probe`/`spawn_analyze`/`probe_json`/`report_json`/`error_json`/`event_json`/`PipelineError` | **Consumer-only（参照例）** — #155 で「sample は例、API に昇格しない」と確定。CLI shell/envelope/exit code は Consumer 所有 |
| #155 headless 設計 | GUI 非依存分離の `cargo tree` 検証、headless/GUI feature 分離方針 | **Reuse（方針）** — 本契約の headless/GUI 依存境界の検証手法として継承 |
| `ruviform-media-core` | `TimeBase`/`MediaTime`/`ClockDomainId`/`StreamId`/`Fact` | **Reuse（条件付き）** — Step I/O の時間・観測値が必要な Consumer のみ。generic 契約自体はこれに依存しない |
| Synvelis `synvelis-gateway` | `CapabilityRef`/`CapabilityDescriptor`/`SchemaRegistry`/`PolicyEvaluator`/Grant/Approval/`CommitGate`/`ProjectController`/`JobPort`/`GatewayOutcome`/`ErrorEnvelope`/`AdapterSeal`/`CallerIdentity` | **Consumer-only** — 唯一正本。generic 契約へ型を流入させない |
| Synvelis `synvelis-automation`（計画） | `ActionPresentation`/`AtomicMacroDefinition`/`WorkflowDefinition`/`PlanCompiler`/`ExecutablePlan`/`WorkflowRun`/`StepCheckpoint` | **Consumer-only** — AUTX-05/06 が具象 Owner。本契約はその「形」の語彙のみ提供 |
| Synvelis `synvelis-cli`（計画） | `CliEnvelope{schema_version,status,...}`・Exit 0/2〜10・`--file` 契約 | **Consumer-only** — CLI Port の具象は Consumer |
| AudioLens `al-cli`/`al-job`/`al-dataset`/`al-analysis` | Stage Resume/Cancel・Manifest・f64 解析・Portable Export・`probe/run/resume/inspect` | **Consumer-only** — 既存モデルで充足。Workflow 強制なし |
| rf#558〜#560 | `ScriptBackend`/`ScriptSession`/`ScriptValue`/`ScriptArtifact`/`ScriptAnalyzer`/`ExecutionBudget`/`ScriptCancel` | **分離（非重複）** — Script VM/Analyzer は別契約。本契約の Plan は Script lowering の**出力先**であり VM を orchestration に含めない |

結論: Ruviform 既存で orchestration の「実行」部分（Job handle・Cancel token・Progress）は揃っているが、「計画・検証・journal・Resume・Port」の部分は存在しない。Synvelis が具象を先行実装するため、Ruviform が今 generic Engine を作る根拠はない。

## C2. Consumer-neutral 契約語彙（型レベル）

以下は **contract-level の型名と不変条件** であり、Rust 公開 API を確定するものではない。実装時は `#[non_exhaustive]`・crate 間依存方向・`non-RT only` ラベルの既存規約（`docs/adr/public-enum-non-exhaustive-policy.md`、architecture baseline）に従う。

### C2.1 Versioned Bounded Plan

```
OrchestrationDefinition {
  schema_version,            // Versioned。未知 Version は Fail Closed
  definition_id, definition_version,
  inputs,                    // 型付き入力（Consumer Schema への参照は Port 越し）
  steps: [OrchestrationStep],// Tagged Union。Consumer が意味を解釈
  outputs,
  limits: PlanLimits,        // max_steps, max_depth, max_iterations, max_runtime, max_output_bytes
  definition_digest          // canonical 化した決定的 Digest
}
```

不変条件:
- Plan は **authority を持たない**。Grant/Approval/Secret/OS credential/Principal 実体を fields に含めない（参照 ID のみ）。
- `Step` は `CallRef`（Consumer Capability/Stage/Job の versioned 参照）と typed `StepOutputRef` で結合し、暗黙 Target（Selection/ActiveWindow/LastEdited 相当）を禁止する。
- 有界性: `max_steps`/`max_depth`/`max_iterations`/`max_runtime`/`max_output_bytes` を静的に持ち、無界 loop・無制限再帰・外部 I/O の無制限待機を Plan 段階で拒否する。

### C2.2 PlanValidator（Fail-Closed 規則の形）

`PlanValidator` は Definition + Consumer Capability Snapshot → `ExecutablePlan | ValidationError` を返す Port。規則の**形**は中立だが、規則の**内容**は Consumer の Capability/Schema/Effect 意味論に依存する。

Fail-Closed 対象（両 Consumer で共通の形）:
- 未知/非対応 `schema_version`・未知 Step kind・未知 `CallRef` → `UnknownVersion`/`UnknownStep`/`UnknownCapability`
- 循環参照・未来 Step 参照・未定義 `StepOutputRef` → `Cycle`/`ForwardRef`/`MissingOutput`
- 予算超過（steps/depth/iterations/runtime/output）→ `BudgetExceeded`
- Effect/DataAccess 上限の静的 under-approximation → `EffectBoundExceeded`
- 暗黙 Target・Selection 依存 → `ImplicitTarget`

Consumer 固有の規則（Synvelis: EffectSet/DataAccessSet/Grant Scope/Budget/Expiry/Availability/Revision、AudioLens: stage 依存・precision 要件）は Validator の Consumer adapter が注入し、generic 契約はこれらを型として保持しない。

### C2.3 StepExecutionPort（transport-neutral orchestration Port）

```
StepExecutionPort {
  submit(step: BoundStep, ctx: RunContext) -> StepHandle;
  poll(handle) -> StepStatus;         // non-blocking
  cancel(handle) -> CancelReceipt;    // Requested を返す。Confirmed は別経路
  reconcile(handle) -> AuthoritativeOutcome; // Provider/Consumer 正本への照会
}
```

不変条件:
- Port は **transport-neutral**（in-process/CLI/IPC/MCP を区別しない）。GUI/GPUI・OS Process/FS/Network/Shell の型を公開面に出さない。
- `StepHandle` は orchestration 側の参照であり、Consumer の Job/Operation/Grant/Project の第二正本を作らない。`reconcile` は Consumer/Provider の authoritative state を照会する経路であり、generic 側は結果をそのまま `AuthoritativeOutcome` として記録する。
- `cancel` が返すのは `CancelRequested` であり、`Cancelled`/`Completed`/`Indeterminate` の確定は `reconcile` または Consumer の Job truth に委ねる。

### C2.4 RunState / Checkpoint / RecoveryCursor

```
OrchestrationRun {
  run_id, plan_digest, definition_version,
  input_digest, caller_ref,                 // 参照のみ。Secret/Authority は serialize しない
  steps: [StepCheckpoint],
  cursor, run_state
}
StepCheckpoint {
  step_id, call_ref?, operation_id?, job_ref?,
  idempotency_key?, expected_revision?,
  output_digest?, verdict, retry_policy
}
```

Run state（Synvelis AUTX-06 と同型の語彙）:
`Created → Validated → Ready → Running → {AwaitingApproval | WaitingForJob | Paused | CancelRequested | Completed | Failed | Indeterminate}`

不変条件:
- Coordinator は **Run/Step の進行状態と resume cursor のみ**を所有する。Project/Job/Grant/Approval/Provider の第二正本を持たない。
- `Indeterminate` は「外部副作用の結果が不明」な状態であり、Success に昇格させない。Resume は `operation_id`/`job_ref`/`idempotency_key` で authoritative state を照会してから次 Step へ進む。
- `CancelRequested ≠ Cancelled`。Cancellation の Confirmed は Consumer/Provider が返す。
- VM stack/coroutine・Script 内部状態を checkpoint に含めない（rf#558 契約と整合）。

### C2.5 Idempotency / Outcome / Digest

- `IdempotencyKey`/`OperationId`/`JobRef`/`OutputDigest`/`DefinitionDigest`/`InputDigest` は orchestration が発行・記録する参照であり、Consumer の Commit/Job/Provider が冪等性・照合の正本として解釈する。
- `Outcome` は `Completed | Failed | Cancelled | Indeterminate | Unsupported` の coarse 分類 + Consumer 固有 detail への参照。generic は detail を解釈しない。

### C2.6 Resource Budget / Non-RT / Grant / Trust / Job ownership 境界

- **Non-RT**: Plan 解釈・Step dispatch・journal・Resume はすべて `non-RT only`。RT callback での orchestration・alloc・block・IO を禁止する。
- **Budget**: `PlanLimits`（C2.1）に加え、実行時の Step 数・経過時間・出力サイズを Coordinator が計測し、超過は `BudgetExceeded` で Fail Closed。
- **Grant/Trust**: generic 契約は Principal/Grant/Approval/AdapterSeal/Trusted Issuer を**型として持たない**。認可は Consumer の Gateway/Policy が各 effect/commit boundary で再評価する。analyzer/plan 検証の成功は実行権限を意味しない。
- **Job ownership**: Workflow Run を `JobKind`/`Job` と偽装しない。子 Job の状態・Cancel 真偽は Consumer の JobPort/Provider が正本。
- **CLI Port 境界**: CLI Entry（引数・file 解決・envelope・exit code）は Consumer 所有。generic が提供し得るのは `CliEnvelope` 相当の**結果 encoding の形**（`schema_version/status/result/error/references`）と Exit Code 分類の**語彙**のみであり、実装は Consumer。dry-run/preview 系の入口とその保証範囲も Consumer が定義し、generic の結果 encoding/exit code 語彙は外部 Job・filesystem 副作用の成否を予測・保証しない。

## C3. 2 Consumer 代表 Scenario（正例・反例）

### Synvelis（正例）

`synvelis-cli workflow run --file render-chain.yaml --project mix.svproj`:
`Query(track list) → AtomicMacro(batch normalize, ≤64, 1 CAS/1 Undo) → JobSubmit(render) → AwaitJob → Result → If/BoundedLoop`

- generic `OrchestrationDefinition`/`PlanValidator`/`StepExecutionPort`/`OrchestrationRun` の語彙で表現可能。`CallRef` は `synvelis.operation.*` CapabilityRef、`StepExecutionPort` は `synvelis-gateway`/`synvelis-render` JobPort への adapter、journal は `synvelis-automation` Coordinator が所有。
- Cancel: `synvelis-cli` の SIGINT/`cancel` → Coordinator `CancelRequested` → Gateway/JobPort へ伝播 → `Cancelled`/`Indeterminate` は provider 照合。Resume: `operation_id`/`job_ref` で再照会し二重 Commit/Job を防ぐ。

### Synvelis（反例 — 契約違反として reject）

- Plan/Definition に `principal`/`grant`/`approval`/`os_credential` を serialize する → **Grant/Trust 境界違反**。
- VM stack/coroutine を checkpoint に保存して Resume する → **rf#558 契約違反**（typed Definition/Plan + Coordinator state のみが Resume 正本）。
- Workflow 全体を単一 atomic transaction として扱い外部副作用の rollback を generic が保証する → **非 Atomic 性違反**。
- Script 生成 Definition を VM 内で直接 Gateway invoke する（Plan Compiler を通さない）→ **第二実行経路**。

### AudioLens（正例 — consumer-only 継続）

`al-cli run --resume <manifest>`: stage 列（decode→analyze→export）の逐次実行。既存 `al-job` Stage resume/cancel・Manifest・f64/Dataset が充足しており、分岐・非同期 Job 待機・外部 I/O 混在・Condition を持たない。generic `OrchestrationDefinition` の語彙で「versioned bounded plan + step refs + run cursor」に写像**可能**だが、現要件では新規依存を正当化しない。

### AudioLens（反例）

- generic 契約が `CapabilityRef`/`Grant`/`Project Revision`/`GPUI`/Synvelis 型を要求する → **Consumer 型流入禁止違反**。
- AudioLens の f64/NaN/gap/PTS/長尺/Export semantics を generic orchestration が変更・制約する → **精度/性能回帰違反**（#127/#189 の独立 Oracle を犠牲にしない）。
- Script Workflow を AudioLens に強制導入する → **対象外**（#127 で Script 非要求）。

## C4. Fault / Recovery / Counterexample Matrix

| 状況 | generic 契約の振る舞い | 正本 |
| --- | --- | --- |
| Blocking provider/host work 中の Cancel | `CancelRequested` を記録。cooperative または indeterminate。Confirmed は待たない | Consumer/Provider Job truth |
| CLI process exit / disconnect / Window close | Run/Job ownership は Consumer 側に残る。orchestration journal は consumer が durable に保持 | Consumer（Synvelis #265 受入条件と同型） |
| Commit/Job 完了後 Journal 前 Crash | Resume 時に `operation_id`/`job_ref`/`idempotency_key` で authoritative state を照会。重複 side effect を起こさない | Consumer CommitGate/JobPort/Provider |
| Network timeout / provider restart | `Indeterminate` → reconcile → `Completed/Failed/Cancelled` を確定してから次 Step | Provider/Consumer |
| Grant revoke / expiry / policy epoch 変更 | 次 Step 実行時に Consumer が再評価。Plan 保存・Human login の存在は権限にならない | Consumer Gateway/Policy |
| Plan/Definition の version/ digest drift | `ReplanRequired`/Fail Closed。古い承認を使い回さない | PlanValidator + Consumer |
| 外部副作用（Export/Publish/外部 I/O） | generic は atomicity を保証しない。補償は Consumer Capability の明示 Undo/Repair | Consumer |

## C5. Deferred concerns（本設計で固定しない）

- Rust 公開 API の正確な型・crate 名・feature 構成（実装 Issue で決定）
- `ruviform-orchestration` 仮称 crate の新設可否（再評価 Trigger 後）
- Synvelis `synvelis-automation`/`synvelis-cli` の具象型と generic 契約の adapter 形状（Synvelis 側設計が先行）
- AudioLens への orchestration 適用（現要件では consumer-only 継続）
- 分散 Queue/Server/常駐 Scheduler/自動 Trigger/Webhook（両 Consumer とも対象外）
- 実 OS/実 Job/実 Provider での Resume/Cancel 実測（各 Consumer の E2E Gate）

## Verification notes

- 本設計の検証は文書照合（`evidence/rf553/contract-trace.md` の対応表、test-spec の Rule/Example 割当、`mise run fmt-check` + `prek` の軽量チェック）に限定する。
- `mise run check`/`test`/`test-rt`/`test-ci-consumer-verify` はコード変更がないため対象外（実行する場合も文書契約の照合であり runtime Pass ではない）。実 Job/Resume/Cancel/OS の受入は Synvelis #267・AudioLens #127/#189 の各 Gate が Owner。
