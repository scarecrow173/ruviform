---
id: "TSPEC-034XecRP3Wpuq0BscTS283"
type: "test-spec"
status: "proposed"
title: "Multi-backend Script/Analyzer契約検証（Issue #558 RF-SCRIPT-00）"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/558"
  changes:
    added:
      - type: "doc"
        path: "docs/test-specs/issue-558-script-contract.md"
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
    - "docs/designs/issue-558-script-contract-design.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-558-script-contract-design.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-558-script-contract.md"
    - "docs/designs/issue-558-script-contract-design.md"
    - "docs/adr/issue-558-script-backend-contract.md"
  verified-by: []
  references:
    - "docs/designs/issue-558-script-contract-design.md"
    - "evidence/rf558/verification.md"
  defers: []
  deferred-by: []
---

# Multi-backend Script/Analyzer契約検証（Issue #558 RF-SCRIPT-00）

## Purpose

`docs/designs/issue-558-script-contract-design.md` が定義する言語非依存 Script/Analyzer 契約の各条項が、将来の #559/#560 PoC・Consumer 統合で「検証可能な保証」として保持されることを記録する。契約が変更・再設計されても、ここに列挙する Rule/Guarantee が黙って失われないことを目的とする。実装が存在しない現時点では実行テストはなく、Rule/Example は将来の検証タスクが従うべき仕様である。実装が契約と乖離した場合、または契約自体が supersede された場合に本 test-spec を見直す。

## Feature

Multi-backend Script Runtime/Analyzer の言語非依存契約が、Fail-Closed 解決・値モデル完全性・Artifact 版管理・Analyzer 独立性・Budget/Cancel/Session 隔離・RT 禁止・Security/Grant 境界を満たすこと。

## Rules

### R-REG: Registry/Dispatch は Fail-Closed

- Rule: `ScriptBackendRegistry::resolve` は、未知 `ScriptLanguageId`/`ScriptBackendId`・要件不一致・`contract_version` 非互換・必須 Capability 未対応・複数候補の曖昧性を、型付き `ResolveError` で拒否する。暗黙フォールバック・最近似 Backend 選択・Feature による実行時排他を行わない。
- Rule: 同一 `ScriptBackendId` の重複登録は `RegistryError::DuplicateBackend` で拒否する。
- Rule: 同一プロセス内で複数 Backend を同時登録でき、`resolve` が正しく Dispatch する（#559 で実測）。

### R-SES: Session は隔離され配置選択可能

- Rule: `ScriptSession` は無条件 Send/Sync を要求しない。配置特性（owner-thread/Send可能等）は `BackendCapabilities` が広告し、Consumer が選択する。
- Rule: Session 間で VM 可変状態を共有しない（`shared_vm_state` capability 明示時を除く）。
- Rule: `cancel()` は冪等・協調的。ブロッキング Host 関数中の即時中断は保証しない（`cancel_granularity` で広告）。
- Rule: 全 Script API は non-RT。RT パスからの呼出しは契約違反。

### R-VAL: ScriptValue は完全性を保持

- Rule: `ScriptValue` は Absent と Null を区別する。混同は禁止。
- Rule: `Integer(i64)` は厳密。表現不能な整数は `ValueOutOfRange` で拒否し、暗黙丸めしない。
- Rule: `Time{domain,timebase,count}` と `ResourceId{domain,kind,value}` を保持し、暗黙のドメイン変換・単位丸めをしない。
- Rule: VM テーブル/関数/userdata/coroutine/thread・ネイティブポインタ・Grant/Secret/Token・製品型（Synvelis/AudioLens/GPUI）・Rust `Any`/trait object を `ScriptValue` 越境に含めない。

### R-ART: ScriptArtifact は版管理・canonical・安全

- Rule: `ScriptArtifact` は `schema_version`/`kind`/`digest`/`payload`/`source`/`produced_by` を持つ。
- Rule: `payload` は canonical form でシリアライズ可能。同一意味の定義は Luau/Rhai いずれからも同一 `DefinitionDigest` を生成する。
- Rule: 未知 `schema_version` は `UnsupportedSchemaVersion` で Fail Closed。
- Rule: `payload` に VM 状態（stack/coroutine/table/function）・Grant/Secret/Token を含めない。
- Rule: Source `digest` 不一致（再読込/Resume）は `SourceDigestMismatch` で Fail Closed。

### R-ANA: Analyzer は Runtime から独立

- Rule: `ScriptAnalyzer`/`AnalyzerRegistry` は `ScriptBackendRegistry` とは別 Registry/Port。
- Rule: `AnalyzerCapabilities` で未実装能力（Diagnostics/Completion/型検査深度）を `Unsupported` で返し、成功を偽装しない。
- Rule: `AnalyzeResult`/`Diagnostic`/`Completion` は `source_revision` を保持し、stale 結果は Consumer が拒否する。
- Rule: Analyzer 成功は実行 Grant を意味しない。静的解析成功 ≠ 実行許可。
- Rule: Analyzer は Runtime Backend なしで利用可能。逆も同様。

### R-BUD: Budget/Cancel は強制可能・正直

- Rule: `ExecutionBudget` の各項目は `Option`。要求された Budget 種別が Backend 未対応なら `open_session` 時に `SessionError::UnsupportedBudget` で Fail Closed。
- Rule: Budget 超過（`max_instructions`/`max_memory_bytes`/`max_output_bytes` 等）は `ScriptError::BudgetExceeded{kind}` で session を終了する。
- Rule: `cancel()` は冪等。Cancel は協調的で safepoint で観測される。

### R-SEC: Host/Security/Grant 境界

- Rule: 中立契約に OS/FS/Network/Shell の Host binding を含めない。Consumer が `HostApiSurface` として注入する。
- Rule: 各 Host 呼出しは Consumer が実行時に再認可する。`effect`（Pure/Query/Effect）は助言情報であり権限根拠にしない。
- Rule: Artifact/定義文書に Grant/Secret/Token を含めない（`ArtifactError::ForbiddenField`）。
- Rule: Script は任意 Shell/Network/FS を公開面から呼び出せない。

### R-WF: Workflow/Artifact は型付き文書へ収束

- Rule: Script が生成する `ScriptArtifact`（`ActionRecipe`/`WorkflowDefinition`）は、Consumer の PlanCompiler/Coordinator が消費する型付き文書。
- Rule: 異種言語間の参照は型付き Step/Definition 参照経由。VM 直接呼出し・VM stack/coroutine 永続化を禁止。
- Rule: 定義内の循環参照・未知参照・異種 VM direct reference は Consumer Plan 検証で `CycleDetected`/`UnknownReference`/`CrossVmReference` として Fail Closed。
- Rule: Ruviform は Workflow Engine/Coordinator/Job の正本を持たない。`ruviform-exec` を再利用し、新規 Job/Project 正本を作らない。

### R-DOC: 文書チェーンと証拠の整合

- Rule: spec→design→ADR→test-spec→plan→task→IR/exp の front matter・relation・管理 index が `doc-status`/`audit_docs`/`audit_adr`/`audit_impl_record` を通過する。
- Rule: 未実行・未実測項目（実 Runtime 共存・3OS・native FFI・Rhai 本番採用・AL Script 適用）は `evidence/rf558/` で NotRun/Deferred と明記し、Pass に偽装しない。

## Examples

### 正例

- Example FX-P-01（Luau Action Recipe）: Given Luau Backend が登録済み, When `evaluate` が Action Recipe を生成, Then `ScriptArtifact{kind: ActionRecipe, schema_version, digest, payload, source, produced_by}` が返る。
- Example FX-P-02（Rhai 同一意味 Action Recipe）: Given Rhai Backend が登録済み, When 同一意味の Action Recipe を生成, Then FX-P-01 と同一 `DefinitionDigest` が得られる。
- Example FX-P-03（同一意味 Workflow Definition）: Given Luau/Rhai Backend, When 同一意味の Workflow Definition を生成, Then 同一 digest。異種言語間は型付き参照のみ。
- Example FX-P-04（混合言語 Workflow）: Given Luau Action + Rhai Action, When 型付き Step/Definition 参照で結合, Then VM 直接参照なしに PlanCompiler が受理する。
- Example FX-P-05（Absent/Null）: Given 値モデル往復, When Absent と Null を含む値を変換, Then Absent≠Null が保持される。
- Example FX-P-06（i64/Time/ID）: Given i64 精度・Time/ResourceId を含む値, When `ScriptValue` を往復, Then 値が損失なく保持される。
- Example FX-P-07（同時登録・実行時選択）: Given Luau+Rhai Backend を同一プロセスに登録, When `resolve` で言語/Backend を指定, Then 正しく Dispatch される（#559 で実測）。
- Example FX-P-08（Analyzer Diagnostics/Completion）: Given Analyzer 登録済み, When `analyze` を実行, Then `source_revision` 付きの `AnalyzeResult` が返る。

### 反例（Fail Closed）

- Example FX-N-01: Given `"python"` 等の未知 `ScriptLanguageId`, When `resolve`, Then `NoMatchingBackend{UnknownLanguage}`。
- Example FX-N-02: Given 未知 `ScriptBackendId`, When `resolve`, Then `NoMatchingBackend{UnknownBackend}`。
- Example FX-N-03: Given 登録 Backend なし, When `resolve`, Then `NoMatchingBackend`。
- Example FX-N-04: Given 同一言語に 2 Backend で `backend` 未指定, When `resolve`, Then `AmbiguousBackend`。
- Example FX-N-05: Given `contract_version` 非互換 Backend, When `resolve`, Then `NoMatchingBackend{UnsupportedContractVersion}`。
- Example FX-N-06: Given `max_memory_bytes` 強制を要求するが Backend 未対応, When `open_session`, Then `SessionError::UnsupportedBudget`。
- Example FX-N-07: Given 未知 `schema_version` の Artifact, When Consumer 検証, Then `UnsupportedSchemaVersion`。
- Example FX-N-08: Given Source digest 不一致で Resume, When Session 開始, Then `SourceDigestMismatch`。
- Example FX-N-09: Given 定義内に異種 VM への直接参照, When Plan 検証, Then `CrossVmReference`。
- Example FX-N-10: Given Workflow Definition 内の循環参照, When Plan 検証, Then `CycleDetected`。
- Example FX-N-11: Given 未知 Step/Capability 参照, When Plan 検証, Then `UnknownReference`。
- Example FX-N-12: Given i64 非表現可能な整数を `Integer` へ, When `ScriptValue` 変換, Then `ValueOutOfRange`。
- Example FX-N-13: Given Analyzer が Completion 未実装で要求, When `analyze`, Then `Unsupported{completion}`。
- Example FX-N-14: Given 古い `source_revision` の Analyzer 結果, When Consumer 検証, Then `StaleResult` として拒否。
- Example FX-N-15: Given RT コールバックから `evaluate`, When 呼出し, Then non-RT 保証違反を検出。
- Example FX-N-16: Given `shared_vm_state` 未広告での Session 間状態共有, When 検証, Then `IsolationViolation`。
- Example FX-N-17: Given Artifact payload に Grant/Secret/Token, When 検証, Then `ForbiddenField`。
- Example FX-N-18: Given HostApiSurface に OS/FS/Net/Shell binding, When Consumer 注入, Then Consumer 側で拒否。
- Example FX-N-19: Given ブロッキング Host 関数中の `cancel()`, When 実行, Then 即時中断は保証せず結果破棄+Session close。
- Example FX-N-20: Given `max_instructions` 超過の無限ループ, When `evaluate`, Then `BudgetExceeded{max_instructions}`。
- Example FX-N-21: Given `max_memory_bytes` 超過, When `evaluate`, Then `BudgetExceeded{max_memory_bytes}`。
- Example FX-N-22: Given `max_source_bytes` 超過の巨大ソース, When `open_session`, Then Parse/Compile エラー。
- Example FX-N-23: Given 敵対的入力（深いネスト・爆弾）, When `evaluate`, Then BudgetExceeded/Cancelled で終了。
- Example FX-N-24: Given Analyzer 未登録で Runtime Backend のみ存在, When `analyze` を要求, Then `AnalyzerError::Unavailable`（Backend 解決とは独立に Fail Closed）。

## Fail-Closed / Fault Matrix 写像

design `docs/designs/issue-558-script-contract-design.md` の Fail-Closed 判定表（F-01〜F-18）と Fault Matrix（M-01〜M-13）を、本 test-spec の Rule/Example へ写像する。Issue #558 が要求する「機械検査可能な境界/試験マトリクス」として、各条件が型付きエラーまたは契約違反で Fail Closed することを保証する。

| design 条項 | 本 test-spec Rule | Example/Fixture |
| --- | --- | --- |
| F-01〜F-04（未知/曖昧/0件/複数候補） | R-REG | FX-N-01〜N-04 |
| F-05〜F-06（Version/Capability 不一致） | R-REG・R-BUD | FX-N-05・N-06 |
| F-07（未知 schema_version） | R-ART | FX-N-07 |
| F-08（Source digest 不一致） | R-ART | FX-N-08 |
| F-09（異種 VM direct reference） | R-WF | FX-N-09 |
| F-10〜F-11（cycle/未知参照） | R-WF | FX-N-10・N-11 |
| F-12（i64 非表現） | R-VAL | FX-N-12 |
| F-13（Analyzer Capability 未実装） | R-ANA | FX-N-13 |
| F-14（stale source_revision） | R-ANA | FX-N-14 |
| F-15（RT 呼出し） | R-SES | FX-N-15 |
| F-16（Session 状態漏洩） | R-SES | FX-N-16 |
| F-17（Grant/Secret 混入） | R-SEC | FX-N-17 |
| F-18（Analyzer 未登録/利用不可） | R-ANA | FX-N-24 |
| M-01（Sandbox/OS API 不正公開） | R-SEC | FX-N-18 |
| M-02/M-04（Host 中断不能・Cancel 中ブロッキング） | R-SES・R-BUD | FX-N-19 |
| M-03/M-09（Budget/Quota 超過） | R-BUD | FX-N-20・N-21 |
| M-05（Session cross-talk） | R-SES | FX-N-16 |
| M-06（RT/allocating/blocking） | R-SES | FX-N-15 |
| M-07（Analyzer crash/timeout/FFI） | R-ANA | Deferred（#560） |
| M-08（VM panic/abort） | — | Deferred（#559、プロセス分離は Consumer 責務） |
| M-10〜M-11（巨大/敵対的入力） | R-BUD | FX-N-22・N-23 |
| M-12（digest 衝突/改竄） | R-ART | FX-N-08 |
| M-13（Version drift） | R-ART・R-ANA | FX-N-05・N-07・N-14 |

## Guarantees

- 正しい実装は、全 Rule を満たし、全 Fixture（FX-P-01〜P-08, FX-N-01〜N-24）を検証可能な形で保持する。
- `resolve`/`register`/`open_session`/`evaluate`/`analyze` の全失敗経路が型付きエラーで Fail Closed し、暗黙フォールバックがない。
- `ScriptValue`/`ScriptError`/`ScriptArtifact`/`AnalyzeResult` の全公開型が VM vendor 型・製品型（Synvelis/AudioLens/GPUI/Plugin SDK）・Grant/Secret・OS ハンドルを含まない。
- Analyzer と Runtime が独立して登録・利用でき、Analyzer 成功が実行 Grant を意味しない。
- 未実行項目が `evidence/rf558/` で NotRun/Deferred と明記され、Pass に偽装されない。

## Non-goals

- 実装コード・crate・公開 API の Rust シグネチャの検証（#559/#560 + #553 判定後）。
- 実 Runtime 共存・3OS Build/Run・native FFI の実測（#559/#560 受入）。
- 共通 API の製品昇格・Rhai 本番採用・Workflow Engine/CLI 共通化の検証（#553）。
- Consumer 側の ScriptApiModel 生成・GPUI Editor・PlanCompiler/Coordinator の検証（Synvelis 責務）。
- Lua 5.4・JIT/性能比較・VM stack/coroutine 永続化・分散 Script 実行の検証。
- AudioLens への Script 適用の検証（現時点 Deferred）。

## Risk

- 契約が変更・再設計された場合、本 test-spec の Rule/Guarantee が黙って失われるリスク → spec/design/ADR の supersede 時に本書を見直す。
- Fixture が実装都合で弱められるリスク → `evidence/rf558/contract-trace.md` で受入条件→契約条項→Fixture の対応を固定し、変更時に差分を確認する。
- 未実測項目を Pass と誤認するリスク → `evidence/rf558/verification.md` に実行コマンド・OS・toolchain・exit code を記録し、未実行は NotRun/Deferred と明記する。
