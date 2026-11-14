# RF553 Workflow/CLI orchestration契約 — 受入条件→契約条項→根拠 対応表

- Issue: https://github.com/scarecrow173/ruviform/issues/553
- 作業 branch: `docs/553-workflow-orchestration`
- 作成日: 2026-11-14
- 対象スライス: **DESIGN-ONLY**（実装 crate・公開 API・PoC コードは生成しない）
- 判定語彙: `POLICIES.md` §2/§3（要件状態 Required/Deferred/NotApplicable、実行結果 Pass/Fail/NotRun/Unsupported）

## 1. 受入条件 → 契約条項 → 根拠

Issue #553 の 5 つの受入条件を、`docs/designs/issue-553-workflow-orchestration-design.md`（以下 design）の契約条項へ対応付ける。

### AC-1: 現行Ruviform Exec/Job/CLI/IOと#155の実在APIを棚卸しし、Workflow/Action/Scriptの責務が#558〜560と重複しない対応表を作成

| 契約条項 | design 節 | 内容 |
| --- | --- | --- |
| 既存資産の棚卸しと判定 | C1 | `ruviform-exec`（`CancellationToken`/`Progress`/`OfflineDriver`/`OfflineJob`/`JobState`/`JobOutcome`/`JobEvent`/`JobError`/`JobScope`）を Reuse、`ruviform-cli-sample`（`probe`/`spawn_analyze`/`*_json`/`PipelineError`）を Consumer-only（参照例）、#155 headless 分離方針を Reuse（方針）、`ruviform-media-core` を条件付き Reuse と判定 |
| Synvelis AUTX 境界 | C1 | `synvelis-gateway`（`CapabilityRef`/`CapabilityDescriptor`/`SchemaRegistry`/`PolicyEvaluator`/Grant/Approval/`CommitGate`/`ProjectController`/`JobPort`/`GatewayOutcome`/`ErrorEnvelope`/`AdapterSeal`/`CallerIdentity`）と `synvelis-automation`（`ActionPresentation`/`AtomicMacroDefinition`/`WorkflowDefinition`/`PlanCompiler`/`ExecutablePlan`/`WorkflowRun`/`StepCheckpoint`）、`synvelis-cli`（`CliEnvelope`/Exit 0,2-10）を Consumer-only と判定 |
| AudioLens 境界 | C1 | `al-cli`/`al-job`/`al-dataset`/`al-analysis`/`al-service` を Consumer-only と判定 |
| Script 非重複 | C1 | rf#558〜560（`ScriptBackend`/`ScriptSession`/`ScriptValue`/`ScriptArtifact`/`ScriptAnalyzer`/`ExecutionBudget`/`ScriptCancel`）を分離（非重複）と判定。Script が生成する Definition は Consumer の Plan Compiler へ lower される入力であり、orchestration 契約の一部ではない |

**根拠**: `crates/ruviform-exec/src/{job,offline,lib}.rs`・`crates/ruviform-cli-sample/src/{lib,main}.rs` の公開 API grep、`docs/designs/issue-155-headless-cli-foundation.md`（「新設 CLI フレームワークは方針により作らない」「sample は例・API に昇格しない」）、synvelis `docs/automation/`（PR #420 マージ済み）と synvelis#259〜#267/#271〜#274、audio-lens#127/#189、rf#558〜560 Issue/文書。

### AC-2: Synvelis Action/Workflow/HeadlessとAudioLens分析/CLI/Exportの代表Scenario各1件以上で、同一Plan/Step/Cancel/Resume/Budget/Receiptが通用する正例・反例を記録

| 契約条項 | design 節 | Scenario |
| --- | --- | --- |
| Synvelis 正例 | C3 | `synvelis-cli workflow run --file render-chain.yaml --project mix.svproj`: `Query→AtomicMacro→JobSubmit→AwaitJob→Result→If/BoundedLoop` を generic 語彙で表現可能。`CallRef` は `synvelis.operation.*` CapabilityRef、`StepExecutionPort` は Gateway/JobPort adapter、journal は `synvelis-automation` Coordinator が所有 |
| Synvelis 反例 | C3 | Plan に `principal`/`grant`/`approval`/`os_credential` を serialize、VM stack/coroutine を checkpoint に保存、Workflow 全体を単一 atomic transaction として扱う、Script 生成 Definition を Plan Compiler を通さず VM 内で直接 Gateway invoke → いずれも契約違反として reject |
| AudioLens 正例 | C3 | `al-cli run --resume <manifest>` の stage 列（decode→analyze→export）。既存 `al-job` Stage resume/cancel・Manifest・f64/Dataset が充足。generic 語彙で「versioned bounded plan + step refs + run cursor」に写像可能だが現要件では新規依存を正当化せず consumer-only 継続 |
| AudioLens 反例 | C3 | generic 契約が `CapabilityRef`/`Grant`/`Project Revision`/`GPUI`/Synvelis 型を要求、f64/NaN/gap/PTS/長尺/Export semantics を generic が変更・制約、Script Workflow を強制導入 → いずれも拒否 |
| Cancel/Resume 反例 | C4 | Blocking provider work 中の Cancel（cooperative/indeterminate）、CLI disconnect/Window close（ownership は Consumer に残る）、Commit/Job 完了後 Journal 前 Crash（`operation_id`/`job_ref`/`idempotency_key` で再照会し二重 side effect 0）、Grant revoke/expiry/policy epoch 変更（次 Step で Consumer が再評価）、外部副作用（generic は atomicity を保証しない） |

**根拠**: synvelis#262（CLI `CliEnvelope`/Exit Code/`--file`）、#264（`WorkflowDefinition`/`PlanCompiler`/`ExecutablePlan`）、#265（Coordinator/Run journal/Resume/Cancel/Indeterminate）、#263（Atomic Macro ≤64/非 Nested/1 CAS/1 Undo）、audio-lens#127（Stage Resume/Cancel/Manifest/f64/Export）、#189（旧 WebView 撤去・CLI/headless 分離）。

### AC-3: Generic Portの公開型・Dependency/NonRT/Grant/Trust/Job ownership境界と、Consumer型の流入禁止、両Consumerの精度/性能回帰基準を固定

| 契約条項 | design 節 | 内容 |
| --- | --- | --- |
| 公開型（contract-level） | C2.1〜C2.6 | `OrchestrationDefinition`/`PlanLimits`/`PlanValidator`/`StepExecutionPort`/`OrchestrationRun`/`StepCheckpoint`/`IdempotencyKey`/`OperationId`/`JobRef`/`OutputDigest`/`DefinitionDigest`/`InputDigest`/`Outcome`/`CliEnvelope` 相当の結果 encoding の形 |
| Dependency/NonRT | C2.6 | transport-neutral（in-process/CLI/IPC/MCP を区別しない）。GUI/GPUI・OS Process/FS/Network/Shell の型を公開面に出さない。Plan 解釈・Step dispatch・journal・Resume は `non-RT only` |
| Grant/Trust | C2.6/C4 | generic 契約は Principal/Grant/Approval/AdapterSeal/Trusted Issuer を型として持たない。認可は Consumer の Gateway/Policy が各 effect/commit boundary で再評価。analyzer/plan 検証の成功は実行権限を意味しない |
| Job ownership | C2.4/C2.6 | Workflow Run を `JobKind`/`Job` と偽装しない。子 Job の状態・Cancel 真偽は Consumer の JobPort/Provider が正本 |
| Consumer 型流入禁止 | C2.6 | `synvelis-project`/`synvelis-gateway`（`CapabilityRef`・`Grant`/`Approval`/`Effect`/`CommitGate`・`JobPort`・`Composite`/`CAS`/`Undo`）・GPUI・AudioLens 製品型・独自 Grant Policy へ直接依存しない。公開 enum は `#[non_exhaustive]` |
| 精度/性能回帰基準 | C3/C4 | AudioLens f64/NaN/gap/PTS/長尺/Export semantics・Synvelis RT/DAW semantics を generic orchestration が変更・制約しない。片側 Consumer の成功だけで共通化を採用しない |

**根拠**: `docs/adr/public-enum-non-exhaustive-policy.md`、`docs/designs/ruviform-architecture-design-baseline.md`（crate 依存方向・RT/非 RT 分離）、synvelis `docs/automation/design-owner-dag.md`（第二正本 0 件・Owner/型境界）、audio-lens#127（f64/NaN/gap/PTS/長尺/CLI/Export の独立 Oracle）。

### AC-4: Reuse/Extend/New/Consumer-only/Deferred別に最小実装責務と必要な後続Issueを決裁し、共通化不能の場合はNo-go理由と再評価Triggerを記録

| 契約条項 | design/ADR 節 | 内容 |
| --- | --- | --- |
| Candidate 分類 | ADR「Candidate 分類」 | `ruviform-exec` Job/Cancel primitive=Reuse、`ruviform-cli-sample`=Consumer-only（参照例）、#155 headless 方針=Reuse（方針）、`ruviform-media-core`=Reuse（条件付き）、Versioned Bounded Plan/`PlanValidator`/`StepExecutionPort`/`RunState`/`Checkpoint`/`RecoveryCursor`/`IdempotencyRef`/`Outcome`/`Digest`/Budget/Non-RT/Grant/Trust/Job ownership/CLI Result 境界=New（契約）/実装 Deferred、CLI Entry/Result Encoding Adapter=Consumer-only、Synvelis Gateway/automation/cli=Consumer-only、AudioLens al-*=Consumer-only、Script VM/Analyzer=分離（rf#558〜560）、GPUI=対象外 |
| 最小実装責務 | ADR「Candidate 分類」 | 各候補の Owner（Ruviform 既存/Consumer/Synvelis AUTX lane/AudioLens 追従/rf#558〜560）を明記 |
| 後続 Issue 候補 | ADR「後続 Issue 候補」 | RF-AUTX-02 候補（Deferred・再評価 Trigger 成立後）、Synvelis #28/#259〜#267（具象 Owner）、AudioLens #127/#189（consumer-only 継続） |
| 再評価 Trigger | ADR「再評価 Trigger」 | (1) Synvelis AUTX-05/06/03 具象実装完了で generic 契約の「形」が安定、(2) AudioLens/第三 Consumer が独立した有界 Step orchestration の必要性を具体的 Scenario で示す、の 2 条件が揃った時点で `ruviform-orchestration` 仮称 crate の新設可否を別 Issue で判断 |
| 共通化不能の場合 | ADR「Decision Outcome」 | 本件は「契約語彙は採用・実装は Deferred（条件付き Go）」であり完全 No-go ではないが、実装共通化は現時点で Deferred。No-go 相当の判断（AudioLens への強制・片側成功での採用）は consumer-only 継続と再評価 Trigger で記録 |

**根拠**: `docs/adr/issue-553-workflow-cli-contract.md` の Option A/B/C 比較と Decision Outcome、`evidence/rf553/consumer-applicability.md` の 2 Consumer 判定ログ。

### AC-5: 本設計だけでWorkflow Engine実装・両Consumer移行・OS Native実走がPassしたとは主張しない

| 契約条項 | design/ADR 節 | 内容 |
| --- | --- | --- |
| DESIGN-ONLY 明示 | spec「Scope」/design「Context」/ADR「Decision Drivers」 | 実装 crate・公開 API・PoC コードは生成しない。Synvelis/AudioLens の具象 Owner は各 Consumer。OS Native 実走・実 Job/Resume/Cancel は各 Consumer の E2E Gate（Synvelis #267、AudioLens #127/#189）が Owner |
| NotRun/Unsupported 記録 | `evidence/rf553/verification.md` | Runtime/Consumer/OS 実機は NotRun（コード変更なし・文書契約のみ）。`mise run check`/`test`/`test-rt`/`test-ci-consumer-verify` は文書契約の照合であり runtime Pass ではない |

**根拠**: spec「Non-goals」、design「Verification notes」、`evidence/rf553/verification.md`。

## 2. 非重複対応表（Workflow/Action/Script vs #558〜560）

| 責務 | #553（本契約） | #558〜560（Script） | Synvelis AUTX | AudioLens |
| --- | --- | --- | --- | --- |
| Workflow 計画（versioned bounded plan） | 契約語彙のみ（design §C2.1） | 対象外（Script が生成する Definition の**出力先**） | AUTX-05 `synvelis-automation` が具象 Owner | consumer-only（既存 stage-job） |
| 有界 Step 実行 | 契約語彙のみ（design §C2.3） | 対象外 | AUTX-06 Coordinator が具象 Owner | consumer-only |
| Cancel/Resume | 契約語彙のみ（design §C2.4/C4） | 対象外 | AUTX-06 が具象 Owner | `al-job` Stage resume/cancel が既存 |
| CLI Port | 契約語彙のみ（design §C2.6、CLI Entry は Consumer 所有） | 対象外 | AUTX-03 `synvelis-cli`/`CliEnvelope`/Exit Code | `al-cli` が既存 |
| Script VM/Analyzer | 分離（非重複） | rf#558〜560 が Owner | SV-SCRIPT-01〜04 が具象 Owner | 非要求 |
| Capability/Grant/Approval/Project/Job | 対象外（Consumer-only） | 対象外 | `synvelis-gateway`/`synvelis-project` が唯一正本 | `al-*` が Consumer-only |

**根拠**: rf#558〜560 Issue/文書、synvelis `docs/automation/`、audio-lens#127。
