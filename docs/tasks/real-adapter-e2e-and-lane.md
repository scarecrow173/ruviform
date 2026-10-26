---
id: "TASK-034MLSyEUOp70P1hRsepM9"
type: "task"
status: "todo"
title: "実プラグイン L1 E2E + opt-in lane（tests/l1_real_adapter_e2e.rs 両規格・mise task・l1-real-verify job・OFX 境界明文化・doc 同期）"
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
    - "docs/tasks/real-adapter-runner-bins.md"
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

# 実プラグイン L1 E2E + opt-in lane（tests/l1_real_adapter_e2e.rs 両規格・mise task・l1-real-verify job・OFX 境界明文化・doc 同期）

## Work

Plan actions 12–14。両 adapter crate に `tests/l1_real_adapter_e2e.rs` を実装し、`PluginHost`/`OutOfProcessSpec` 経由で実規格バイナリの L1 E2E（spawn→handshake→lifecycle→shm block→state 動詞 defined 応答→destroy + kill/欠落異常系）を `#[ignore]` ゲートで pin する。`mise.toml` の 2 lane task・`ci.yml` の `l1-real-verify` job・OFX 境界明文化・doc 同期を配線する。

- **`tests/l1_real_adapter_e2e.rs`（両 crate・新規）**: `#![cfg(feature = "host")]`（feature off で空・default gate 不変）+ 各 `#[test]` に `#[ignore = "<理由: fixture cdylib + runner bin + feature host を要求 — mise run test-l1-real-plugin で実行>"]`。runner は `env!("CARGO_BIN_EXE_<runner>")`（same-crate・`#[cfg]` 内のみ有効）・fixture は `current_exe`→`deps/`→`target/<profile>/` または `deps/` sibling の `{DLL_PREFIX}{name}{DLL_SUFFIX}` を `assert!(path.is_file())`（`runner_program()` 同型・欠落 fail・silent skip なし）
- **主 E2E（両規格 1 本）**: `EXPECTED: PluginDescriptor`（CLAP: `org.ruviform.clap-gain-poc`/`Ruviform Gain PoC`/`Ruviform`/`0.1.0`/`&["audio-effect","stereo"]` — plugin.rs:197-203・VST3: cid hex — **expected id は test が `vst3::uid(0x5A1B2C3D, 0x4E5F6A7B, 0x8C9D0E1F, 0x2A3B4C5D)` raw bytes から実行時に derive**（`vst3` は adapter crate の既存 prod dep で test target から参照可・`GainComponent::CID` は `pub(crate)` で tests/ から不可のため `uid(...)` 呼出しで等価 derive・`id` は `String::leak` で `&'static` 化）・`#[cfg]`-conditional literal は regression pin のみ: Win `"3d2c1b5a5f4e7b6a8c9d0e1f2a3b4c5d"`/非 Win `"5a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d"`）/`Gain`/`Ruviform`/`0.0.0`/`&["Fx"]` — factory.rs・`GainComponent::CID`）を `OutOfProcessSpec::new(EXPECTED, runner)`（`spec.args` 空・既定 `lookahead_blocks=2`/`command_timeout=10s`）→ `PluginHost` で `Scan`→`Instantiate`→`Initialize`→`Activate`→`StartProcessing`→N block `process`（`ParamValue` event 込み — **CLAP normalized v → plain `2v`**（`ParamValue::new(0.25)` → `×0.5`）・**VST3 → plain `v`**（`0.25` → `×0.25`）— delivered block == gain × lookahead 前 input・underrun/pass-through 意味不変）→ `SaveState`/`RestoreState` → `Err(Unsupported)` defined 応答（verb が wire を往復・bridge は規格 state 動詞を呼ばない — `Err(Unsupported)` の pin のみ・偽の round-trip 主張禁止）→ `StopProcessing`→`Deactivate`→`Destroy` → 子 exit(0)
- **異常系 1 本/規格**: `PluginHost::kill_child`（composite.rs:810）→ `Failed` 収束（`wait_host_failed`）→ `HostNotification::InstanceFailed` once（`wait_host_notification` — 二重 emit なし）→ 以後 `Err(HostError::Failed)` 系 defined 拒否・pass-through 意味不変・自動再起動なし + **binary 不在**: `spec.program` を存在しない runner path にして spawn → `Err(HostError::Ipc)`（`Failed` 未到達・notification なし — pre-Hello exit(1) 規約・ハング・silent skip なし）
- **`mise.toml`**: `[tasks.test-l1-real-plugin]`（`run = ["cargo build -p ruviform-plugin-clap -p ruviform-plugin-vst3 --features host", "cargo test -p ruviform-plugin-clap -p ruviform-plugin-vst3 --features host --lib", "cargo test -p ruviform-plugin-clap -p ruviform-plugin-vst3 --features host --test l1_real_adapter_e2e -- --ignored"]` — **3 段**: build で artifact 生成を deterministic に pin → `--lib` で `#[cfg(all(test, feature = "host"))]` の in-process bridge unit test 層を実行（`--test`/`--ignored` 指定では実行されないため明示 step — これを省くと bridge 契約層が CI lane に載らない）→ `--ignored` で E2E — description に fixture+feature 要件を明記）+ `[tasks.lint-l1-real-plugin]`（`run = "cargo clippy -p ruviform-plugin-clap -p ruviform-plugin-vst3 --all-targets --features host -- -D warnings"`）
- **`.github/workflows/ci.yml`**: `l1-real-verify` job を既存 workflow_dispatch workflow 内に追加（**別 workflow ファイル・新規 trigger なし** — `verify`/`icc-verify` 同型 steps: `actions/checkout` → `jdx/mise-action` → `mise install` → `cargo` cache → run）・3-OS matrix（`ubuntu-latest`/`windows-latest`/`macos-latest`・`fail-fast: false` で全 OS 結果収集）・run = `mise run test-l1-real-plugin` + `mise run lint-l1-real-plugin`（local/CI 同一入口 — `test-l1-real-plugin` が build→`--lib` unit test→`--ignored` E2E の 3 段を含むため CI で bridge unit test 層も実行される・ALSA 不要: adapter dep 木に cpal 非出現・`-p` で adapter 2 crate に限定のため `audio-device`/`cpal` は dep 木に出ない）
- **OFX 境界明文化（AC5）**: `ruviform-plugin-ofx`/`VideoPluginInstance` が `run(factories)`/audio shm 面に載らない構造的境界を crate README（または適切節）に明記 — `VideoPluginInstance` は全操作 NONRT・video wire 非確立のため L1 子プロセス経路には載らない・Issue #118 で追跡・trigger = NONRT video port の分離要求
- **doc 同期**: `AGENTS.md` 検証入口 task 表に `test-l1-real-plugin`/`lint-l1-real-plugin` 2 行追加・`crates/README.md` adapter 行更新・`crates/ruviform-plugin-clap/README.md`/`crates/ruviform-plugin-vst3/README.md` に `host` feature/bridge/runner/lane 節（`required-features`・`RUVIFORM_L1_REAL_PLUGIN_DIR`・`mise run test-l1-real-plugin`・`--features host`・`#[ignore]` ゲート・`Err(Unsupported)` state/GUI・`uid()` platform-order id hex の注意）
- **Cargo.toml（両 crate）**: `[dev-dependencies] ruviform-plugin-host = { path = "../ruviform-plugin-host" }`（E2E の `PluginHost`/`OutOfProcessSpec` のため — dev-dep 方向で DAG 非環維持）

## Classification

- `normal-plan-task`（Task 3（runner bins）に依存 — runner + fixture が揃った状態で E2E/lane を配線・final verification（T5）の前提）

## Upstream Documents

- `docs/plans/implement-issue-114-real-adapter-l1-e2e.md`（actions 12–14・Files/Ownership）
- `docs/designs/issue-114-real-adapter-l1-e2e.md`（決定サマリ g/h/i・「E2E」「lane」「OFX 境界」「既知値 const pin」「ParamValue 係数」）
- `docs/specs/issue-114-real-adapter-l1-e2e.md`（AC2–AC7 — E2E・異常系・OFX 境界・CI lane・契約不変）
- `docs/test-specs/real-adapter-l1-e2e-tests.md`（E2E 契約・既知値・ParamValue 係数・kill/欠落系）
- `docs/test-specs/l1-real-plugin-lane-tests.md`（lane 機構・`#[ignore]` ゲート・fixture 欠落 fail・CI job 配線）
- `crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs`（`runner_program()`/`mixed_routes_full_series`/`wait_host_failed`/`wait_host_notification`/`host_block` 先例）
- `crates/ruviform-plugin-export/src/lib.rs`（`run`/`OutOfProcessSpec`/`RunnerArgs` — `spec.args` 空・argv 規約）
- `mise.toml`（`test-rt`/`test-ofx`/`lint-ofx`/`icc-verify` 等既存 lane 先例）
- `.github/workflows/ci.yml`（`verify`/`rt-verify`/`icc-verify` job 先例 — `actions/checkout`/`jdx/mise-action`/`mise install`/`cargo` cache/steps）

## Dependencies

- `docs/plans/implement-issue-114-real-adapter-l1-e2e.md`
- `docs/tasks/real-adapter-runner-bins.md`（runner bin + fixture 解決 — E2E の `CARGO_BIN_EXE_*`/fixture path がこれに依存）
- `docs/tasks/clap-host-bridge.md`・`docs/tasks/vst3-host-bridge.md`（bridge の `Library::load`/`factories()`/`descriptors()` を runner 経由で消費 — bridge 未完了では runner が機能しない）

## Affected Files

- `crates/ruviform-plugin-clap/tests/l1_real_adapter_e2e.rs`（新規）— `#![cfg(feature = "host")]` + `#[ignore]` 全テスト
- `crates/ruviform-plugin-vst3/tests/l1_real_adapter_e2e.rs`（新規）— 同型
- `crates/ruviform-plugin-clap/Cargo.toml` — `[dev-dependencies] ruviform-plugin-host`
- `crates/ruviform-plugin-vst3/Cargo.toml` — `[dev-dependencies] ruviform-plugin-host`
- `mise.toml` — `[tasks.test-l1-real-plugin]` + `[tasks.lint-l1-real-plugin]`
- `.github/workflows/ci.yml` — `l1-real-verify` job（additive・別ファイルなし・workflow_dispatch 維持）
- `AGENTS.md` — 検証入口 task 表に 2 行追加
- `crates/README.md` — adapter 行（`ruviform-plugin-clap`/`ruviform-plugin-vst3`）更新
- `crates/ruviform-plugin-clap/README.md` — `host` feature/bridge/runner/lane 節
- `crates/ruviform-plugin-vst3/README.md` — 同型節 + `uid()` platform-order id hex の注意
- `crates/ruviform-plugin-ofx/README.md`（または適切 crate doc）— OFX 境界明文化（`VideoPluginInstance` NONRT・video wire 非確立・Issue #118 追跡・trigger）
- `Cargo.lock` — `ruviform-plugin-host` dev-dep の差分

## Done When

- [ ] `tests/l1_real_adapter_e2e.rs` が両 crate に `#![cfg(feature = "host")]` + `#[ignore]` 全テストで存在し、`env!("CARGO_BIN_EXE_*")`・`PluginDescriptor` pin（VST3 `id` は `vst3::uid(...)` bytes derive — literal は regression pin のみ）・`spec.args` 空・既定 lookahead/timeout を使う
- [ ] 主 E2E が `PluginHost` 経由で `Scan`→`Instantiate`→`Initialize`→`Activate`→`StartProcessing`→N block `process`（`ParamValue` event 込み・規格別 gain 係数: CLAP `0.25→0.5`・VST3 `0.25→0.25`）→`SaveState`/`RestoreState`→`Err(Unsupported)`→`StopProcessing`→`Deactivate`→`Destroy`→exit(0) を往復する — delivered block == gain × lookahead 前 input・pass-through 意味不変
- [ ] 異常系が `kill_child` → `Failed` + `InstanceFailed` once + `Err(Failed)` 系 defined 拒否・binary 不在 → `Err(Ipc)`（ハング・silent skip なし）を pin
- [ ] `mise.toml` に `test-l1-real-plugin`（build→`--lib` unit test→`--ignored` E2E の 3 段）・`lint-l1-real-plugin`（clippy `--features host -D warnings`）task が存在し、description に fixture+feature 要件を明記
- [ ] `ci.yml` に `l1-real-verify` job が既存 workflow 内 additive で存在（3-OS matrix・`fail-fast: false`・`verify`/`icc-verify` 同型 steps・`mise run test-l1-real-plugin`+`lint-l1-real-plugin`・別 workflow ファイルなし・workflow_dispatch 維持）
- [ ] OFX 境界明文化（`VideoPluginInstance` NONRT・`run`/audio shm 面非搭載・Issue #118 追跡・trigger）が crate README/doc に記載
- [ ] `AGENTS.md` 検証入口 task 表・`crates/README.md` adapter 行・両 crate README に `host` feature/runner/lane/`Err(Unsupported)`/`uid()` platform-order 注意が記載
- [ ] `cargo test --workspace` 等 default gate が本変更で一切差分なし（`#[ignore]`+`#![cfg]`+`required-features`+optional dep の 4 層分離）
- [ ] `mise run test-l1-real-plugin`・`mise run lint-l1-real-plugin` が exit 0（両規格主 E2E + kill + 欠落系全 pass・host feature clippy 0 warning）
- [ ] `cargo tree -p ruviform-plugin-clap -p ruviform-plugin-vst3` で adapter→export/host(dev) 方向・逆向き依存ゼロ・`host` off で新規 dep が non-optional 木に出ない

## Verification

- [ ] `mise run test-l1-real-plugin` — exit 0（`cargo build --features host` → `cargo test --features host --lib`（in-process bridge unit test 層）→ `cargo test --test l1_real_adapter_e2e -- --ignored` — 両規格主 E2E + kill + binary 不在系全 pass）
- [ ] `mise run lint-l1-real-plugin` — exit 0（`cargo clippy -p ... --all-targets --features host -- -D warnings` — host feature 全 target 0 warning）
- [ ] `cargo test -p ruviform-plugin-clap -p ruviform-plugin-vst3 --features host` — exit 0（`--ignored` なし — unit test（bridge contract）のみ実行・`#[ignore]` E2E は除外）
- [ ] `mise run check`・`mise run lint`・`mise run test` — exit 0（default 構成不変グリーン — `#[ignore]`+`#![cfg]`+`required-features`+optional dep の 4 層分離・`l1_real_adapter_e2e.rs` は feature off で空）
- [ ] `cargo test --workspace` — exit 0（`#[ignore]` E2E は実行されない — 既定経路不変グリーン）
- [ ] `cargo tree -p ruviform-plugin-clap -p ruviform-plugin-vst3` — adapter→export/host(dev) 方向・逆向き依存ゼロ・`host` off で新規 dep が non-optional 木に出ない
- [ ] `mise run test-rt` — exit 0・実一致件数が無退行（`rt_` 命名規約維持 — bridge 追加テストに `rt_` 名なし）
