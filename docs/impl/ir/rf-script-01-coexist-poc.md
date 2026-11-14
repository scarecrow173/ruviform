---
id: "IMPL-034Xgif1IGRANhhbuKUGs1"
type: "impl"
status: "completed"
title: "RF-SCRIPT-01 coexist PoC"
created: "2026-11-14"
updated: "2026-11-14"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/559"
  changes:
    added:
      - type: "file"
        path: "poc/script-coexist/Cargo.toml"
      - type: "file"
        path: "poc/script-coexist/src/lib.rs"
      - type: "file"
        path: "poc/script-coexist/src/contract.rs"
      - type: "file"
        path: "poc/script-coexist/src/value.rs"
      - type: "file"
        path: "poc/script-coexist/src/canonical.rs"
      - type: "file"
        path: "poc/script-coexist/src/codec.rs"
      - type: "file"
        path: "poc/script-coexist/src/plan.rs"
      - type: "file"
        path: "poc/script-coexist/src/analyzer.rs"
      - type: "file"
        path: "poc/script-coexist/src/main.rs"
      - type: "file"
        path: "poc/script-coexist/src/backends/mod.rs"
      - type: "file"
        path: "poc/script-coexist/src/backends/luau.rs"
      - type: "file"
        path: "poc/script-coexist/src/backends/rhai.rs"
      - type: "file"
        path: "poc/script-coexist/tests/fixtures.rs"
      - type: "file"
        path: "poc/script-coexist/docs/spec.md"
      - type: "file"
        path: "poc/script-coexist/docs/design.md"
      - type: "file"
        path: "poc/script-coexist/docs/plan.md"
      - type: "file"
        path: "poc/script-coexist/docs/task.md"
      - type: "file"
        path: "docs/impl/exp/rf-script-01-coexist-poc.md"
      - type: "file"
        path: "evidence/rf559/gap-report.md"
      - type: "file"
        path: "evidence/rf559/verification.md"
    modified:
      - type: "file"
        path: "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-558-script-contract.md"
    - "docs/designs/issue-558-script-contract-design.md"
    - "docs/adr/issue-558-script-backend-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "poc/script-coexist/docs/spec.md"
    - "poc/script-coexist/docs/design.md"
    - "poc/script-coexist/docs/plan.md"
    - "poc/script-coexist/docs/task.md"
    - "docs/specs/issue-558-script-contract.md"
    - "docs/designs/issue-558-script-contract-design.md"
    - "docs/adr/issue-558-script-backend-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-558-script-contract-design.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/impl/exp/rf-script-01-coexist-poc.md"
    - "evidence/rf559/gap-report.md"
    - "evidence/rf559/verification.md"
  defers: []
  deferred-by: []
metadata:
  issues: [559]
  experiments:
    adopted:
      - "docs/impl/exp/rf-script-01-coexist-poc.md"
    rejected: []
---

# RF-SCRIPT-01 coexist PoC

## Summary

Issue #559 の headless PoC。`poc/script-coexist/`（独立 workspace
crate `script-coexist-poc`）で、Luau（mlua vendored `luau0-src`）と
Rhai を同一実行ファイルに同時リンク・初期化し、契約型をミラーした
PoC ローカル `ScriptBackendRegistry` 経由の実行時選択・Versioned
Typed Definition の canonical digest 一致・混種 Workflow 参照・
Fail-Closed を実装・実測した。36 fixture test 全 Pass、demo binary
で同一 digest を実演。共存は **成立**（PoC 範囲）。製品正式
サポート・3OS 配布・Consumer 受入は後続 Gate（#558/#553/#294/
synvelis#264）に deferred。

## Context

- Task: Issue #559（RF-SCRIPT-01）。上位契約は Issue #558
  （RF-SCRIPT-00、PR #621 で main 統合済み）の spec/design/ADR。
- 内部文書チェーン: `poc/script-coexist/docs/{spec,design,plan,task}.md`
  （canonical docs ツリー外の poc 内部文書）。
- `ScriptBackendRegistry` は契約のみで本番実装がないため、本 PoC は
  契約型をミラーした最小ローカル registry を PoC 印付きで実装した
  （`src/contract.rs` の `POC_CONTRACT_VERSION` / `poc_simulate_rt_context`）。

## Implementation

### Changes

- `poc/script-coexist/Cargo.toml`: 独立 `[workspace]`、mlua(0.12.2,
  luau+send)/rhai(1.27.0,sync)/serde_json/sha2。
- `src/contract.rs`: `ScriptBackendRegistry`（resolve fail-closed:
  UnknownLanguage/UnknownBackend/Duplicate/Ambiguous/UnsupportedContract
  Version/MissingCapability）、`BackendDescriptor`/`BackendCapabilities`/
  `SessionConfig`/`ExecutionBudget`/`CancelToken`/`ScriptSession: Send`/
  `HostApiSurface`/`ensure_non_rt`。
- `src/value.rs`: `ScriptValue`（Absent/Null/Bool/Integer/Number/
  String/Bytes/Array/Record/Time/ResourceId）、`ScriptArtifact`/
  `ScriptResult`/`ScriptError`/`ScriptErrorKind`、`BudgetKind` 単一正本。
- `src/canonical.rs`: canonical serialization + `definition_digest`/
  `source_digest`（SHA-256 `sha256:v1:`）。
- `src/codec.rs`: `$rv` reserved tag record による materialize/
  dematerialize（i64 厳密・Time・ResourceId・Absent/Null を vendor
  非依存に往復）。
- `src/backends/{mod,luau,rhai}.rs`: session 検証・host dispatch・
  budget/cancel safepoint・結果組立の共通層 + 両 backend 実装。
  Luau は `set_interrupt`+`set_memory_limit`、Rhai は `on_progress`。
- `src/plan.rs`: `DefinitionStore`（digest 検証付き content-addressed
  store）+ `build_workflow_plan`（`definition_ref` のみ受理、
  unknown/stale/kind/digest/cycle fail-closed）。
- `src/analyzer.rs`: parse-only `AnalyzerRegistry`（Luau/Rhai analyzer）。
- `src/main.rs`: 単一バイナリ demo（registry 登録・交互評価・
  同一 digest・fail-closed 表示）。
- `tests/fixtures.rs`: 36 test（FX-P-01〜08・FX-N-01〜28）。
- `mise.toml`: `poc-script-coexist-{check,lint,test,run}` 4 task 追加。
- `evidence/rf559/{gap-report,verification}.md`: gap 判定・OS matrix・
  コマンド記録。

### Decisions During Implementation

- mlua `send` feature を有効化し `ScriptSession: Send` を満たした
  （session は依然 per-eval 独立 VM）。
- `ScriptError.message` を `Box<str>` 化し `clippy::result_large_err`
  を解消（エラー生成頻度が低いため box コストは無視可能）。
- Luau の空 table `{}` は Record、Rhai の `[]` は Array と decode
  される差異を確認し、demo workflow では `references` キーを省略して
  canonical 一致を保った（`inspect_artifact` は省略を「参照なし」と
  受理）。
- Rhai map literal は `key: value` 構文 — fixture source を修正。
- 全公開 enum に `#[non_exhaustive]` を付与（repo policy）。

## Related Experiments

### Adopted

- `docs/impl/exp/rf-script-01-coexist-poc.md`: 採用した設計判断
  （send feature・`$rv` codec・digest=canonical SHA-256・capability
  による budget 差分の正直な広告）を記録。

### Rejected

- なし。

## Validation

`evidence/rf559/verification.md` にコマンド・exit code・demo 出力を
記録。canonical は mise:

- `mise run fmt-check` → Pass
- `mise run poc-script-coexist-check` → Pass
- `mise run poc-script-coexist-lint` → Pass（`-D warnings`）
- `mise run poc-script-coexist-test` → Pass（36 test 全 Pass）
- `mise run poc-script-coexist-run` → Pass

OS matrix: Windows=Pass、macOS/Linux=NotRun（実行不能、非対応未確定）。

## Risks

- Rhai backend は `MaxMemoryBytes` を強制できない（capability 広告で
  非対応を明示、要求時 `UnsupportedBudget`）。製品で memory quota が
  必須なら Rhai 側に計測 wrapper か process 分離が必要。
- instruction/wall-time budget は VM safepoint 近似であり厳密な命令数
  ではない（`CancelGranularity::Safepoint` を広告）。
- Luau safe state は stdlib 非公開の最小構成。製品で stdlib を開く
  場合は host surface（`os`/`io`/`debug`/`loadstring` 遮断）を別途設計。

## Follow-ups

- #558: 契約の capability 語彙・budget 意味論（safepoint 近似）・
  `$rv` tag の製品 codec 化を検討材料として引き継ぐ。
- #553: Workflow/CLI orchestration への本 PoC registry/codec の
  移植可否、Rhai memory quota の扱い（wrapper vs process 分離）を
  判断材料として引き継ぐ。
- #560: 3OS Build/Link/Run は別 Gate（本 PoC は Windows のみ実測、
  macOS/Linux は NotRun）。
