---
id: "DESIGN-034XeaFjxZePL0m6LYuEeS"
type: "design"
status: "approved"
title: "Multi-backend Script/Analyzer言語非依存契約設計（Issue #558 RF-SCRIPT-00）"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/558"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-558-script-contract-design.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-558-script-contract.md"
  implemented-by: []
  depends-on:
    - "docs/adr/issue-558-script-backend-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-558-script-contract.md"
    - "docs/adr/issue-558-script-backend-contract.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-558-script-contract.md"
  references:
    - "docs/adr/issue-558-script-backend-contract.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
    - "docs/designs/issue-155-headless-cli-foundation.md"
    - "evidence/rf558/contract-trace.md"
    - "evidence/rf558/consumer-applicability.md"
  defers: []
  deferred-by: []
---

# Multi-backend Script/Analyzer言語非依存契約設計（Issue #558 RF-SCRIPT-00）

本書は Issue #558 の DESIGN-ONLY 契約を定義する。実装 crate・公開 Rust API・PoC コードは含まず、#559/#560 の実装が従うべき型・意味論・Fail-Closed 規則・Fixture 割当を固定する。

## Context

- 上位契約: `docs/specs/issue-558-script-contract.md`（本設計の `implements` 元）。
- 決定記録: `docs/adr/issue-558-script-backend-contract.md`（Option C: 言語非依存 Port + 実行時 Registry/Dispatch を採用）。
- アーキテクチャ正本: `docs/designs/ruviform-architecture-design-baseline.md`（レイヤー分離・RT 境界）、`docs/designs/shared-foundations-and-modularization-policy.md`（共通化/Consumer 判定の枠組み）。
- Consumer 要件: Synvelis #259/#271〜274（AUTX-00・SV-SCRIPT-01〜04）、AudioLens #127（Script 非要求の明記）。Ruviform #553/#219/#294（共通化判断・両 Consumer 平等・独立回帰）。
- 既存境界: `ruviform-exec`（OfflineJob/OfflineDriver/CancellationToken — Job/Executor 正本ではない）、`ruviform-cli-sample`（#155 利用例アダプタ）、`ruviform-media-core` の TimeBase/MediaTime/ClockDomainId/StreamId（Time/ResourceId 意味論の参照元）。

## Scope

### In Scope

- 公開型の契約レベル定義（Rust シグネチャではなく型・不変条件・エラー分類）。
- Registry/Dispatch・Session/Value/Error/Artifact・Budget/Cancel・Analyzer Port の意味論。
- Fail-Closed 判定表、Fault Matrix、設計 Fixture（正例・反例）の割当。
- 2 Consumer 適用性の判定サマリ（詳細は `evidence/rf558/consumer-applicability.md`）。

### Out of Scope

- 実装コード・crate 配置・公開 API の Rust 確定（#559/#560 + #553 判定後）。
- 実 Runtime 共存・3OS Build/Run・native FFI の実測（#559/#560 受入）。
- 共通 API の製品昇格・Rhai 本番採用・Workflow Engine/CLI 共通化実装（#553）。
- Consumer 側の ScriptApiModel 生成・GPUI Editor・PlanCompiler/Coordinator 実装。

## Components and Boundaries

### C1. 識別子と Registry（`ScriptBackendRegistry`）

| 型 | 役割 | 契約上の不変条件 |
| --- | --- | --- |
| `ScriptLanguageId` | 言語識別子（例: `"luau"`, `"rhai"`） | 不透明な文字列 newtype。小文字・言語名で一意。`"lua"`/`"lua54"` は別 ID で、Luau とは混同しない。 |
| `ScriptBackendId` | Backend 実装識別子（例: `"luau-mlua"`, `"rhai-embedded"`） | 不透明な文字列 newtype。同一 BackendId の重複登録は `RegistryError::DuplicateBackend` で拒否。 |
| `ScriptBackendVersion` | Backend 実装バージョン + 契約互換版 | `impl_version`（semver/opaque）と `contract_version`（本契約の版）を分離。`contract_version` 不一致は Fail Closed。 |
| `BackendCapabilities` | Backend が広告する能力集合 | 値種別対応・Budget 強制種別・Session 配置特性（owner-thread/Send可能等）・最大 source サイズ・生成可能 Artifact 種別を含む。未広告の能力を要求された場合は Fail Closed。 |
| `ScriptBackendRegistry` | プロセス内 Backend 登録・実行時解決 | `register`/`resolve`/`list` を提供。同一プロセス内で複数 Backend を同時登録可能。**Feature 非排他**: 契約は共存を前提とし、feature はバイナリ同梱の有無を決めるだけで実行時選択を排他にしない。 |

#### Dispatch 規則（`resolve` の Fail-Closed）

入力: `ScriptRequirement { language: Option<ScriptLanguageId>, backend: Option<ScriptBackendId>, required_capabilities: CapabilitySet, min_contract_version, artifact_kinds: ArtifactKindSet }` + `ScriptSource`（digest 含む）。

解決手順:

1. 登録済み Backend を `language`/`backend` 指定でフィルタ。
2. `contract_version` 互換・`required_capabilities`・`artifact_kinds` を検査。
3. 候補 0 件 → `ResolveError::NoMatchingBackend`（理由: `UnknownLanguage`/`UnknownBackend`/`UnsupportedContractVersion`/`MissingCapability` を区別）。
4. 候補 1 件 → その Backend を選択。
5. 候補 2 件以上 → `ResolveError::AmbiguousBackend`（`backend` 明示指定を要求）。

禁止: 最近似 Backend への暗黙フォールバック、未登録 Backend の遅延ロード、Feature flag による実行時排他。

### C2. Runtime Port（`ScriptBackend`/`ScriptSession`）

| 型 | 役割 | 契約上の不変条件 |
| --- | --- | --- |
| `ScriptBackend` | Backend 実装の trait/interface | `descriptor() -> &BackendDescriptor`、`open_session(&SessionConfig) -> Result<ScriptSession, SessionError>`。VM vendor 型をシグネチャに出さない。 |
| `BackendDescriptor` | Backend の静的記述 | `id`/`language`/`version`/`contract_version`/`capabilities`/`supported_artifact_kinds`。 |
| `SessionConfig` | Session 生成時の設定 | `budget: ExecutionBudget`、`host_bindings: HostApiSurface`、`artifact_kinds`、`source: ScriptSource`、`placement_hint`（配置特性の要求、任意）。 |
| `ScriptSession` | 1 回の評価コンテキスト | `evaluate(entry?) -> Result<ScriptResult, ScriptError>`、`cancel()`、`close()`。**Send/Sync を無条件に要求しない**。配置は Backend Capabilities が広告する特性に従い Consumer が選択。 |
| `ScriptResult` | Session の評価結果 | `artifact: ScriptArtifact`（型付き定義文書）+ `return_value: Option<ScriptValue>`（任意）+ `diagnostics: Vec<ScriptDiagnostic>`（実行時診断、任意）。 |

Session の所有権・ライフサイクル:

- Session は Backend が生成し、Consumer が所有する。`open_session` 失敗時は SessionError（Budget 非対応・Host binding 不正・Resource 不足等）。
- Session 間で VM 可変状態を共有しない（`shared_vm_state` capability が明示された場合のみ例外）。グローバル汚染は Capabilities で広告。
- `cancel()` は冪等。Cancel は協調的で、VM の safepoint で観測される。**ブロッキング Host 関数呼出し中は即時中断を保証しない**（Fault Matrix 参照）。

### C3. 値モデル（`ScriptValue`/`ScriptError`/`ScriptSource`/`ScriptArtifact`）

#### `ScriptValue` — 閉じた variant 集合

```
Absent | Null | Bool(bool) | Integer(i64) | Number(f64)
| String(String) | Bytes(Vec<u8>)
| Array(Vec<ScriptValue>) | Record(BTreeMap<String, ScriptValue>)
| Time { domain: TimeDomainId, timebase: Rational, count: i64 }
| ResourceId { domain: ResourceDomainId, kind: String, value: String }
```

不変条件:

- **Absent ≠ Null**: Absent = フィールド未提供/optional 未設定、Null = 明示的 null。混同禁止（Synvelis #271 要件）。
- **Integer(i64)**: 厳密。Luau number（f64）で表現不能な整数は `ScriptError::ValueOutOfRange` で拒否し、暗黙丸め禁止。
- **Number(f64)**: IEEE-754 binary64。NaN/±Inf は表現可能だが、Backend は NaN payload 正規化を文書化する義務がある。
- **Time**: `domain`（ClockDomainId 相当）+ `timebase`（有理数 num/den）+ `count`（i64）を保持。暗黙のドメイン変換・単位丸め禁止。
- **ResourceId**: `domain`（リソース種別）+ `kind` + `value`（opaque string）。VM/契約は解釈せず、Host binding のみが意味を持つ。生ポインタ・OS ハンドル・製品型の直接越境を防ぐ。
- **禁止越境**: VM テーブル/関数/userdata/coroutine/thread、ネイティブポインタ、`Grant`/`Secret`/`Token`/認証情報、Synvelis Gateway/Project 型・AudioLens 型・GPUI 型・Plugin SDK 具象型、Rust `Any`/trait object、OS ハンドル（`ResourceId` の opaque 表現を除く）。

#### `ScriptError` — 言語中立エラー

```
ScriptError {
  category: ScriptErrorKind,   // Parse|Compile|Type|Runtime|BudgetExceeded{kind}|Cancelled|Unsupported{what}|Host{detail}|Backend|Internal
  message: String,
  position: Option<SourcePosition>,  // offset/line/col
  backend_id: ScriptBackendId,
  session_id: Option<SessionId>,
  cause: Option<Box<ScriptError>>,
}
```

Vendor エラー型（mlua::Error・rhai::ParseError 等）を公開面に出さず、Backend が `ScriptError` へ写像する責務を負う。

#### `ScriptSource`/`SourceDigest`/`SourceRevision`

```
ScriptSource {
  text: String,                 // UTF-8 ソース
  language: ScriptLanguageId,
  backend_hint: Option<ScriptBackendId>,   // 明示指定（省略時は language+capabilities で解決）
  schema_version: u32,          // ソース形式版
  digest: SourceDigest,         // 内容ハッシュ（アルゴリズム版付き、例: "sha256:v1:..."）
  revision: SourceRevision,     // Consumer が管理する版識別子（編集ごとに単調増加）
}
```

`SourceDigest` は内容同一性、`SourceRevision` は Consumer 側の編集版を表す。Analyzer 結果・Artifact は `source_revision` を保持し、Consumer は stale 結果を拒否する。

#### `ScriptArtifact` — 型付き定義文書

```
ScriptArtifact {
  schema_version: u32,
  kind: ArtifactKind,           // ActionRecipe | WorkflowDefinition | ConsumerExtension{domain}
  digest: DefinitionDigest,     // 正規化シリアライズのハッシュ
  payload: ScriptValue::Record, // 型付き定義本体（canonical form）
  source: { language, backend_id, source_digest, source_revision },
  produced_by: { backend_id, impl_version },
}
```

不変条件:

- `payload` は canonical form（キーソート・数値正規化・文字列エスケープ統一）でシリアライズ可能。**同一意味の定義は Luau/Rhai いずれの Backend からも同一 digest を生成する**ことを Fixture で検証する。
- `schema_version` は Artifact 形式の版。未知の版は Consumer/検証側で `UnsupportedSchemaVersion` として Fail Closed。
- VM 状態（stack/coroutine/table/function）を `payload` に含めない。Serialized 定義に Grant/Secret/Token を含めない（検証で拒否）。
- Artifact は「Action Recipe」または「Workflow Definition」の型付き文書であり、Workflow DSL YAML と共通の型付き Plan/Coordinator へ lower される入力となる。VM を実行正本にしない。

### C4. Budget/Cancel（`ExecutionBudget`/`ScriptCancel`）

```
ExecutionBudget {
  max_instructions: Option<u64>,
  max_wall_time: Option<Duration>,
  max_memory_bytes: Option<u64>,
  max_output_bytes: Option<u64>,
  max_host_calls: Option<u64>,
}
```

- 各項目は `Option`。`None` = 無制限（Consumer が明示的に選択）。
- Backend Capabilities は強制可能な Budget 種別を広告する。要求された Budget 種別が未対応なら `open_session` 時に `SessionError::UnsupportedBudget` で Fail Closed。
- `ScriptCancel`: `cancel()` 冪等・協調的。VM safepoint で観測。**ブロッキング Host 関数呼出し中の即時中断は保証しない** — Host binding は bounded に設計するか、Consumer が別スレッドで実行して cancel 後に結果を破棄する責務を負う。
- RT パスからの `open_session`/`evaluate`/`cancel` 呼出しは契約違反。全 API は non-RT のみ。

### C5. Analyzer Port（`ScriptAnalyzer`/`AnalyzerRegistry`）

Runtime とは**完全に独立**した Registry/Port。

| 型 | 役割 | 契約上の不変条件 |
| --- | --- | --- |
| `ScriptAnalyzer` | 解析実装の trait/interface | `analyze(&AnalyzeRequest) -> Result<AnalyzeResult, AnalyzerError>`。Vendor 型を出さない。 |
| `AnalyzerRegistry` | Analyzer 登録・実行時解決 | Backend Registry とは別。`resolve` は同一の Fail-Closed 規則（未知/曖昧/Version/Capability）。 |
| `AnalyzerCapabilities` | 解析能力の広告 | `diagnostics: Unsupported \| Syntax \| SyntaxAndType`、`completion: bool`、`max_source_bytes`、`supported_languages`。未実装能力は `Unsupported` で返し、成功を偽装しない。 |
| `AnalyzeRequest` | 解析要求 | `source: ScriptSource`（revision/digest 含む）、`language`、`type_definitions: Option<DefinitionSnapshotRef>`（Consumer 提供の型定義スナップショット）、`wanted_capabilities`。 |
| `AnalyzeResult` | 解析結果 | `source_revision`（解析対象の版）、`diagnostics: Vec<Diagnostic>`、`completions: Option<Vec<Completion>>`。各 Diagnostic/Completion は `source_revision` を保持。 |
| `Diagnostic` | 構造化診断 | `severity`、`code`、`message`、`range{start,end}`、`source_revision`。 |
| `Completion` | 補完候補 | `label`、`kind`、`detail?`、`replace_range`、`source_revision`。 |

不変条件:

- Analyzer の成否は実行 Grant を意味しない。静的解析成功 ≠ 実行許可。
- `AnalyzeResult.source_revision` が要求時の `ScriptSource.revision` と一致しない場合、Consumer は stale 結果として拒否する（契約上の責務は Consumer 側）。
- Analyzer は Runtime Backend なしで登録・利用可能（例: 編集時のみ解析）。逆も同様。
- Luau.Analysis C++ native FFI は #560 の隔離 PoC で検証。契約は FFI 実装の詳細を規定しない。

### C6. Host API（`HostApiSurface`/`HostBinding`）

Script が呼び出せる Host 関数の**記述子**のみを契約化する。実装・権限は Consumer が所有。

```
HostBinding {
  name: String,
  signature: HostSignature,        // ScriptValue 入出力の型付きシグネチャ
  effect: HostEffect,              // Pure | Query | Effect（情報提供用、権限ではない）
  schema_version: u32,
  timeout_hint: Option<Duration>,  // 推奨上限（強制は Consumer 責務）
}
HostApiSurface { bindings: Vec<HostBinding>, schema_version: u32 }
```

不変条件:

- 中立契約に OS/FS/Network/Shell の Host binding を**含めない**。それらが必要なら Consumer が `HostApiSurface` として注入する。
- 各 Host 呼出しは Consumer（例: Synvelis Gateway）が実行時に再認可する。Analyzer/静的成功は Grant を与えない。
- `effect` は助言情報であり、権限判定の根拠にしない（権限は Consumer のポリシー層）。
- Artifact/定義文書に Grant/Secret/Token を含めない。検証側で拒否。

### C7. Workflow/Artifact lowering（Consumer 側責務の境界）

- Script が生成する `ScriptArtifact`（`kind = ActionRecipe | WorkflowDefinition`）は、Consumer の PlanCompiler/Coordinator が消費する**型付き文書**。
- YAML/Luau/Rhai は全て同一の型付き `WorkflowDefinition`/`Plan` へ lower される（Synvelis #273 要件）。lowering は Consumer 側責務であり、本契約は Artifact の**形**のみを規定する。
- 異種言語間の参照は型付き Step/Definition 参照経由。VM 直接呼出し・VM stack/coroutine 永続化は禁止。
- Ruviform は Workflow Engine/Coordinator/Job の正本を持たない。`ruviform-exec` は OfflineJob/Executor として session を駆動する既存基盤であり、Script 契約はそれを再利用する（新規 Job/Project 正本を作らない）。

## Data and Control Flow

1. **登録**: Consumer が `ScriptBackendRegistry::register(Box<dyn ScriptBackend>)` で Backend を登録。`AnalyzerRegistry` も同様に独立登録。
2. **解決**: `resolve(ScriptRequirement)` → Fail-Closed で Backend 選択（C1）。
3. **Session 開始**: `open_session(SessionConfig{budget, host_bindings, source})` → Backend が Session を生成。Budget 非対応・Host binding 不正は `SessionError`。
4. **評価**: `session.evaluate(entry?)` → `ScriptResult{artifact, return_value?, diagnostics?}`。Cancel/ Budget 超過は `ScriptError`。
5. **Artifact 検証**: Consumer が `ScriptArtifact` の `schema_version`/`digest`/`payload` を検証し、PlanCompiler/Coordinator へ渡す。
6. **解析（任意）**: Editor/CLI が `ScriptAnalyzer::analyze(AnalyzeRequest)` → `AnalyzeResult{diagnostics, completions?}`。stale revision は Consumer が拒否。

## Non-Functional Constraints

- **RT 禁止**: 全 Script API は non-RT。RT callback からの呼出しは契約違反。
- **Session 隔離**: Session 間で VM 可変状態を共有しない（明示 capability を除く）。
- **Fail Closed**: 未知・曖昧・Version/Capability/Schema 不一致は型付きエラーで拒否。暗黙フォールバック禁止。
- **値完全性**: Absent/Null 区別、i64 精度、Time/ID 保持、VM/製品型/Grant/Secret 非露出。
- **正直な未実測**: 未実行の Runtime 共存・3OS・native FFI は NotRun/Deferred。Pass に偽装しない。検証レベルは `POLICIES.md` に従い OS-L1（native build/link/run）/GUI-L2/EXT-L3 を区別し、実機能を使う箇所にのみ適用する。fake/headless/offscreen 証拠は実 GUI/Plugin/デバイス受入の代替にしない。
- **非重複**: `ruviform-exec`/`ruviform-cli-sample`/Consumer Coordinator/Gateway/Job と責務を重複させない。
- **公開 enum 方針**: 将来の公開 enum/型拡張は `docs/adr/public-enum-non-exhaustive-policy.md` に従う。

## Fail-Closed 判定表

| # | 条件 | 検出点 | 必須結果 | 対応 Fixture |
| --- | --- | --- | --- | --- |
| F-01 | 未知 `ScriptLanguageId` | `resolve` | `ResolveError::NoMatchingBackend{reason: UnknownLanguage}` | FX-N-01 |
| F-02 | 未知 `ScriptBackendId` | `resolve` | `ResolveError::NoMatchingBackend{reason: UnknownBackend}` | FX-N-02 |
| F-03 | 要件を満たす Backend 0 件 | `resolve` | `ResolveError::NoMatchingBackend` | FX-N-03 |
| F-04 | 要件を満たす Backend 2 件以上 | `resolve` | `ResolveError::AmbiguousBackend` | FX-N-04 |
| F-05 | `contract_version` 不一致 | `resolve`/`register` | `ResolveError::NoMatchingBackend{reason: UnsupportedContractVersion}` | FX-N-05 |
| F-06 | 必須 Capability 未対応 | `resolve`/`open_session` | `ResolveError::NoMatchingBackend{reason: MissingCapability}` / `SessionError::UnsupportedBudget` | FX-N-06 |
| F-07 | `schema_version` 未知の Artifact | Consumer 検証 | `ArtifactError::UnsupportedSchemaVersion` | FX-N-07 |
| F-08 | Source `digest` 不一致（再読込/Resume） | Session/Consumer | `SessionError::SourceDigestMismatch` | FX-N-08 |
| F-09 | 異種 VM direct reference を含む定義 | Consumer Plan 検証 | `PlanError::CrossVmReference`（契約上の表現不可） | FX-N-09 |
| F-10 | 定義内の循環参照 | Consumer Plan 検証 | `PlanError::CycleDetected` | FX-N-10 |
| F-11 | 未知 Step/Capability 参照 | Consumer Plan 検証 | `PlanError::UnknownReference` | FX-N-11 |
| F-12 | i64 非表現可能な整数の越境 | `evaluate`/`ScriptValue` 変換 | `ScriptError::ValueOutOfRange` | FX-N-12 |
| F-13 | Analyzer Capability 未実装 | `analyze` | `AnalyzerError::Unsupported{capability}` | FX-N-13 |
| F-14 | Analyzer 結果の stale revision | Consumer 検証 | `AnalyzerError::StaleResult`（Consumer 側拒否） | FX-N-14 |
| F-15 | RT パスからの呼出し | 全 API | 契約違反（debug assert / 型レベル non-RT 保証） | FX-N-15 |
| F-16 | Session 間の状態漏洩 | Session 隔離検証 | `SessionError::IsolationViolation`（capability 未広告時） | FX-N-16 |
| F-17 | Artifact/定義への Grant/Secret 混入 | Consumer 検証 | `ArtifactError::ForbiddenField` | FX-N-17 |
| F-18 | Analyzer 未登録/利用不可（Runtime Backend のみ存在） | `AnalyzerRegistry::resolve`/`analyze` | `ResolveError::NoMatchingBackend` / `AnalyzerError::Unavailable`（Backend 解決とは独立に Fail Closed） | FX-N-24 |

## Fault Matrix

| # | 障害 | 検出 | 封じ込め | 検証 |
| --- | --- | --- | --- | --- |
| M-01 | Sandbox/OS API の不正公開 | HostApiSurface 検査 | 中立契約に OS/FS/Net/Shell binding を含めない。Consumer 注入のみ | FX-N-18 |
| M-02 | Host 関数のブロッキング中断不能 | `cancel()` が safepoint に到達しない | Host binding は bounded 設計 or 別スレッド実行+結果破棄（Consumer 責務）。`cancel_granularity` capability で広告 | FX-N-19 |
| M-03 | Budget 超過（無限ループ等） | `ExecutionBudget` 強制 | `ScriptError::BudgetExceeded{kind}` で session 終了 | FX-N-20 |
| M-04 | Cancel 中のブロッキング Host 呼出し | `cancel()` 後も Host が return しない | 結果を破棄し Session を close。即時中断は保証しない旨を文書化 | FX-N-19 |
| M-05 | Session 間の VM 状態共有 | `shared_vm_state` capability 未広告での共有 | SessionError::IsolationViolation | FX-N-16 |
| M-06 | RT コールバックからの呼出し | non-RT 保証の違反 | 契約違反として拒否/検出 | FX-N-15 |
| M-07 | Analyzer の crash/timeout/FFI 障害 | Analyzer 実装の障害 | #560 で隔離 PoC。契約は FFI 詳細を規定しない。Unsupported/エラーで返す | Deferred（#560） |
| M-08 | VM panic/abort（Backend 内部） | プロセス障害 | Backend 実装の責務。契約は panic を ScriptError へ写像することを要求しない（プロセス分離は Consumer 責務） | Deferred（#559） |
| M-09 | メモリ枯渇 | `max_memory_bytes` 超過 | BudgetExceeded で終了。OOM は Backend/OS 責務 | FX-N-21 |
| M-10 | 巨大/深いネストのソース | `max_source_bytes`/parser 制限 | Parse/Compile エラーで拒否 | FX-N-22 |
| M-11 | 悪意ある入力（ReDoS 相当・爆弾） | Budget/timeout | BudgetExceeded/Cancelled で終了 | FX-N-23 |
| M-12 | Source digest 衝突/改竄 | `SourceDigest` 検証 | digest 不一致で Fail Closed | FX-N-08 |
| M-13 | Version drift（Backend/Analyzer/Artifact） | `contract_version`/`schema_version`/`source_revision` | Fail Closed（F-05/F-07/F-14） | FX-N-05/F-07/F-14 |

## 設計 Fixture 割当

正例（Luau/Rhai → 同一意味 Versioned Definition）:

| Fixture | 内容 | 検証 |
| --- | --- | --- |
| FX-P-01 | Luau Backend が Action Recipe を生成 | `ScriptArtifact{kind: ActionRecipe}` が schema_version/digest/payload を持つ |
| FX-P-02 | Rhai Backend が**同一意味**の Action Recipe を生成 | FX-P-01 と同一 `DefinitionDigest`（canonical form で一致） |
| FX-P-03 | Luau/Rhai が同一意味の Workflow Definition を生成 | 同一 digest。異種言語間は型付き Step/Definition 参照のみ |
| FX-P-04 | 混合言語 Workflow（Luau Action + Rhai Action を型付き参照で結合） | VM 直接参照なし。PlanCompiler が受理 |
| FX-P-05 | `Absent` と `Null` の区別を含む値モデル往復 | Absent≠Null が保持される |
| FX-P-06 | i64 精度・Time/ResourceId の往復 | 値が損失なく保持される |
| FX-P-07 | 同一プロセス内で Luau+Rhai Backend を同時登録・実行時選択 | `resolve` が正しく Dispatch（#559 で実測） |
| FX-P-08 | Analyzer が Diagnostics/Completion を返す | `AnalyzeResult` が revision 付きで返る |

反例（Fail Closed）:

| Fixture | 内容 | 期待結果 |
| --- | --- | --- |
| FX-N-01 | 未知 `ScriptLanguageId`（例: `"python"`） | `ResolveError::NoMatchingBackend{UnknownLanguage}` |
| FX-N-02 | 未知 `ScriptBackendId` | `ResolveError::NoMatchingBackend{UnknownBackend}` |
| FX-N-03 | 登録 Backend なし | `ResolveError::NoMatchingBackend` |
| FX-N-04 | 同一言語に 2 Backend、`backend` 未指定 | `ResolveError::AmbiguousBackend` |
| FX-N-05 | `contract_version` 非互換 Backend | `ResolveError::NoMatchingBackend{UnsupportedContractVersion}` |
| FX-N-06 | 必須 Capability（例: `max_memory_bytes` 強制）未対応 | `SessionError::UnsupportedBudget` |
| FX-N-07 | 未知 `schema_version` の Artifact | `ArtifactError::UnsupportedSchemaVersion` |
| FX-N-08 | Source digest 不一致での Resume | `SessionError::SourceDigestMismatch` |
| FX-N-09 | 定義内に異種 VM への直接参照（例: Luau table → Rhai function） | `PlanError::CrossVmReference`（表現不可） |
| FX-N-10 | Workflow Definition 内の循環参照 | `PlanError::CycleDetected` |
| FX-N-11 | 未知 Step/Capability 参照 | `PlanError::UnknownReference` |
| FX-N-12 | i64 非表現可能な整数を `ScriptValue::Integer` へ | `ScriptError::ValueOutOfRange` |
| FX-N-13 | Analyzer が Completion 未実装で要求 | `AnalyzerError::Unsupported{completion}` |
| FX-N-14 | 古い `source_revision` の Analyzer 結果 | Consumer が `StaleResult` として拒否 |
| FX-N-15 | RT コールバックからの `evaluate` | 契約違反（non-RT 保証違反を検出） |
| FX-N-16 | `shared_vm_state` 未広告での Session 間状態共有 | `SessionError::IsolationViolation` |
| FX-N-17 | Artifact payload に Grant/Secret/Token フィールド | `ArtifactError::ForbiddenField` |
| FX-N-18 | HostApiSurface に OS/FS/Net/Shell binding を注入 | Consumer 側で拒否（契約は中立面のみ規定） |
| FX-N-19 | ブロッキング Host 関数中の `cancel()` | 即時中断は保証しない。結果破棄+Session close を確認 |
| FX-N-20 | `max_instructions` 超過の無限ループ | `ScriptError::BudgetExceeded{max_instructions}` |
| FX-N-21 | `max_memory_bytes` 超過 | `ScriptError::BudgetExceeded{max_memory_bytes}` |
| FX-N-22 | `max_source_bytes` 超過の巨大ソース | `ScriptError::Parse`/`Compile`（Backend 制限） |
| FX-N-23 | 敵対的入力（深いネスト・爆弾） | BudgetExceeded/Cancelled で終了 |
| FX-N-24 | Analyzer 未登録で `analyze` 要求（Runtime Backend のみ存在） | `AnalyzerError::Unavailable`（Backend 解決とは独立に Fail Closed） |

## 2 Consumer 適用性（サマリ）

詳細は `evidence/rf558/consumer-applicability.md`。判定語彙: `Reuse`/`Extend`/`New`/`Consumer-only`/`Deferred`。

| Consumer Scenario | 判定 | 根拠 |
| --- | --- | --- |
| Synvelis SV-SCRIPT-01 ScriptApiModel/型生成 | Reuse（契約）+ Consumer-only（生成器） | Ruviform は `ScriptValue`/`HostBinding` 記述子を提供。Gateway Schema からの型生成は Synvelis 責務。 |
| Synvelis SV-SCRIPT-02 Action Script | Reuse（Session/Artifact）+ Consumer-only（Recipe lowering/Gateway 統合） | 本契約の Runtime Port/Artifact を消費。Action Recipe への lowering は Synvelis。 |
| Synvelis SV-SCRIPT-03 Workflow lowering | Reuse（Artifact=WorkflowDefinition）+ Consumer-only（PlanCompiler/Coordinator） | 型付き Definition/Plan への収束は Synvelis #273。Coordinator は #265。 |
| Synvelis SV-SCRIPT-04 Editor/Analyzer | Reuse（ScriptAnalyzer Port）+ Consumer-only（GPUI Editor/ScriptApiModel stub） | Diagnostics/Completion/Source Revision を提供。GPUI Editor は Synvelis。 |
| Synvelis AUTX CLI（`action run --file` 等） | Consumer-only/Deferred | CLI は製品側責務。Ruviform は CLI framework を昇格しない（#155 先例）。 |
| AudioLens 分析/CLI/export | Deferred（現時点 Script 非要求） | AL #127 は Script 導入を要求しない。将来 crate 昇格時に f64/long-duration/CLI 影響を評価。 |
| AudioLens 将来 Script 適用 | Deferred | Ruviform 側昇格で AL Cargo.lock/依存木に影響が出た時点で再評価。headless/f64/CLI 互換を条件とする。 |
| Ruviform `ruviform-exec` | Reuse | OfflineJob/OfflineDriver/CancellationToken を session 駆動に再利用。新規 Job 正本を作らない。 |
| Ruviform `ruviform-cli-sample` | Reuse（参考） | #155 利用例アダプタ。CLI framework としての昇格はしない。 |
| Ruviform `ruviform-media-core` Time/ID | Reuse（意味論参照） | TimeDomain/ResourceId の意味論を参照。実装型の再利用は #559 で決定。 |
| Workflow Engine/Coordinator/Job 正本 | Consumer-only（Synvelis）/ Deferred（#553 で Ruviform 共通化を判定） | Ruviform は Script 契約のみ。Workflow 実行正本は #553 の判定対象。 |

**重要**: 1 Consumer の成功は共通採用を意味しない。Synvelis の Script 機能採用は AudioLens の採用を意味せず、逆も同様。両 Consumer の独立受入（#294）が必要。

## Risks and Trade-offs

| Risk | 影響 | 緩和 |
| --- | --- | --- |
| 契約の網羅性不足で #559/#560 が設計齟齬 | PoC 手戻り | 本設計で型・エラー分類・Fixture・Fault Matrix を固定。review-protocol でレビュー。 |
| `ScriptSession` の Send/Sync 非要求で配置設計が複雑化 | Consumer 側の所有権設計負荷 | BackendCapabilities で配置特性を広告。#559 で実測。 |
| Canonical form/digest の実装が Backend 間で不一致 | FX-P-02/P-03 が Fail | Fixture で同一 digest を要求。canonical 化規則を契約で固定。 |
| Analyzer と Runtime の能力差で「解析成功=実行可」の誤解 | セキュリティ/整合性 | 契約で「Analyzer 成功は Grant を意味しない」を明記。Consumer 側で再認可。 |
| 未実測項目を Pass と誤認 | POLICIES 違反 | 全未実行項目を NotRun/Deferred と明記。`evidence/rf558/verification.md` に記録。 |

## Deferred Design Concerns

- 実装 crate 配置・命名・公開 API の Rust 確定 | PoC 前に crate を生むと #559/#560 の実測なしに API を固定 | #559/#560 PoC + #553 判定後 | 契約と実装の齟齬 | #553 決定ログ・#559/#560 受入
- 共通 API の製品昇格・Rhai 本番採用 | Issue 合意どおり実証後判断 | #559 PoC 共存実証 + Consumer 受入 | 未検証のまま製品依存 | #553/#294・SV #28・AL #127
- AudioLens への Script 適用 | AL #127 は現時点で Script 非要求 | Ruviform 昇格で AL Cargo.lock/依存木に影響が出た時点 | AL への不要な依存・精度負担 | AL #127 候補 SHA 差分評価
- `Send`/`Sync` 可能 Session の提供形態 | Backend 実装依存 | #559 PoC で配置特性を実測後 | Consumer 要求を満たせない Backend | BackendCapabilities + #559 Fixture
- Lua 5.4 対応 | Issue ベースラインで非対応と明記 | 将来の明示的要求があった場合のみ | 言語数増加による複雑化 | 新規 Issue/ADR |
- VM stack/coroutine 永続化 | Issue で禁止 | Resume/Checkpoint は型付き Definition/Plan 経由のみ | 状態復旧の不整合 | Synvelis #265 Coordinator |

## Verification Notes

- [x] `docs/specs/issue-558-script-contract.md` の受入条件が本設計の各節へ写像されている（`evidence/rf558/contract-trace.md` で対応確認）。
- [x] Fail-Closed 判定表（F-01〜F-17）が全て Fixture（FX-N-01〜FX-N-23）または Fault Matrix（M-01〜M-13）へ割当済み。
- [x] 2 Consumer 適用性が `evidence/rf558/consumer-applicability.md` に判定記録され、Ruviform の製品型非依存が保たれている。
- [x] 未実行項目（実 Runtime 共存・3OS・native FFI・Rhai 本番採用・AL Script 適用）が NotRun/Deferred と明記されている。
- [ ] #559 PoC で FX-P-07（Luau+Rhai 同時登録・実行時選択）と FX-N-01〜N-08 が実測 Pass する（後続ゲート）。
- [ ] #560 PoC で Luau.Analysis native FFI・3OS 配布が実測 Pass する（後続ゲート）。
- [ ] Synvelis/AudioLens の独立受入（#294）が本契約の Consumer 適用性判定を確認する（後続ゲート）。
