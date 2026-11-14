# Issue #553 Workflow/CLI orchestration契約 — 文書監査・検証試行記録（2026-11-14）

`docs/tasks/issue-553-workflow-orchestration-task.md` の文書チェーン作成で試行した手法と結果の記録。コード変更を伴わないため、実験は「文書規約の適用と検証手法の選択」に限定される。正本の契約・判定は `docs/designs/issue-553-workflow-orchestration-design.md`・`docs/adr/issue-553-workflow-cli-contract.md`・`evidence/rf553/` を参照。

## 試行 1: 既存 `ruviform-exec`/`ruviform-cli-sample` API の棚卸し方法

- **目的**: Issue #553 の「現行Ruviform Exec/Job/CLI/IOと#155の実在APIを棚卸し」を、実在する公開 API に基づいて行う。
- **試行**: `crates/ruviform-exec/src/{job,offline,lib}.rs` と `crates/ruviform-cli-sample/src/{lib,main}.rs` の `pub fn`/`pub struct`/`pub enum`/`pub trait` を grep で抽出し、`docs/designs/issue-155-headless-cli-foundation.md` の設計結論（「新設 CLI フレームワークは方針により作らない」「sample は例・API に昇格しない」）と照合。
- **観測**: `ruviform-exec` は `CancellationToken`/`Progress`/`OfflineDriver`/`OfflineJob`（`JobState{Running,Cancelling,Finished}`・`JobOutcome`・`JobEvent`・`JobError`・`JobScope`）を提供するが、Plan/Step/Run journal/Resume/Idempotency/CLI Envelope は存在しない。`ruviform-cli-sample` は `probe`/`spawn_analyze`/`probe_json`/`report_json`/`error_json`/`event_json`/`PipelineError` の利用例であり、CLI framework ではない。
- **結論**: 棚卸し結果を design §C1 の対応表に採用。`ruviform-exec` の Job/Cancel primitive を Reuse、`ruviform-cli-sample` を Consumer-only（参照例）と判定。

## 試行 2: Synvelis `docs/automation/` と Issue スナップショットの境界抽出

- **目的**: AUTX-00〜08 の Owner/型/Cancel/Resume/Job 正本の境界を、generic 契約へ流入させない形で抽出する。
- **試行**: synvelis `docs/automation/{README,overview,spec,design-owner-dag,design-capability-action,design-atomic-macro,design-trust-boundary,design-workflow-run,design-script,plan,tasks}.md` と synvelis#259〜#267/#271〜#274 の Issue スナップショットを読み、`synvelis-gateway`/`synvelis-project`/`synvelis-automation`/`synvelis-cli` の各 Owner を特定。
- **観測**: Gateway が Capability/Policy/Grant/Approval/CommitGate/Project/Job の唯一正本。AUTX-05 が `WorkflowDefinition`/`PlanCompiler`/`ExecutablePlan`、AUTX-06 が Coordinator/Run journal/Resume/Cancel、AUTX-03 が `synvelis-cli`/`CliEnvelope`/Exit Code を Owner。Atomic Macro（≤64/非 Nested/1 CAS/1 Undo）と Workflow（非 RT orchestration）は別契約。Script は #558〜560 と分離。
- **結論**: Synvelis 固有型を generic 契約へ流入させず、「形」の語彙のみを design §C2 で固定。Synvelis 具象は consumer-only、generic 実装は Deferred と判定。

## 試行 3: AudioLens の代表 Scenario と適用性判定

- **目的**: AudioLens が既存 stage-job モデルで orchestration を充足するか、generic 契約の適用が必要かを判定する。
- **試行**: audio-lens#127/#189 と `crates/al-{cli,job,dataset,analysis,service,gui}` の構成を確認し、Stage Resume/Cancel・Manifest・f64/Dataset/Export の既存契約を特定。
- **観測**: `al-cli`/`al-job`/`al-dataset`/`al-analysis` で Stage Resume/Cancel・Manifest・f64 解析・Portable Export を既に持ち、分岐・非同期 Job 待機・外部 I/O 混在・Condition を持たない。Script 導入も明示的に非要求。
- **結論**: AudioLens は consumer-only 継続。generic `OrchestrationDefinition` の語彙で「versioned bounded plan + step refs + run cursor」に写像可能だが、現要件では新規依存を正当化しない。片側成功での共通化採用を防ぐため、AudioLens/第三 Consumer の独立した必要性を再評価 Trigger にする。

## 試行 4: 文書チェーンの front matter・relation 規約適用

- **目的**: #558 の文書チェーン（spec/design/ADR/test-spec/plan/task/IR/exp）と同じ規約で #553 の文書を作成する。
- **試行**: `docs/{specs,designs,adr,test-specs,plans,tasks,impl/ir,impl/exp}/issue-558-*.md` の front matter・relation・status を参照し、#553 用に `SPEC-`/`DESIGN-`/`ADR-`/`TSPEC-`/`PLAN-`/`TASK-`/`IMPL-` の ID を採番。
- **観測**: status は spec=proposed・design=approved・adr=accepted・test-spec=proposed・plan=completed・task=done・ir=completed。relation は `implements`/`depends-on`/`derives-from`/`verified-by`/`references`/`changes` を相互に整合させる。`docs/impl/exp` は `.md` の自由形式記録を許容（#192 等の先例）。
- **結論**: #553 の文書チェーンを同規約で作成。`doc-maintenance index apply` で管理 index を再生成し `in-sync` を確認。
