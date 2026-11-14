# RF553 Workflow/CLI orchestration契約 — 2 Consumer 適用性判定ログ（Reuse/Extend/New/Consumer-only/Deferred）

- Issue: https://github.com/scarecrow173/ruviform/issues/553
- 作業 branch: `docs/553-workflow-orchestration`
- 作成日: 2026-11-14
- 対象スライス: **DESIGN-ONLY**（実装なし。適用性の判定のみ）
- 判定語彙: `Reuse`（既存契約/基盤をそのまま利用）/ `Extend`（既存契約の拡張が必要）/ `New`（新規契約/実装が必要）/ `Consumer-only`（Consumer 側責務。Ruviform 共通化対象外）/ `Deferred`（現時点で判定保留）

## 1. 判定原則

- **Ruviform の製品型非依存**: Synvelis Gateway/Project 型・AudioLens 型・GPUI 型・独自 Grant Policy を generic 公開契約に含めない。
- **1 Consumer の成功 ≠ 共通採用**: Synvelis の Workflow 採用は AudioLens の採用を意味せず、逆も同様。両 Consumer の独立受入（rf#294、audio-lens#127、synvelis#267）が必要。
- **非重複**: 既存 `ruviform-exec`/`ruviform-cli-sample`/#155/#558〜560/Consumer Coordinator/Gateway/Job と責務を重複させない。
- **第二正本の禁止**: Capability Registry/Policy/Project Writer/Commit engine/Job truth/Approval store/Transaction engine を generic orchestration が複製しない。
- **正直な未実測**: 未実行の共存・実 OS・実 Job/Resume/Cancel は NotRun/Deferred。Pass に偽装しない。

## 2. Synvelis Consumer Scenario

Synvelis は AUTX-00（#259）で Action/Macro/Workflow/Script/Headless CLI の統合契約を所有し、AUTX-01〜08（#260〜#267）で各 lane を計画、SV-SCRIPT-01〜04（#271〜#274）で Script lane を計画している。

### 2.1 代表 Scenario（正例）

`synvelis-cli workflow run --file render-chain.yaml --project mix.svproj`

- Step 列: `Query(track list) → AtomicMacro(batch normalize, ≤64, 1 CAS/1 Undo) → JobSubmit(render) → AwaitJob → Result → If/BoundedLoop`
- generic `OrchestrationDefinition`/`PlanValidator`/`StepExecutionPort`/`OrchestrationRun` の語彙で表現可能。
- `CallRef` は `synvelis.operation.*` CapabilityRef（versioned）。`StepExecutionPort` は `synvelis-gateway`/`synvelis-render` JobPort への adapter。journal/Coordinator は `synvelis-automation`（AUTX-06 #265）が所有。
- Cancel: `synvelis-cli` の SIGINT/`cancel` → Coordinator `CancelRequested` → Gateway/JobPort へ伝播 → `Cancelled`/`Indeterminate` は provider 照合。Resume: `operation_id`/`job_ref` で再照会し二重 Commit/Job を防ぐ。

### 2.2 代表 Scenario（反例 — 契約違反として reject）

- Plan/Definition に `principal`/`grant`/`approval`/`os_credential` を serialize → **Grant/Trust 境界違反**。
- VM stack/coroutine を checkpoint に保存して Resume → **rf#558 契約違反**（typed Definition/Plan + Coordinator state のみが Resume 正本）。
- Workflow 全体を単一 atomic transaction として扱い外部副作用の rollback を generic が保証 → **非 Atomic 性違反**。
- Script 生成 Definition を VM 内で直接 Gateway invoke（Plan Compiler を通さない）→ **第二実行経路**。
- Atomic Macro（≤64/非 Nested/1 CAS/1 Undo）と Workflow（非 RT orchestration）を混同 → **AUTX-04/05 境界違反**。

### 2.3 判定表

| 候補 | 判定 | 根拠・境界 |
| --- | --- | --- |
| `WorkflowDefinition`/`PlanCompiler`/`ExecutablePlan` の具象 | **Consumer-only** | AUTX-05（synvelis#264）が `synvelis-automation` で具象 Owner。generic 契約は「形」の語彙のみ提供 |
| `WorkflowRun`/`StepCheckpoint`/Coordinator/Resume/Cancel | **Consumer-only** | AUTX-06（synvelis#265）が具象 Owner。journal authority は Consumer |
| `synvelis-cli`/`CliEnvelope`/Exit Code/`--file` | **Consumer-only** | AUTX-03（synvelis#262）が具象 Owner。CLI Entry/Result Encoding は Consumer |
| `CapabilityRef`/`Grant`/`Approval`/`Effect`/`CommitGate`/`JobPort`/`Composite`/`CAS`/`Undo` | **Consumer-only** | `synvelis-gateway`/`synvelis-project` が唯一正本。generic 契約へ型を流入させない |
| `AtomicMacroDefinition`→`synvelis.operation.batch`→`ProjectOp::Composite` | **Consumer-only** | AUTX-04（synvelis#263）が具象 Owner。1 Project/1 ExpectedRevision/≤64/非 Nested/1 CAS/1 Undo |
| Script Action/Workflow lowering | **分離（rf#558〜560 + SV-SCRIPT-01〜04）** | Script VM/Analyzer は rf#558〜560、Synvelis 側 lowering は SV-SCRIPT-01〜04。orchestration 契約は Script を含めない |
| GPUI Action Palette/Workflow Editor/Run Monitor | **対象外** | AUTX-07（synvelis#266）/SV-SCRIPT-04（#274）が具象 Owner。Driver-neutral ViewSpec は別 lane |

## 3. AudioLens Consumer Scenario

AudioLens は `al-cli`/`al-job`/`al-dataset`/`al-analysis`/`al-service`/`al-gui` で Stage Resume/Cancel・Manifest・f64 解析・Portable Export を既に持つ。audio-lens#127（継続追従）・#189（旧 WebView 撤去）で CLI/headless 分離と精度独立 Oracle を要求。

### 3.1 代表 Scenario（正例 — consumer-only 継続）

`al-cli run --resume <manifest>`: stage 列（decode→analyze→export）の逐次実行。

- 既存 `al-job` Stage resume/cancel・Manifest・f64/Dataset が充足。
- 分岐・非同期 Job 待機・外部 I/O 混在・Condition を持たない。
- generic `OrchestrationDefinition` の語彙で「versioned bounded plan + step refs + run cursor」に写像**可能**だが、現要件では新規依存を正当化しない。consumer-only 継続。

### 3.2 代表 Scenario（反例）

- generic 契約が `CapabilityRef`/`Grant`/`Project Revision`/`GPUI`/Synvelis 型を要求 → **Consumer 型流入禁止違反**。
- AudioLens の f64/NaN/gap/PTS/長尺/Export semantics を generic orchestration が変更・制約 → **精度/性能回帰違反**（#127/#189 の独立 Oracle を犠牲にしない）。
- Script Workflow を AudioLens に強制導入 → **対象外**（#127 で Script 非要求）。
- 片側（Synvelis）の成功だけで共通化を採用 → **禁止**（両 Consumer の独立受入が必要）。

### 3.3 判定表

| 候補 | 判定 | 根拠・境界 |
| --- | --- | --- |
| `al-cli`/`al-job`/`al-dataset`/`al-analysis`/`al-service` | **Consumer-only** | 既存 stage-job モデルで充足。Workflow/Script 強制なし |
| Stage Resume/Cancel・Manifest | **Consumer-only（既存 Reuse）** | `al-job` が既存。generic orchestration の適用は現要件で不要 |
| f64/NaN/gap/PTS/長尺/Export semantics | **Consumer-only** | `al-analysis`/`al-dataset`/`al-cli` が独立 Oracle を保持。generic が変更・制約しない |
| Script Workflow/Analyzer | **対象外** | audio-lens#127 で Script 非要求。rf#558〜560 の Consumer 適用性は別評価 |
| GPUI/GUI | **対象外** | `al-gui` が Consumer-only。Driver-neutral ViewSpec は別 lane |

## 4. Ruviform 既存資産の判定表

| 候補 | 判定 | 根拠・境界 |
| --- | --- | --- |
| `ruviform-exec` `CancellationToken`/`Progress`/`OfflineDriver`/`OfflineJob`/`JobScope`/`JobState`/`JobOutcome`/`JobEvent`/`JobError` | **Reuse** | 既存。Cancel Requested≠Confirmed（`JobState.Cancelling` vs `Finished`）・進捗・job handle の primitive。Consumer orchestration の下位 substrate であり変更不要 |
| `ruviform-cli-sample` `probe`/`spawn_analyze`/`*_json`/`PipelineError` | **Consumer-only（参照例）** | #155 で「sample は例・API に昇格しない」と確定。CLI shell/envelope/exit code は Consumer |
| #155 headless 分離方針（`cargo tree` 検証・GUI 非依存） | **Reuse（方針）** | headless/GUI 依存分離の検証手法を継承 |
| `ruviform-media-core` `TimeBase`/`MediaTime`/`ClockDomainId`/`StreamId`/`Fact` | **Reuse（条件付き）** | Step I/O の時間・観測値が必要な Consumer のみ。generic 契約自体はこれに依存しない |

## 5. Generic 契約語彙の判定表（New/Deferred）

| 候補 | 判定 | 最小実装責務 / Owner |
| --- | --- | --- |
| `OrchestrationDefinition`/`PlanLimits`（Versioned Bounded Plan の形） | **New（契約文書化）/ 実装 Deferred** | design §C2.1。具象は Synvelis AUTX-05 |
| `PlanValidator`（Fail-Closed 規則の形） | **New（契約）/ Consumer-only 実装** | design §C2.2。規則内容は Consumer adapter |
| `StepExecutionPort` | **New（契約）/ 実装 Deferred** | design §C2.3。transport-neutral orchestration Port |
| `OrchestrationRun`/`StepCheckpoint`/`RecoveryCursor` | **New（契約）/ Consumer-only 実装** | design §C2.4。journal authority は Consumer |
| Cancel Requested/Confirmed・Indeterminate | **Reuse概念 → 契約固定** | design §C2.4/C4。`ruviform-exec` `JobState.Cancelling` と整合 |
| `IdempotencyKey`/`OperationId`/`JobRef`/`OutputDigest`/`DefinitionDigest`/`InputDigest`/`Outcome` | **New（契約）** | design §C2.5 |
| Resource Budget・Non-RT・Grant/Trust/Job ownership 境界 | **New（契約）** | design §C2.6/C4 |
| CLI Entry/Result Encoding Adapter 境界 | **Consumer-only** | #155 確定。Synvelis `CliEnvelope`/Exit 0,2-10・AudioLens `al-cli` は各 Consumer |
| GPUI/GUI | **対象外** | Driver-neutral ViewSpec は別 lane |

## 6. 結論

- **契約語彙は採用、実装は Deferred（条件付き Go）**: Consumer-neutral orchestration の「形」を契約として文書化し、Ruviform 共通 Engine/CLI crate は作らない。
- **Synvelis は具象を先行**: `synvelis-automation`/`synvelis-cli` が AUTX-05/06/03 で具象 Owner のまま進める。generic 契約の adapter 適合は各 AUTX Issue の受入で確認。
- **AudioLens は consumer-only 継続**: 既存 stage-job モデルで充足。Workflow/Script 強制なし。AudioLens/第三 Consumer が独立した有界 Step orchestration の必要性を示した時点で再評価。
- **第二正本・Consumer 型流入・精度犠牲を設計段階で防ぐ**: generic は Principal/Grant/Approval/AdapterSeal/Trusted Issuer・`synvelis-*`/GPUI/AudioLens 製品型・独自 Grant Policy を型として持たず、Capability/Policy/Project/Job/Approval の第二正本を作らない。
