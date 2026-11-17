---
id: "IMPL-716RuntimeBackend0001"
type: "ir"
status: "implemented"
title: "Issue #716 runtime ScriptBackend — 実装記録"
created: "2026-11-17"
updated: "2026-11-17"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/specs/issue-716-runtime-backend.md"
    - "docs/designs/issue-716-runtime-backend.md"
    - "docs/adr/issue-716-runtime-backend.md"
    - "docs/tasks/issue-716-runtime-backend.md"
    - "docs/impl/exp/issue-716-runtime-backend.md"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-script-luau/src/runtime.rs"
      - type: "file"
        path: "crates/ruviform-script-luau/tests/runtime.rs"
      - type: "file"
        path: "docs/impl/ir/issue-716-runtime-backend.md"
      - type: "file"
        path: "docs/impl/exp/issue-716-runtime-backend.md"
      - type: "file"
        path: "docs/impl/exp/issue-716-runtime-backend-exp.jsonl"
      - type: "dir"
        path: "evidence/rf716/"
      - type: "file"
        path: "docs/discovery/issue-716-runtime-backend.md"
      - type: "file"
        path: "docs/specs/issue-716-runtime-backend.md"
      - type: "file"
        path: "docs/designs/issue-716-runtime-backend.md"
      - type: "file"
        path: "docs/plans/issue-716-runtime-backend.md"
      - type: "file"
        path: "docs/tasks/issue-716-runtime-backend.md"
      - type: "file"
        path: "docs/test-specs/issue-716-runtime-backend.md"
      - type: "file"
        path: "docs/adr/issue-716-runtime-backend.md"
    modified:
      - type: "file"
        path: "crates/ruviform-script-luau/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-script-luau/README.md"
      - type: "file"
        path: "crates/ruviform-script-luau/Cargo.toml"
      - type: "file"
        path: "crates/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-716-runtime-backend.md"
    - "docs/designs/issue-716-runtime-backend.md"
    - "docs/adr/issue-716-runtime-backend.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-558-script-contract-design.md"
    - "crates/ruviform-script"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/exp/issue-716-runtime-backend.md"
    - "evidence/rf716/verification.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-716-runtime-backend.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-716-runtime-backend.md"
  verified-by:
    - "evidence/rf716/verification.md"
  references:
    - "docs/specs/issue-716-runtime-backend.md"
    - "docs/designs/issue-716-runtime-backend.md"
    - "docs/adr/issue-716-runtime-backend.md"
    - "docs/test-specs/issue-716-runtime-backend.md"
    - "crates/ruviform-script-luau/src/runtime.rs"
    - "crates/ruviform-script-luau/tests/runtime.rs"
    - "evidence/rf716/verification.md"
  defers:
    - "実 OS/実 VM 経路の受入（consumer gate — synvelis#516・synvelis#267 AUTX-08）"
  deferred-by: []
---

# Issue #716 runtime ScriptBackend — 実装記録

## 実装サマリ

`crates/ruviform-script-luau` に runtime `ScriptBackend`/
`ScriptSession` を実装し、vendored Luau VM（mlua 0.12.2 Rust API
→ luau0-src 0.22.0+luau740 = Luau 0.740）へ接続した。

- **`src/runtime.rs`（新設）**: `LuauBackend`（`ScriptBackend`
  実装 — descriptor 構築 + `open_session` fail-closed 検証）と
  private `LuauSession`（`ScriptSession` 実装 — fresh `Lua`/
  sandbox strip/interrupt budget/cancel/artifact/lift）。
  `unsafe` 追加ゼロ（mlua safe API のみ）。公開 enum 追加なし。
- **`src/lib.rs`**: `mod runtime` + `pub use runtime::LuauBackend`
  + crate docs 更新（runtime 非目標記述の除去・依存選定 ADR 追記）。
- **`tests/runtime.rs`（新設）**: 43 test — descriptor/registry/
  open fail-closed/evaluate 写像/budget/cancel/lifecycle/artifact/
  determinism。全て deterministic OS-L1。
- **`Cargo.toml`/`README.md`/`crates/README.md`**: runtime 対応の
  記述更新（依存自体は変更なし — mlua 既依存を Rust API として
  使用開始）。`Cargo.lock` 差分なし。

## 設計判断の実装対応

- **依存選定**: mlua Rust API 採用（ADR
  `issue-716-runtime-backend.md` — #692 AC4 再評価確定）。
  analyzer=native bridge / runtime=mlua の hybrid 構成。
- **descriptor**: `backend_id="luau-mlua"`、language=`luau`、
  `impl_version="<crate_ver>+luau<rev>"`（shim `bridge_versions()`
  の Luau revision を provenance 合成）、contract_version=1、
  artifact_kinds={ActionRecipe,WorkflowDefinition}、
  capability_ids={"session.evaluate"}、owner_thread_only=true、
  send/sync/shared_vm_state=false、cancel_granularity=Cooperative。
- **open_session fail-closed**: language → host surface
  （validate_neutral + schema_version）→ source size（1 MiB）→
  source digest 再計算検証 → resume digest → source schema_version
  → budget 次元（Instructions 拒否・MemoryBytes 0/usize 拒否・
  WallTime overflow 拒否）→ artifact kind（広告外/複数拒否）→
  VM 生成。全検証を VM 生成前に実行。
- **budget**: wall_time=interrupt deadline（`Instant+Duration`
  checked_add で overflow 拒否）、memory=`set_memory_limit`、
  output=eval 後 canonical 直列化サイズ、host_calls=vacuous 受理、
  instructions=UnsupportedBudget。
- **cancel**: contract `CancelToken` を interrupt closure へ clone
  注入 + hit flag（cancel/deadline 区別）を post-eval 再検査
  （pcall 飲み込み対策）。`cancel_token()` は共有 token の clone
  を返し consumer 側 cancel も観測。
- **evaluate**: single-shot（2 回目 Internal）・entry→Unsupported
  ・closed→Internal・`ChunkMode::Text` のみ（bytecode 拒否）・
  MultiValue 0→default/1→lift/>1→Internal・nil→default。
- **artifact**: `artifact_kinds` 1 件時に lift 値が Record なら
  `ScriptArtifact::new`（payload/source/provenance/digest は
  contract が計算）— 非 Record→Internal、nil→default。
- **値 lift**: depth≤64・nodes≤1M・string key→Record/連続 int
  key→Array/空→Record・mixed/sparse→Internal・非 UTF-8→Internal・
  Function/Thread/UserData 等→Unsupported・Number は整数かつ
  i64 範囲内→Integer 降格（非有限は Number のまま canonical tag
  record で決定的）。
- **sandbox**: `StdLib::ALL_SAFE` + `require`/`loadstring`/
  `print`/`debug` を Nil で除去（digest pin 外 code 実行・stdout
  汚染・debug 公開の遮断 — `type(require)=="nil"` 等を test で
  観測）。

## 検証（canonical = mise、Windows x86_64）

| コマンド | 結果 |
| --- | --- |
| `mise run check ruviform-script-luau` | exit 0 |
| `mise run lint ruviform-script-luau` | exit 0（`-D warnings`） |
| `mise run test ruviform-script-luau` | exit 0 — 91 tests pass（unit 16 + bridge 22 + faults 1 + registry 9 + runtime 43） |
| `mise run test ruviform-script` | exit 0 — 非回帰 |
| `mise run fmt-check` | exit 0 |
| `mise exec -- prek run --all-files` | （実施結果を evidence に記録） |

詳細な expected/observed は `evidence/rf716/verification.md` と
`docs/impl/exp/issue-716-runtime-backend.md`（+ `-exp.jsonl`）を
参照。実 OS/実 VM 経路の受入は consumer gate（synvelis#516・
synvelis#267 AUTX-08）へ委譲 — 本 slice は deterministic OS-L1
のみ。
