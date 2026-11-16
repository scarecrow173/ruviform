---
id: "PLAN-034ZkLmNpQrStUvWxYzGh4"
type: "plan"
status: "approved"
title: "Issue #692 実施計画 — ruviform-script 中立契約 port crate 新設・検証・docs chain"
created: "2026-11-16"
updated: "2026-11-16"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/692"
    - "docs/specs/issue-692-script-port.md"
    - "docs/designs/issue-692-script-port.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-692-script-port.md"
      - type: "file"
        path: "docs/tasks/issue-692-script-port.md"
      - type: "file"
        path: "docs/test-specs/issue-692-script-port.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-692-script-port.md"
  implemented-by:
    - "docs/tasks/issue-692-script-port.md"
  depends-on:
    - "docs/specs/issue-692-script-port.md"
    - "docs/designs/issue-692-script-port.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-692-script-port.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-692-script-port.md"
    - "docs/designs/issue-692-script-port.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-692-script-port.md"
  verified-by:
    - "docs/test-specs/issue-692-script-port.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #692 実施計画 — ruviform-script 中立契約 port crate 新設・検証・docs chain Implementation Plan

## Goal

`crates/ruviform-script` を新設し、RF-SCRIPT-00 中立契約の design
完全形（C1 識別子/Registry・C2 Runtime Port・C3 値モデル・C4
Budget/Cancel・C5 Analyzer Port・C6 Host API 記述子）+ canonical
digest utils + codec を headless に実装し、fake fixture で検証する。

## Scope

単一 implementation stream。Issue #692 first slice（中立契約 port）
のみ。実 Luau backend/Analyzer・synvelis 接続・依存選定は後続
slice / consumer gate。`mise.toml`・workflow・`docs/**/README.md`・
`POLICIES.md`・`AGENTS.md`・`poc/`・`ffi-poc/`・consumer リポジトリは
変更しない。

## Upstream Documents

- `docs/specs/issue-692-script-port.md`（approved）
- `docs/designs/issue-692-script-port.md`（approved）
- `docs/designs/issue-558-script-contract-design.md`（approved —
  契約正本）
- `docs/test-specs/issue-692-script-port.md`（approved）
- `POLICIES.md`

## File Map

- `crates/ruviform-script/Cargo.toml` — package 定義、serde_json/sha2
  のみ（理由コメント）、`[lints] workspace = true`。
- `crates/ruviform-script/README.md` — 責務・層・依存境界・検証手順。
- `crates/ruviform-script/src/lib.rs` — crate doc・`CONTRACT_VERSION`・
  module 公開・prelude。
- `crates/ruviform-script/src/id.rs` — ID newtype + `SessionId` +
  `ScriptBackendVersion`。
- `crates/ruviform-script/src/value.rs` — `ScriptValue`・位置/範囲・
  digest/revision・`ScriptSource`・`ScriptArtifact` 系・`ScriptError` 系・
  `DiagnosticSeverity`・`ScriptDiagnostic`・`ScriptResult`。
- `crates/ruviform-script/src/budget.rs` — `BudgetKind`・
  `ExecutionBudget`・`CancelGranularity`・`CancelToken`・
  `PlacementHint`。
- `crates/ruviform-script/src/capability.rs` — `ValueKind`・
  `BackendCapabilities`(+`capability_ids`)。
- `crates/ruviform-script/src/host.rs` — `HostEffect`・`HostSignature`・
  `HostBinding`・`HostApiSurface`(+`validate_neutral`)・
  `HostSurfaceError`。
- `crates/ruviform-script/src/session.rs` — `SessionConfig`・
  `SessionError`・`ScriptBackend`・`ScriptSession`。
- `crates/ruviform-script/src/registry.rs` — `BackendDescriptor`・
  `ScriptRequirement`・`RegistryError`・`ResolveError(Reason)`・
  `ScriptBackendRegistry`。
- `crates/ruviform-script/src/canonical.rs` — `canonical_value`・
  `definition_digest`・`source_digest`・`sha256_hex`・`hex_*`・
  `CanonicalError`・`TAG_KEY`。
- `crates/ruviform-script/src/codec.rs` — `materialize`・
  `dematerialize`・`CodecError`・`F64_SAFE_I64`。
- `crates/ruviform-script/src/analyzer.rs` — C5 analyzer port 一式。
- `crates/ruviform-script/tests/{common/mod.rs,registry.rs,codec.rs,
  analyzer.rs,host_surface.rs,session.rs}` — fake fixture + 契約検証。
- `Cargo.toml`（root）— `members` に `"crates/ruviform-script"` 追加のみ。
- `crates/README.md` — 利用側層の行追加 + クレート数更新。
- `Cargo.lock` — workspace 依存変更に伴う再生成（新規クレート分のみ）。
- `docs/{discovery,specs,designs,plans,tasks,test-specs}/
  issue-692-script-port.md` — docs chain。
- `docs/impl/{ir,exp}/issue-692-script-port.md` +
  `issue-692-script-port-exp.jsonl` — impl 記録。
- `evidence/rf692/` — 検証記録。

## Implementation Sequence

- [x] T1: discovery/spec/design/test-spec/plan/task docs chain 作成
- [ ] T2: crate 骨格（Cargo.toml/README/lib.rs）+ workspace 登録
- [ ] T3: `id`/`value`/`budget`/`capability`/`host` module（契約型一式）
- [ ] T4: `session`/`registry` module（Runtime Port + fail-closed resolve）
- [ ] T5: `canonical`/`codec` module（canonical/digest + codec）
- [ ] T6: `analyzer` module（C5 analyzer port + registry）
- [ ] T7: `tests/` fake fixture + 契約検証（fail-closed/codec/digest/
  analyzer/host/session）
- [ ] T8: 検証 lane（fmt-check/check/lint/test/prek）実行・evidence 記録
- [ ] T9: impl-doc（ir/exp）+ review-protocol
- [ ] T10: commit/push/PR/worker_done

## Dependencies and Risks

- serde_json/sha2 は Cargo.lock 既在 — lockfile 差分は新規クレート分のみ。
- `clippy::struct_excessive_bools`（BackendCapabilities）・
  `clippy::float_cmp`（canonical_number）は reason 付き `#[expect]` で
  局所許可。その他 pedantic lint は `#[expect]`+reason、`#[allow]`
  不使用。
- `SourceRange` position 統合形で backend が offset を計算する責務
  — design D5 記録済み。
- `Cargo.lock`・docs index は生成物 — docs index は Coordinator が
  統合時に再生成（本 slice では編集しない）。

## Task Breakdown

- `docs/tasks/issue-692-script-port.md`（T1〜T10 を内包）。

## Verification

- [ ] `mise run fmt-check` — Pass
- [ ] `mise run check ruviform-script` — Pass
- [ ] `mise run lint ruviform-script` — Pass（clippy `-D warnings`）
- [ ] `mise run test ruviform-script` — Pass
- [ ] `mise exec -- prek run --all-files` — Pass
- [ ] registry fail-closed 行列・codec 往復・digest・analyzer・
  host surface・session 契約が fake fixture で検証される
- [ ] macOS/Linux build・実 Luau VM/FFI・実 OS・GUI は NotRun/
  Deferred（CI lane / s2 slice / synvelis gate）

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（worker dispatch）
- Independent work streams: none
- User approval required before dispatch: review-protocol で代替
- If approved: execute task-by-task with review checkpoints.

## Review Handoff

- `SourceRange` position 統合形の backend 負荷（offset 計算責務）。
- `canonical_number` の数値正規化と `Integer`/`Number` の digest
  同一化の意図的仕様。
- `AnalyzerCapabilities::capability_ids()` 導出語彙の consumer 適合性。
