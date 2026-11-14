# RF558 Script/Analyzer 契約 — 2 Consumer 適用性判定ログ（Reuse/Extend/New/Consumer-only/Deferred）

- Issue: https://github.com/scarecrow173/ruviform/issues/558
- 作業 branch: `docs/558-script-contract`
- 作成日: 2026-11-14
- 対象スライス: **DESIGN-ONLY**（実装なし。適用性の判定のみ）
- 判定語彙: `Reuse`（既存契約/基盤をそのまま利用）/ `Extend`（既存契約の拡張が必要）/ `New`（新規契約/実装が必要）/ `Consumer-only`（Consumer 側責務。Ruviform 共通化対象外）/ `Deferred`（現時点で判定保留）

## 1. 判定原則

- **Ruviform の製品型非依存**: Synvelis Gateway/Project 型・AudioLens 型・GPUI 型・VM vendor 型を公開契約に含めない。
- **1 Consumer の成功 ≠ 共通採用**: Synvelis の Script 機能採用は AudioLens の採用を意味せず、逆も同様。両 Consumer の独立受入（#294）が必要。
- **非重複**: 既存 `ruviform-exec`/`ruviform-cli-sample`/Consumer Coordinator/Gateway/Job と責務を重複させない（#553 の材料）。
- **正直な未実測**: 未実行の共存・3OS・FFI は NotRun/Deferred。Pass に偽装しない。

## 2. Synvelis Consumer Scenario

Synvelis は AUTX-00（#259）で Action/Macro/Workflow/Script/Headless CLI の統合契約を所有し、SV-SCRIPT-01〜04（#271〜274）で Script lane を計画している。

| Scenario | Issue | 判定 | 根拠・境界 |
| --- | --- | --- | --- |
| **SV-SCRIPT-01 ScriptApiModel/型生成** | synvelis#271 | **Reuse（契約）+ Consumer-only（生成器）** | Ruviform は `ScriptValue`/`HostBinding`/`HostApiSurface` の中立記述子を提供（design C3/C6）。Gateway Capability→ScriptApiModel の型生成・Stub 生成は Synvelis 責務。Ruviform は製品型を出さない。 |
| **SV-SCRIPT-02 Action Script** | synvelis#272 | **Reuse（Runtime Port/Artifact）+ Consumer-only（Recipe lowering/Gateway 統合）** | `ScriptBackend`/`ScriptSession`/`ScriptArtifact{kind: ActionRecipe}` を消費。Action Recipe への lowering・Gateway 統合・Grant/Secret 注入は Synvelis。VM stack/coroutine 永続化は禁止（design C7）。 |
| **SV-SCRIPT-03 Workflow lowering** | synvelis#273 | **Reuse（Artifact=WorkflowDefinition）+ Consumer-only（PlanCompiler/Coordinator）** | YAML/Luau/Rhai が同一の型付き `WorkflowDefinition`/`Plan` へ lower される（design C7）。PlanCompiler は Synvelis #264、Coordinator/Run/Step FSM は #265。異種言語間は型付き参照のみ（FX-N-09）。 |
| **SV-SCRIPT-04 Editor/Analyzer** | synvelis#274 | **Reuse（ScriptAnalyzer Port）+ Consumer-only（GPUI Editor/ScriptApiModel stub）** | `ScriptAnalyzer`/`AnalyzerCapabilities`/`AnalyzeResult`/`Diagnostic`/`Completion`/`SourceRevision` を提供（design C5）。GPUI Editor・ScriptApiModel stub・補完 UI は Synvelis。Analyzer 成功は Grant を意味しない。 |
| **AUTX CLI（`action run --file`/`workflow run --file`）** | synvelis#259 | **Consumer-only/Deferred** | CLI は製品側責務。Ruviform は CLI framework を昇格しない（#155 先例: `ruviform-cli-sample` は利用例アダプタ）。Script を CLI から起動する統合は Synvelis 側で `ruviform-exec`/`ScriptSession` を駆動。 |
| **Gateway/Project/DAW 意味論** | synvelis#45/#46/#259 | **Consumer-only** | Grant/Secret/認可/Commit/Job は Synvelis Gateway/Project 責務。Ruviform 契約は `HostBinding` 記述子と `ResourceId` の中立表現のみを提供し、権限判定・製品型を含まない。 |

## 3. AudioLens Consumer Scenario

AudioLens #127 は Ruviform 互換の継続追従を要求するが、**Script 機能は現時点で要求しない**（snapshot 明記）。

| Scenario | Issue | 判定 | 根拠・境界 |
| --- | --- | --- | --- |
| **現行分析/CLI/export（Script なし）** | audio-lens#127 | **NotApplicable（現状維持）** | AL #127 は Script 導入を要求しない。Ruviform の Script 契約は文書のみで実装 crate を生まないため、AL の Cargo.lock/依存木に影響しない。 |
| **将来 Script 適用（headless CLI/f64/long-duration）** | audio-lens#127 | **Deferred** | Ruviform 側で Script crate/feature が昇格し AL の Cargo.lock/依存木に影響が出た時点で再評価。条件: headless CLI 互換・f64 精度（`ScriptValue::Number` は binary64 で AL の解析精度を損なわない）・long-duration Time/ResourceId 保持・GUI 非依存。 |
| **Script を AL へ強制適用** | — | **禁止** | AL #127 の独立性を尊重。Script は Synvelis の DAW/Workflow 文脈で必要な機能であり、AL の解析ツールには不要。1 Consumer の採用を他方へ強制しない。 |

## 4. Ruviform 既存基盤との境界（#553 非重複表の材料）

| 既存基盤 | 判定 | 境界 |
| --- | --- | --- |
| `ruviform-exec`（OfflineJob/OfflineDriver/CancellationToken） | **Reuse** | Script session を offline 駆動する既存 Executor。新規 Job/Project 正本を作らない。`ScriptSession` の配置（owner-thread/worker）に利用可能。 |
| `ruviform-cli-sample` | **Reuse（参考）** | #155 利用例アダプタ。CLI framework としての昇格はしない。Script CLI 起動は Consumer 側責務。 |
| `ruviform-io` | **NotApplicable（重複なし）** | Script 契約は中立面に IO/FS/Network/Shell binding を含めず、Host binding は Consumer 注入（design C6）。IO は Consumer/Host 側責務であり、Script 契約は IO 正本を複製しない。 |
| `ruviform-media-core`（TimeBase/MediaTime/ClockDomainId/StreamId/Fact） | **Reuse（意味論参照）** | `ScriptValue::Time`/`ResourceId` の意味論を参照。実装型の再利用は #559 で決定。 |
| Workflow Engine/Coordinator/Step/Checkpoint | **Consumer-only（Synvelis）/ Deferred（#553）** | Ruviform は Script 契約のみ。Workflow 実行正本は Synvelis #265（Coordinator）または #553 で共通化を判定。Script は Artifact/Definition を生成するのみで、実行 FSM を持たない。 |
| Job/Project/Grant/Secret 正本 | **Consumer-only** | Ruviform は独自認可/Job/Project 正本を持たない。Synvelis Gateway/Project・AL 側が所有。 |

## 5. 共通化対象/非対象（#553 への供給材料）

### 共通化対象（Ruviform が提供すべき中立契約）

- `ScriptBackendRegistry`/`ScriptBackend`/`ScriptSession`（Runtime Port）
- `ScriptAnalyzer`/`AnalyzerRegistry`（Analyzer Port、Runtime 独立）
- `ScriptValue`/`ScriptError`/`ScriptSource`/`ScriptArtifact`（値モデル・Artifact）
- `ExecutionBudget`/`ScriptCancel`（実行制御）
- `HostApiSurface`/`HostBinding`（Host API 記述子）
- Fail-Closed 判定表・Fault Matrix・Fixture（検証仕様）

### 共通化非対象（Consumer 責務または Deferred）

- ScriptApiModel/型生成（Synvelis #271）
- Action Recipe/Workflow Definition への lowering（Synvelis #272/#273）
- GPUI Editor・補完 UI（Synvelis #274）
- PlanCompiler/Coordinator/Run/Step FSM（Synvelis #264/#265）
- Gateway Capability/Grant/Secret/Commit/Job（Synvelis #45/#46）
- CLI framework・製品 CLI（両 Consumer）
- Workflow Engine/Job/Project 正本（#553 で判定）
- AudioLens への Script 適用（Deferred）
- 実装 crate・公開 API の Rust 確定（#559/#560 + #553 判定後）

## 6. 判定サマリ

| 区分 | 件数 | 内訳 |
| --- | --- | --- |
| Reuse（契約/基盤） | 4 | SV-SCRIPT-01〜04 の契約部分、`ruviform-exec`、`ruviform-media-core` 意味論 |
| Extend | 0 | 現時点で既存契約の拡張は不要（新規契約として設計） |
| New | 1 | 言語非依存 Script/Analyzer 契約自体（本スライスの成果物） |
| Consumer-only | 6 | SV-SCRIPT-01〜04 の製品側、AUTX CLI、Gateway/Project/DAW 意味論、Workflow Engine/Coordinator |
| Deferred | 4 | AL Script 適用、Send/Sync Session 形態、製品昇格/Rhai 本番採用、crate/API 確定 |
| NotApplicable | 1 | AL 現行 Script なし |
| NotRun | 3 | 実 Runtime 共存（#559）、Luau.Analysis FFI（#560）、両 Consumer 独立受入（#294） |

## 7. 製品型非依存の確認

- `ScriptValue`/`ScriptError`/`ScriptArtifact`/`AnalyzeResult` に Synvelis Gateway/Project 型・AudioLens 型・GPUI 型・VM vendor 型を含めない（design C3/C5/C6）。
- `ResourceId` は opaque `domain`+`kind`+`value` で製品型を中立化。VM/契約は解釈せず Host binding のみが意味を持つ。
- `HostApiSurface`/`HostBinding` は記述子のみ。OS/FS/Net/Shell の中立 binding を含めず、権限判定は Consumer 責務。
- Analyzer 成功は実行 Grant を意味しない。静的解析成功 ≠ 実行許可。
