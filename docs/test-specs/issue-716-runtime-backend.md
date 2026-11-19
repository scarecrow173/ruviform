---
id: "TSPEC-zDBgJ4WsZM8yS7zwjSIrwW"
type: "test-specs"
status: "approved"
title: "Issue #716 runtime ScriptBackend — テスト仕様"
created: "2026-11-17"
updated: "2026-11-17"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/specs/issue-716-runtime-backend.md"
    - "docs/designs/issue-716-runtime-backend.md"
    - "docs/knowledge/scripting-luau.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/impl/exp/issue-716-runtime-backend.md"
  depends-on:
    - "docs/specs/issue-716-runtime-backend.md"
    - "docs/designs/issue-716-runtime-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/exp/issue-716-runtime-backend.md"
    - "evidence/rf716/verification.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-716-runtime-backend.md"
  derived-by:
    - "docs/impl/exp/issue-716-runtime-backend.md"
  verifies: []
  verified-by:
    - "docs/impl/exp/issue-716-runtime-backend.md"
  references:
    - "docs/specs/issue-716-runtime-backend.md"
    - "docs/designs/issue-716-runtime-backend.md"
    - "evidence/rf560/platform-matrix.md"
  defers:
    - "実 OS/実 VM 経路の受入（consumer gate — synvelis#516・synvelis#267 AUTX-08）"
  deferred-by: []
---

# Issue #716 runtime ScriptBackend — テスト仕様

## 検証レベル

**OS-L1**（deterministic headless）— Windows x86_64 実機で
`mise run test ruviform-script-luau` により実行。3-OS は本 slice
対象外（`evidence/rf560/platform-matrix.md` の NotRun 分類を
継承 — macOS/Linux は NotRun、unsupported 次元は Unsupported
として個別に記録）。実 OS/実 VM 経路の受入は consumer gate
（synvelis#516・synvelis#267 AUTX-08）へ委譲。

## Test file

`crates/ruviform-script-luau/tests/runtime.rs`（新設 — backend/
session 系を集約。既存 `tests/{bridge,registry,faults}.rs` は
analyzer/bridge 系で不変）。

## Expected / Observed 行列

凡例: E=expected（contract/設計の要求）、O=observed（実測 —
`docs/impl/exp` と `evidence/rf716/` に記録）。

### descriptor / registry

| # | ケース | E | O |
| --- | --- | --- | --- |
| R1 | `LuauBackend::new()` descriptor | backend_id=`luau-mlua`, language=`luau`, contract_version=1, artifact_kinds={ActionRecipe,WorkflowDefinition}, capability_ids={"session.evaluate"}, owner_thread_only=true, send=false, sync=false, shared_vm_state=false, cancel_granularity=Cooperative | 実測で確認 |
| R2 | `ScriptBackendRegistry` resolve→open→evaluate `return 42` | resolve 成功→`Integer(42)` | 実測で確認 |
| R3 | 空 registry / 未登録言語 / 未登録 capability（`host.call`）/ ConsumerExtension artifact kind 要求 | fail-closed（`NoMatchingBackend`/`MissingCapability`） | 実測で確認 |

### open_session fail-closed

| # | ケース | E | O |
| --- | --- | --- | --- |
| O1 | language=`rhai` | `LanguageMismatch` | 実測で確認 |
| O2 | source.digest 偽装（text と不一致の digest を持つ ScriptSource） | `SourceDigestMismatch` | 実測で確認 |
| O3 | resume=digest 不一致 | `SourceDigestMismatch` | 実測で確認 |
| O4 | resume=digest 一致 | open 成功 | 実測で確認 |
| O5 | source > 1 MiB | `SourceTooLarge` | 実測で確認 |
| O6 | `max_instructions` 指定 | `UnsupportedBudget{Instructions}` | 実測で確認 |
| O7 | `max_memory_bytes=0` | `UnsupportedBudget{MemoryBytes}` | 実測で確認 |
| O8 | `max_wall_time`/`max_memory_bytes`/`max_output_bytes`/`max_host_calls` 指定 | open 成功（受理） | 実測で確認 |
| O9 | `artifact_kinds`={ConsumerExtension} or 2 件 | `Backend` | 実測で確認 |
| O10 | host binding `os.exec`（validate_neutral 失敗） | `InvalidHostSurface` | 実測で確認 |
| O11 | host_bindings.schema_version 不正 | `InvalidHostSurface` | 実測で確認 |
| O12 | 妥当な host surface（binding 宣言のみ） | open 成功・script から binding 非到達 | 実測で確認 |
| O13 | source.schema_version 不正 | `Backend` | 実測で確認 |

### evaluate / 値写像

| # | ケース | E | O |
| --- | --- | --- | --- |
| V1 | `return 40 + 2` | `Integer(42)` | 実測で確認 |
| V2 | `return 4.5` / `return 2^63` | `Number` | 実測で確認 |
| V3 | `return "hi"` / `return true` / `return nil` / 末尾 return なし | `String`/`Bool`/`default`/`default` | 実測で確認 |
| V4 | `return {1,2,3}` / `return {a=1,b="x"}` / nested | `Array`/`Record`/`Record` | 実測で確認 |
| V5 | `return 1, 2`（multi-return） | `Internal` | 実測で確認 |
| V6 | `return function() end` / `return coroutine.create(function() end)` / `return buffer.create(4)` | `Unsupported` | 実測で確認 |
| V7 | `return {[1]="a",x=1}`（mixed）/ `return {[2]="x"}`（sparse） | `Internal` | 実測で確認 |
| V8 | 深さ >64 の nested table | `Internal` | 実測で確認 |
| V9 | `return string.char(255)`（非 UTF-8） | `Internal` | 実測で確認 |
| V10 | `return 0/0` | `Number(NaN)`（canonical tag record 対応） | 実測で確認 |
| V11 | syntax error source | `Parse` | 実測で確認 |
| V12 | runtime error（`error("boom")` / nil call） | `Runtime` | 実測で確認 |
| V13 | bytecode magic source（`\x1bLuau`） | err（Text mode 拒否 — kind は実測で確定） | 実測で確認 |
| V14 | `entry=Some("main")` | `Unsupported` | 実測で確認 |
| V15 | sandbox: `return type(require)` 等 | `"nil"`（require/loadstring/print/debug 除去） | 実測で確認 |
| V16 | deterministic: 同一 source 2 session | 同一 `ScriptResult` | 実測で確認 |

### budget / cancel

| # | ケース | E | O |
| --- | --- | --- | --- |
| B1 | `max_wall_time` 極小 + `while true do end` | `BudgetExceeded` | 実測で確認 |
| B2 | `max_wall_time` 十分 + 高速 script | 成功 | 実測で確認 |
| B3 | `max_memory_bytes` 小 + 大 allocation | `BudgetExceeded` | 実測で確認 |
| B4 | `max_output_bytes` 小 + 大きい return | `BudgetExceeded` | 実測で確認 |
| B5 | cancel 済み token で evaluate | `Cancelled` | 実測で確認 |
| B6 | `session.cancel()` 後 evaluate | `Cancelled` | 実測で確認 |
| B7 | consumer 側 `cancel_token().cancel()` 後 evaluate | `Cancelled` | 実測で確認 |
| B8 | 別 thread から mid-eval cancel（`while true do end` + thread cancel） | `Cancelled` | 実測で確認 |
| B9 | evaluate 2 回目 | `Internal` | 実測で確認 |
| B10 | close 後 evaluate / close 冪等 | err / 冪等成功 | 実測で確認 |

### artifact

| # | ケース | E | O |
| --- | --- | --- | --- |
| A1 | artifact_kinds={ActionRecipe} + record return | `artifact.kind=ActionRecipe`、payload=record map、`source_digest`/`source_revision` provenance、`produced_by.backend_id=luau-mlua`、digest=`definition_digest(payload)` と一致、`return_value=Some(Record)` | 実測で確認 |
| A2 | artifact_kinds 空 + record return | `artifact=None`、`return_value=Some(Record)` | 実測で確認 |
| A3 | artifact 要求 + 非 Record return（`return 1`） | `Internal` | 実測で確認 |
| A4 | artifact 要求 + `return nil` | `default`（artifact=None） | 実測で確認 |
| A5 | artifact 要求 + 空 record `return {}` | artifact with empty payload | 実測で確認 |

## OS / toolchain 記録

- OS: Windows（worktree `G:\orca\workspaces\...` — NTFS）
- toolchain: `mise.toml` pin（rust 1.99.0 系 — 実測値は
  evidence に記録）
- 実行: `mise run test ruviform-script-luau` 等 — exit code を
  `evidence/rf716/` に記録

## 非対象（Unsupported/NotRun）

- `max_instructions` — **Unsupported**（open_session で
  `UnsupportedBudget{Instructions}` — O6 で検証）
- host call 実行 — **Unsupported**（契約に seam なし — O12 で
  「宣言受理・非実行」を検証）
- named entry points・evaluate 再実行・Resume VM 復元 —
  **Unsupported**（V14/B9 で fail-closed を検証）
- macOS/Linux — **NotRun**（3-OS は Daily/手動 Full CI 委譲）
- 実 OS/実 VM 経路受入 — **consumer gate 委譲**
  （synvelis#516・synvelis#267 AUTX-08 — 本 slice 範囲外）
