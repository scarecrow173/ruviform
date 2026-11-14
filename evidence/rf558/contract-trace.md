# RF558 Script/Analyzer 言語非依存契約 — 受入条件→契約条項→根拠 対応表

- Issue: https://github.com/scarecrow173/ruviform/issues/558
- 作業 branch: `docs/558-script-contract`
- 作成日: 2026-11-14
- 対象スライス: **DESIGN-ONLY**（実装 crate・公開 API・PoC コードは生成しない）
- 判定語彙: `POLICIES.md` §2/§3（要件状態 Required/Deferred/NotApplicable、実行結果 Pass/Fail/NotRun/Unsupported）

## 1. 受入条件 → 契約条項 → 根拠

Issue #558 の 5 つの受入条件を、`docs/designs/issue-558-script-contract-design.md`（以下 design）の契約条項へ対応付ける。

### AC-1: Backend/Analyzer/Session/Registry/ScriptValue/Budget/Cancel/Artifact の公開型と同一プロセス複数 Backend 同時登録・Runtime 選択/Feature 非排他の契約確定

| 契約条項 | design 節 | 内容 |
| --- | --- | --- |
| `ScriptLanguageId`/`ScriptBackendId`/`ScriptBackendVersion`/`BackendCapabilities`/`ScriptBackendRegistry` | C1 | 識別子分離・Version/Capabilities 広告・同時登録 |
| Dispatch 規則（Fail-Closed） | C1 | `resolve` の 0/1/N 候補処理、暗黙フォールバック禁止 |
| `ScriptBackend`/`BackendDescriptor`/`SessionConfig`/`ScriptSession`/`ScriptResult` | C2 | Runtime Port・Session ライフサイクル |
| `ScriptValue`/`ScriptError`/`ScriptSource`/`ScriptArtifact` | C3 | 値モデル・エラー・Artifact |
| `ExecutionBudget`/`ScriptCancel` | C4 | Budget/Cancel 意味論 |
| `ScriptAnalyzer`/`AnalyzerRegistry`/`AnalyzerCapabilities`/`AnalyzeRequest`/`AnalyzeResult`/`Diagnostic`/`Completion` | C5 | Analyzer Port（Runtime 独立） |
| `HostApiSurface`/`HostBinding` | C6 | Host API 記述子 |

**根拠**: Issue #558 本文 2026-11-11 ベースライン「`ScriptLanguageId`と`ScriptBackendId`を分離し、Version/CapabilitiesをもつRegistryで選択する」「`ScriptBackend/ScriptSession`、`ScriptValue/ScriptError`、Source Version/Digest、Typed Artifact/Definition、ExecutionBudget/CancelをVM非依存とし」。同一プロセス共存・実行時 Dispatch は Issue 本文「同一実行ファイル内でLuau/Rhai共存は#559の検証対象であり、ビルド時排他切替では代替しない」および ADR Option C。

### AC-2: Luau/Rhai 同一意味 Versioned Action/Workflow Definition 正例 + 異種 VM direct reference 禁止・cycle/Unknown/Version 違反 Fail Closed 反例を設計 Fixture へ割当

| 契約条項 | design 節 | Fixture |
| --- | --- | --- |
| `ScriptArtifact{kind: ActionRecipe\|WorkflowDefinition, schema_version, digest, payload, source, produced_by}` | C3 | FX-P-01〜P-04（Luau/Rhai 同一 digest・混合言語 Workflow） |
| canonical form・同一 `DefinitionDigest` | C3 | FX-P-02/P-03 |
| 異種 VM direct reference 禁止 | C7・Fail-Closed F-09 | FX-N-09 |
| cycle/Unknown/Version 違反 | Fail-Closed F-05/F-07/F-10/F-11 | FX-N-05/F-07/F-10/F-11 |

**根拠**: Issue 本文「Script は Action Recipe または Workflow Definition の型付き文書を生成し」「異種言語間は型付き Step/Definition 参照経由とし、VM 直接呼出しを禁止」。Synvelis #273「YAML/Luau/Rhai Workflow lowering」要件。test-spec `docs/test-specs/issue-558-script-contract.md` Examples が対応。

### AC-3: i64/Absent/Null/Time/ID/Host API 型、RT 禁止・Session 隔離・Quota・Security/Grant を機械検査可能な境界/試験マトリクスへ固定

| 契約条項 | design 節 | 検証 |
| --- | --- | --- |
| `ScriptValue` Absent≠Null・i64 厳密・Time/ResourceId 保持 | C3 | R-VAL・FX-P-05/P-06・FX-N-12 |
| Host API 型（`HostBinding`/`HostApiSurface`） | C6 | R-SEC・FX-N-18 |
| RT 禁止 | Non-Functional・C4 | R-SES・FX-N-15 |
| Session 隔離 | C2・Fault Matrix M-05 | R-SES・FX-N-16 |
| Quota（`ExecutionBudget`） | C4・Fault Matrix M-03/M-09 | R-BUD・FX-N-06/N-20/N-21 |
| Security/Grant（Artifact 禁止フィールド・Host 再認可） | C6・Fail-Closed F-17 | R-SEC・FX-N-17 |

**根拠**: Issue 本文「Null/Absent・i64精度・TimeDomain/IDを維持」「VMテーブル/関数/ポインタ・Grant/Secret・製品型・GPUIを外へ出さない」「RT パスからの呼出しを契約違反」。Synvelis #271「Absent vs Null 区別」要件。test-spec Rules R-VAL/R-SES/R-ART/R-BUD/R-SEC が機械検査可能な形で固定。

### AC-4: #553 既存 Workflow/CLI 契約・AL 分析/CLI・SV DAW Workflow の独立 Consumer Scenario で Reuse/New/Deferred を判定し、Ruviform の製品型非依存を保つ

| Consumer Scenario | 判定 | 記録 |
| --- | --- | --- |
| Synvelis SV-SCRIPT-01〜04 | Reuse（契約）+ Consumer-only（生成器/Editor/PlanCompiler） | `evidence/rf558/consumer-applicability.md` |
| Synvelis AUTX CLI | Consumer-only/Deferred | 同上 |
| AudioLens 分析/CLI/export | Deferred（現時点 Script 非要求） | 同上 |
| Ruviform `ruviform-exec`/`ruviform-cli-sample`/`ruviform-media-core` | Reuse | 同上 |
| Workflow Engine/Coordinator/Job 正本 | Consumer-only（Synvelis）/ Deferred（#553） | 同上 |

**根拠**: Issue 本文「#553既存Workflow/CLI契約・AL分析/CLI・SV DAW Workflowの独立Consumer ScenarioでReuse/New/Deferredを判定し、Ruviformの製品型非依存を保つ」。design「2 Consumer 適用性」サマリと `evidence/rf558/consumer-applicability.md` の決定ログ。「1 Consumer の成功は共通採用を意味しない」を明記。

### AC-5: 実 Runtime 共存/3OS 証拠は #559、Luau.Analysis native FFI/配布は #560 へ割り当て、両 PoC 未実行を本設計未決定と混同しない

| 委譲先 | 内容 | 現状 |
| --- | --- | --- |
| #559 | Luau+Rhai 同一バイナリ共存・実行時選択・Fail Closed の実測（FX-P-07・FX-N-01〜N-08） | **NotRun**（本スライスは契約のみ） |
| #560 | Luau.Analysis C++ native FFI・版整合・3OS 配布 | **NotRun**（本スライスは契約のみ） |

**根拠**: Issue 本文「実 Runtime 共存/3OS 証拠・Luau.Analysis native FFI/配布は #559/#560 の受入。未実行・未実測は Pass に偽装せず NotRun/Deferred と明記し、両 PoC 未実行を本設計の未決定と混同しない」。design Verification Notes・test-spec Non-goals・ADR Verification が未実測を NotRun/Deferred と明記。

## 2. 契約条項の網羅性確認

| Issue 要求 | design 節 | test-spec Rule | 状態 |
| --- | --- | --- | --- |
| `ScriptLanguageId`/`ScriptBackendId` 分離 | C1 | R-REG | 確定 |
| Version/Capabilities Registry | C1 | R-REG | 確定 |
| 同一プロセス複数 Backend 同時登録 | C1 | R-REG・FX-P-07 | 確定（実測は #559） |
| 実行時 Dispatch・Feature 非排他 | C1 | R-REG | 確定 |
| 未知/曖昧/Version 不一致 Fail Closed | C1・Fail-Closed F-01〜F-06 | R-REG・FX-N-01〜N-06 | 確定 |
| `ScriptBackend`/`ScriptSession` | C2 | R-SES | 確定 |
| 無条件 Send/Sync 非要求 | C2 | R-SES | 確定 |
| `ScriptValue`/`ScriptError` | C3 | R-VAL | 確定 |
| Absent/Null 区別 | C3 | R-VAL・FX-P-05 | 確定 |
| i64 精度 | C3 | R-VAL・FX-N-12 | 確定 |
| TimeDomain/Resource ID | C3 | R-VAL・FX-P-06 | 確定 |
| VM 固有型/Grant/Secret/製品型/GPUI 非露出 | C3・C6 | R-VAL・R-SEC | 確定 |
| Source Version/Digest | C3 | R-ART・FX-N-08 | 確定 |
| Typed Artifact/Definition | C3 | R-ART・R-WF | 確定 |
| ExecutionBudget/Cancel | C4 | R-BUD | 確定 |
| ブロッキング Host 中断不能性 | C4・Fault Matrix M-02/M-04 | R-SES・FX-N-19 | 確定 |
| Session 隔離 | C2・Fault Matrix M-05 | R-SES・FX-N-16 | 確定 |
| RT 非実行 | Non-Functional・C4 | R-SES・FX-N-15 | 確定 |
| `ScriptAnalyzer`/Capabilities/Source Revision/Diagnostics/Completion | C5 | R-ANA | 確定 |
| 未実装能力は Unsupported | C5 | R-ANA・FX-N-13 | 確定 |
| Luau.Analysis native bridge は #560 | C5・Deferred | Non-goals | 確定（委譲） |
| Fail-Closed 判定表 | Fail-Closed F-01〜F-18 | R-REG〜R-DOC | 確定 |
| Fault Matrix | Fault Matrix M-01〜M-13 | R-SES〜R-SEC | 確定 |
| 正例/反例 Fixture | 設計 Fixture FX-P-01〜P-08・FX-N-01〜N-24 | Examples | 確定 |
| 2 Consumer 適用性 Reuse/New/Deferred | 2 Consumer 適用性・evidence | R-DOC | 確定 |
| 製品型非依存 | C3・C6・Non-Functional | R-VAL・R-SEC | 確定 |
| #553 非重複表材料 | 2 Consumer 適用性 | R-WF・R-DOC | 確定 |
| 実 Runtime 共存/3OS → #559 | Verification Notes | Non-goals | 確定（委譲） |
| Luau.Analysis FFI → #560 | Verification Notes | Non-goals | 確定（委譲） |

## 3. 未決定・委譲事項（本スライスで確定しない）

| 項目 | 状態 | 委譲先/再評価条件 |
| --- | --- | --- |
| 実装 crate 配置・命名・公開 API の Rust 確定 | Deferred | #559/#560 PoC + #553 判定後 |
| 共通 API の製品昇格 | Deferred | #559 PoC 実証 + Consumer 受入 |
| Rhai 本番採用 | Deferred | #559 PoC + Consumer 受入 |
| AudioLens への Script 適用 | Deferred | Ruviform 昇格で AL Cargo.lock/依存木に影響が出た時点 |
| `Send`/`Sync` 可能 Session の提供形態 | Deferred | #559 PoC で配置特性を実測後 |
| Lua 5.4 対応 | NotApplicable | Issue ベースラインで非対応と明記 |
| VM stack/coroutine 永続化 | NotApplicable | Issue で禁止。Resume/Checkpoint は型付き Definition/Plan 経由 |
| 実 Runtime 共存・3OS 実測 | NotRun | #559 受入 |
| Luau.Analysis native FFI・3OS 配布 | NotRun | #560 受入 |
| Synvelis/AudioLens 独立受入 | NotRun | #294・SV #28・AL #127 |

## 4. 文書チェーン対応

| 文書 | パス | 役割 |
| --- | --- | --- |
| Spec | `docs/specs/issue-558-script-contract.md` | 要件・受入条件 |
| Design | `docs/designs/issue-558-script-contract-design.md` | 契約条項・Fail-Closed 表・Fault Matrix・Fixture |
| ADR | `docs/adr/issue-558-script-backend-contract.md` | 決定記録（Option C 採用） |
| Test-spec | `docs/test-specs/issue-558-script-contract.md` | Rule/Example（将来の検証仕様） |
| Plan | `docs/plans/issue-558-script-contract-plan.md` | 実装準備計画（文書スライス） |
| Task | `docs/tasks/issue-558-script-contract-task.md` | 実装スライス |
| IR | `docs/impl/ir/issue-558-script-contract.md` | 実装記録 |
| Exp | `docs/impl/exp/issue-558-script-contract.md` | 実験記録 |
| Evidence | `evidence/rf558/` | contract-trace・consumer-applicability・verification |
