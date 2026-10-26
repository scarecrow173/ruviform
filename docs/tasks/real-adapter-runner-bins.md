---
id: "TASK-034MLSxXJkDzlYvxqtGWDY"
type: "task"
status: "todo"
title: "規格別 runner bin（required-features=[\"host\"]・DLL_PREFIX fixture 解決・RUVIFORM_L1_REAL_PLUGIN_DIR・compile-time 固定 factories）"
created: "2026-10-26"
updated: "2026-10-27"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-114-real-adapter-l1-e2e.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-114-real-adapter-l1-e2e.md"
    - "docs/tasks/clap-host-bridge.md"
    - "docs/tasks/vst3-host-bridge.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-114-real-adapter-l1-e2e.md"
    - "docs/designs/issue-114-real-adapter-l1-e2e.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/real-adapter-l1-e2e-tests.md"
    - "docs/test-specs/l1-real-plugin-lane-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# 規格別 runner bin（required-features=["host"]・DLL_PREFIX fixture 解決・RUVIFORM_L1_REAL_PLUGIN_DIR・compile-time 固定 factories）

## Work

Plan actions 10–11。両 adapter crate に規格別 runner bin を実装する — compile-time 固定の fixture factories を `ruviform_plugin_export::run(factories)` に渡し、`RUVIFORM_L1_REAL_PLUGIN_DIR`/`current_exe` sibling で fixture cdylib を解決する。**`spec.args` は不使用**（`run` argv 規約の拡張禁止 — 未知引数は usage reject → exit 2）。

- **`Cargo.toml`（両 crate）**: `[[bin]] name = "ruviform-plugin-clap-l1-runner" path = "src/bin/ruviform-plugin-clap-l1-runner.rs" required-features = ["host"]` / `[[bin]] name = "ruviform-plugin-vst3-l1-runner" path = "src/bin/ruviform-plugin-vst3-l1-runner.rs" required-features = ["host"]` — default build で生成されない（`required-features` で feature `host` 指定時のみ compile）
- **`src/bin/ruviform-plugin-clap-l1-runner.rs`（新規）**: `main() -> std::process::ExitCode` — fixture 解決 `RUVIFORM_L1_REAL_PLUGIN_DIR` env（任意 shell-only override・`var_os` で `PathBuf` 取得）→ unset 時 `current_exe` sibling（`env::current_exe().parent()` — `target/<profile>/` または `deps/`）→ `dir.join(format!("{}ruviform_plugin_clap{}", env::consts::DLL_PREFIX, env::consts::DLL_SUFFIX))` → `ClapLibrary::load(&path)` → `Err` は `eprintln!`（`{:?}` or `Display`）+ `exit(1)`（**`run` 到達前 = Hello 未送出 → 親 spawn `Err(Ipc)`**）→ `Ok` で `ruviform_plugin_export::run(lib.factories().into_iter().map(|f| Box::new(f) as Box<dyn ruviform_plugin_core::PluginFactory>).collect())`
- **`src/bin/ruviform-plugin-vst3-l1-runner.rs`（新規）**: 同型 — `dir.join(format!("{}ruviform_plugin_vst3{}", DLL_PREFIX, DLL_SUFFIX))` → `Vst3Library::load(&path)` → `run(lib.factories()...)`
- **argv 規約不変**: `run` の argv 規約（`--ruviform-shm-key`（必須 — 欠落 → `ArgvReject::Usage` → exit 2）・`--ruviform-plugin-id`（**`factories.len() != 1` のときのみ必須** — 単一 factory なら省略可だが host は常時 append: spawn argv が `spec.descriptor.id` を必ず渡す — l1/mod.rs:1007-1011・非一致 id → `ArgvReject::Usage`「no factory for plugin id」→ exit 2・`descriptor()` panic → `ArgvReject::Panic` → exit 1 — export/lib.rs:353-372）・**未知引数 → `ArgvReject::Usage` → exit 2**）は拡張しない — `spec.args` は空のまま消費される。plugin path/loader 引数を argv に追加しない（ユーザー決定で rejected — `spec.args` 経由 path 受け渡し・loader 型 runner なし）
- **`Scan` 応答の性質**: `Scan` は load 時 cached descriptor を返す（`descriptor()` が plugin バイナリ依存の失敗を持たない構造 — `Scan`/argv 解決で dlopen 再実行しない — `Library::load` で scan 済みの factories を `run` に渡す）
- **runner smoke gate**: `cargo build -p ruviform-plugin-clap -p ruviform-plugin-vst3 --features host` で両 runner bin + fixture cdylib 生成を確認 + shell 直接起動の defined 挙動 pin — (i) 引数なし起動: fixture load 成功後 `run` argv parse が `missing --ruviform-shm-key` → exit 2（fixture load が通った証拠 + usage reject）・(ii) `RUVIFORM_L1_REAL_PLUGIN_DIR=<empty dir>` 起動 → stderr diagnostic + exit 1（binary 不在 defined rejection — `run` 未到達）

## Classification

- `normal-plan-task`（Task 1（CLAP bridge）と Task 2（VST3 bridge）の両方に依存 — runner が各 bridge API を消費・E2E（T4）の前提）

## Upstream Documents

- `docs/plans/implement-issue-114-real-adapter-l1-e2e.md`（actions 10–11・Files/Ownership）
- `docs/designs/issue-114-real-adapter-l1-e2e.md`（決定サマリ a/g/h・「runner」「`spec.args` 不使用」「fixture 解決」）
- `docs/specs/issue-114-real-adapter-l1-e2e.md`（AC1 — runner オーサリング・compile-time 固定・argv 規約不変）
- `crates/ruviform-plugin-export/src/lib.rs`（`run`/`RunnerArgs::parse`/`ArgvReject::Usage` — argv 規約・exit code 1/2・Hello 未送出契約）
- `crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs`（`runner_program()` — `current_exe`→`deps/`→`target/<profile>/` 解決先例）

## Dependencies

- `docs/plans/implement-issue-114-real-adapter-l1-e2e.md`
- `docs/tasks/clap-host-bridge.md`（`ClapLibrary`/`ClapPluginFactory`/`factories()` を消費）
- `docs/tasks/vst3-host-bridge.md`（`Vst3Library`/`Vst3PluginFactory`/`factories()` を消費）

## Affected Files

- `crates/ruviform-plugin-clap/Cargo.toml` — `[[bin]] name = "ruviform-plugin-clap-l1-runner" path = "src/bin/ruviform-plugin-clap-l1-runner.rs" required-features = ["host"]`
- `crates/ruviform-plugin-vst3/Cargo.toml` — `[[bin]] name = "ruviform-plugin-vst3-l1-runner" path = "src/bin/ruviform-plugin-vst3-l1-runner.rs" required-features = ["host"]`
- `crates/ruviform-plugin-clap/src/bin/ruviform-plugin-clap-l1-runner.rs`（新規）— `main() -> ExitCode`・fixture 解決・`ClapLibrary::load`・`run`
- `crates/ruviform-plugin-vst3/src/bin/ruviform-plugin-vst3-l1-runner.rs`（新規）— 同型・`Vst3Library::load`・`run`
- `Cargo.lock` — `dep:` optional（両 crate の `ruviform-plugin-export` — T1/T2 で追加済み前提）

## Done When

- [ ] `[[bin]] required-features = ["host"]` が両 crate に存在し、default build（`cargo build -p ...` `host` 未指定）では生成されず `--features host` でのみ生成される
- [ ] CLAP runner が `RUVIFORM_L1_REAL_PLUGIN_DIR`/`current_exe` sibling の `{DLL_PREFIX}ruviform_plugin_clap{DLL_SUFFIX}` で fixture を解決し `ClapLibrary::load` → `run` を駆動する
- [ ] VST3 runner が同型で `Vst3Library::load` → `run` を駆動する
- [ ] 引数なし起動で fixture load 成功後 `run` argv parse が `missing --ruviform-shm-key` → exit 2（fixture load 証拠 + usage reject）— shell 直接起動で確認
- [ ] `RUVIFORM_L1_REAL_PLUGIN_DIR=<empty dir>` 起動で stderr diagnostic + exit 1（binary 不在 defined rejection — `run` 未到達）
- [ ] `spec.args` が空のまま消費され、`run` argv 規約（`--ruviform-shm-key` 必須・`--ruviform-plugin-id` は `factories.len() != 1` のときのみ必須・非一致 id/未知引数 → usage reject exit 2・`descriptor()` panic → exit 1）が拡張されない
- [ ] narrow gate（下記 Verification）が exit 0

## Verification

- [ ] `cargo build -p ruviform-plugin-clap -p ruviform-plugin-vst3 --features host` — exit 0（両 runner bin + fixture cdylib 生成）
- [ ] `cargo build -p ruviform-plugin-clap -p ruviform-plugin-vst3` — exit 0（default・`required-features` で runner bin 非生成・既存 bin/target 無退行）
- [ ] `cargo test -p ruviform-plugin-clap --features host` ・ `cargo test -p ruviform-plugin-vst3 --features host` — exit 0（`CARGO_BIN_EXE_*` が same-crate runner bin を解決）
- [ ] `mise run check`・`mise run lint`・`mise run test` — exit 0（default 構成不変グリーン — `required-features` で runner bin は default 木に出ない）
- [ ] runner 直接起動（shell・引数なし）→ exit 2・`RUVIFORM_L1_REAL_PLUGIN_DIR=<empty>` 起動 → stderr + exit 1 — defined 挙動の shell 確認
