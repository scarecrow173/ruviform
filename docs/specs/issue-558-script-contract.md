---
id: "SPEC-034Xe8fKEL6vpFX9pkgPU0"
type: "spec"
status: "proposed"
title: "Multi-backend Script/Analyzer言語非依存契約（Issue #558）"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/558"
    - "https://github.com/scarecrow173/ruviform/issues/553"
    - "https://github.com/scarecrow173/synvelis/issues/259"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-558-script-contract.md"
      - type: "doc"
        path: "docs/designs/issue-558-script-contract-design.md"
      - type: "doc"
        path: "docs/adr/issue-558-script-backend-contract.md"
      - type: "doc"
        path: "docs/test-specs/issue-558-script-contract.md"
      - type: "doc"
        path: "docs/plans/issue-558-script-contract-plan.md"
      - type: "doc"
        path: "docs/tasks/issue-558-script-contract-task.md"
      - type: "doc"
        path: "docs/impl/ir/issue-558-script-contract.md"
      - type: "doc"
        path: "docs/impl/exp/issue-558-script-contract.md"
      - type: "doc"
        path: "evidence/rf558/contract-trace.md"
      - type: "doc"
        path: "evidence/rf558/consumer-applicability.md"
      - type: "doc"
        path: "evidence/rf558/verification.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/designs/issue-558-script-contract-design.md"
    - "docs/plans/issue-558-script-contract-plan.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-155-headless-cli-foundation.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/558"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-558-script-contract.md"
  references:
    - "evidence/rf558/contract-trace.md"
    - "evidence/rf558/consumer-applicability.md"
  defers: []
  deferred-by: []
---

# Multi-backend Script/Analyzer言語非依存契約（Issue #558）

## What Should Be Built

Luau を初期標準とし、同一実行ファイル内で Luau/Rhai を同時登録・実行時選択できる Script Runtime/Analyzer の**言語非依存契約**を文書として確定する DESIGN-ONLY スライス。成果物は契約仕様（公開型・Registry/Dispatch 規則・値モデル・Budget/Cancel・独立 Analyzer Port・Fail-Closed 判定表・Fault Matrix・設計 Fixture 割当）と、Synvelis/AudioLens 2 Consumer 適用性の決定ログ（Reuse/New/Deferred 判定）であり、実装 crate・公開 API・PoC コードは本スライスでは生まない。実 Runtime 共存・3OS Build/Run は #559、Luau.Analysis native FFI/配布は #560 の受入へ委譲する。

## Why Now

Issue #558 は Ruviform #553（Consumer-neutral Workflow/CLI 共通化判断）・Synvelis AUTX-00（#259）・SV-SCRIPT-01〜04（#271〜274）の上位基盤となる FOUNDATIONAL contract である。2026-11-11 設計ベースラインと 2026-11-13 ゲート整理で設計方針は合意済みだが、#559/#560 の PoC が「契約文書に記載された型・意味論」を前提として直ちに着手できるよう、契約をレビュー可能な形で確定する必要がある。契約が未確定のまま PoC を進めると、Backend/Value/Artifact の境界が実装ごとに発散し、#553 の共通化判定と両 Consumer の独立受入が成立しない。

## Users and Value

- **#559/#560 の PoC 実装者**: 言語中立 Port の型・意味論・Fail-Closed 規則を契約文書から一意に読み、Fixture で検証できる。
- **#553 の共通化判定者**: Script Port と Generic Workflow/CLI の責務境界・非重複表の材料を得る。
- **Synvelis（SV-SCRIPT-01〜04）**: Gateway Capability→ScriptApiModel・Action/Workflow lowering・GPUI Editor が消費する中立契約の形を確定できる。
- **AudioLens**: Script 非導入の現状を維持しつつ、将来 crate 昇格時の依存・精度影響を評価する入口を持つ。
- **Ruviform 共通基盤**: 製品型（Synvelis Gateway/Project・AudioLens 型・GPUI・VM 固有 Rust 型）を公開面に出さない境界を固定する。

## Scope

### In Scope

- `ScriptLanguageId`/`ScriptBackendId` 分離、Version/Capabilities を持つ `ScriptBackendRegistry` の同時登録・実行時 Dispatch 契約（未知/曖昧/Version 不一致は Fail Closed）。
- `ScriptBackend`/`ScriptSession` ライフサイクル（無条件 Send/Sync を要求しない配置選択）、`SessionConfig`/`Context`、`ScriptSource`/Version/Digest。
- `ScriptValue`/`ScriptError` 値モデル（Absent/Null 区別・i64 精度・TimeDomain/Resource ID 保持・VM 固有型/Grant/Secret/製品型/GPUI の非露出）。
- `ExecutionBudget`/Cancel セマンティクス、`ScriptArtifact`/Versioned Typed Definition（Action Recipe/Workflow Definition への収束表現）。
- Runtime から独立した `ScriptAnalyzer`/Capabilities/Source Revision/Diagnostics/Completion 境界（未実装能力は Unsupported）。
- Fail-Closed 判定表と Fault Matrix（Sandbox/OS API 公開範囲・Host 関数中断不能性・Budget/Quota・Session 隔離・RT 非実行）を機械検査可能な形で固定。
- Luau/Rhai の同一意味 Versioned Action/Workflow Definition 正例と、異種 VM direct reference・cycle・Unknown・Version 違反の反例を設計 Fixture へ割当。
- #553 既存 Workflow/CLI 契約・AL 分析/CLI・SV DAW Workflow の独立 Consumer Scenario による Reuse/New/Deferred 判定と共通化対象/非対象の決定ログ。
- docs チェーン（spec/design/ADR/test-spec/plan/task/IR/exp）と `evidence/rf558/` 証拠・管理 index 再生成。

### Out of Scope

- 実装 crate の新設・公開 API の Rust コード化・`Cargo.toml`/`Cargo.lock`/`mise.toml`/CI の変更。
- 実 Runtime 共存・同一バイナリ Build/Link/Run・3OS 実機証拠（#559 の受入）。
- Luau.Analysis C++ native FFI・版整合・配布（#560 の受入）。
- 共通 API の製品昇格判断・Rhai 本番採用決定・Workflow Engine/CLI 共通化実装（#553 の判定対象）。
- Lua 5.4、JIT/大規模 MIDI 性能比較、VM stack/coroutine 永続化、独自認可/Job/Project 正本、Script からの任意 Shell/Network/FS 公開。
- Synvelis Gateway/Project・AudioLens 製品コードの変更（両 Consumer は読み取り専用参照）。

## Requirements

- 公開型と契約を、実装言語（mlua/Rhai/Luau.Analysis 等の vendor 型）に依存しない形で定義する。VM テーブル/関数/ネイティブポインタ・Grant/Secret・製品型・GPUI を公開面へ出さない。
- Registry/Dispatch は実行時解決とし、未知・曖昧・Version/Capability 不一致を型付き Fail Closed で拒否する。フォールバック実行・暗黙の最近似 Backend 選択を許さない。
- `ScriptSession` に無条件 Send/Sync を要求しない。owner-thread・専用ワーカー等の配置は Consumer/実装側の選択とし、Backend は対応する配置特性を Capabilities で広告する。
- `ScriptValue` は Absent と Null を区別し、i64 精度・TimeDomain/Resource ID を保持する。Luau number で表現不能な整数を黙って丸めない。VM 固有型・Grant/Secret・製品型の越境を型レベルで禁止する。
- Script は Action Recipe または Workflow Definition の**型付き文書**を生成し、Workflow DSL YAML と共通の型付き Plan/Coordinator へ lower される。VM stack/coroutine を復旧正本にしない。異種言語間は型付き Step/Definition 参照経由とし、VM 直接呼出しを禁止する。
- Analyzer は Runtime と独立した Registry/Port とし、最低能力を Diagnostics/Completion とする。未実装能力は Unsupported であり、型検査完了を偽らない。Analyzer 成功は実行 Grant を意味しない。
- RT パスからの呼出しを契約違反とし、Budget/Cancel/Session 隔離/Security-Grant の境界を Fault Matrix と試験マトリクスへ固定する。未実測・未実装は Pass/Unsupported に偽装せず NotRun/Deferred と明記する。

## Acceptance Criteria

- [ ] Backend/Analyzer/Session/Registry/ScriptValue/Budget/Cancel/Artifact の公開型と、同一プロセス複数 Backend 同時登録・Runtime 選択/Feature 非排他の契約が `docs/designs/issue-558-script-contract-design.md` で確定している。
- [ ] Luau/Rhai の同一意味 Versioned Action/Workflow Definition 正例と、異種 VM direct reference 禁止・cycle/Unknown/Version 違反 Fail Closed の反例が設計 Fixture（design §Fixture・test-spec Examples）へ割当済み。
- [ ] i64/Absent/Null/Time/ID/Host API 型、RT 禁止・Session 隔離・Quota・Security/Grant を機械検査できる境界/試験マトリクス（design Fail-Closed 表・Fault Matrix・test-spec Rules）が固定済み。
- [ ] #553 既存 Workflow/CLI 契約・AL 分析/CLI・SV DAW Workflow の独立 Consumer Scenario で Reuse/New/Deferred が `evidence/rf558/consumer-applicability.md` に判定記録され、Ruviform の製品型非依存が保たれている。
- [ ] 実 Runtime 共存/3OS 証拠は #559、Luau.Analysis native FFI/配布は #560 へ割当済みで、両 PoC 未実行を本設計の未決定と混同していない（`evidence/rf558/contract-trace.md` の対応表）。
- [ ] docs チェーン（spec→design→ADR→test-spec→plan→task→IR/exp）と管理 index が front matter・relation・監査を通過している。

## Deferred Design Concerns

- 実装 crate の配置・命名（例: `ruviform-script`）と公開 API の Rust 確定 | PoC 前に crate を生むと #559/#560 の実測なしに API を固定してしまう | #559/#560 の PoC 結果 + #553 の 2 Consumer 判定後 | 契約と実装の齟齬・手戻り | #553 決定ログ・#559/#560 受入
- 共通 API の製品昇格・Rhai 本番採用 | Issue 合意どおり実証後判断 | #559 PoC の共存実証 + Consumer 受入 | 未検証のまま製品依存が生まれる | #553/#294・SV #28・AL #127
- AudioLens への Script 適用 | AL #127 は現時点で Script 導入を要求しない | Ruviform 側 crate/feature 昇格で AL の Cargo.lock/依存木に影響が出た時点 | AL の headless/f64/CLI へ不要な依存・精度負担 | AL #127 の候補 SHA 差分評価
- `Send`/`Sync` 可能 Session の提供形態 | Backend 実装依存のため契約では要求しない | #559 PoC で配置特性を実測後 | Consumer が要求する配置を満たせない Backend が出る | BackendCapabilities 広告 + #559 Fixture

## Sources and Relations

- Issue #558 本文（2026-11-11 設計ベースライン・2026-11-13 ゲート整理/Stage 1 が正本）。
- 上位/関連 Issue: #553（Workflow/CLI 共通化判断）、#559（Registry/Runtime PoC）、#560（Luau.Analysis FFI/3OS PoC）、#219（両 Consumer 平等）、#294（共通変更時の独立回帰）、#155（headless CLI 基盤・Closed）。
- Synvelis: #259（AUTX-00 統合契約）、#271〜274（SV-SCRIPT-01〜04）、#264/#265（WorkflowDefinition/PlanCompiler・Coordinator）、snapshot は `F:/repositry/audio-products/.devin/tmp/issue-snapshot/`（読み取り専用）。
- AudioLens: #127（継続追従・Script 非要求の明記）、同 snapshot。
- Ruviform 既存契約: `crates/ruviform-exec`（OfflineJob/OfflineDriver/CancellationToken — Job/Executor 正本ではない）、`crates/ruviform-cli-sample`（#155 利用例アダプタ）、`docs/designs/issue-155-headless-cli-foundation.md`、`docs/designs/ruviform-architecture-design-baseline.md`（レイヤー/RT 分離）、`docs/designs/shared-foundations-and-modularization-policy.md`、`ruviform-media-core` の TimeBase/MediaTime/ClockDomainId/StreamId/Fact。
- ADR: `docs/adr/issue-558-script-backend-contract.md`（本スライスの決定記録）。
- 証拠: `evidence/rf558/contract-trace.md`（受入条件→契約条項→根拠）、`evidence/rf558/consumer-applicability.md`（2 Consumer 判定ログ）、`evidence/rf558/verification.md`（検証記録）。
