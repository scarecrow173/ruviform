---
id: "IMPL-034MS9G9d3xl5J3Q1Wk1bn"
type: "impl"
status: "completed"
title: "Issue #114 real adapter runner bins — Task 3 implementation"
created: "2026-10-27"
updated: "2026-10-27"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/bin/ruviform-plugin-clap-l1-runner.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/bin/ruviform-plugin-vst3-l1-runner.rs"
      - type: "file"
        path: "docs/impl/ir/issue-114-real-adapter-runner-bins-task-3-implementation.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-clap/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/Cargo.toml"
      - type: "file"
        path: "docs/tasks/real-adapter-runner-bins.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/real-adapter-runner-bins.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #114 real adapter runner bins — Task 3 implementation

## Summary

Issue #114 Task 3 完了 — plan actions 10–11（規格別 runner bin — T1/T2
の両 host bridge を消費し、compile-time 固定の fixture factories を
`ruviform_plugin_export::run` に渡す L1 child-process runner）を実施。
`ruviform-plugin-clap` / `ruviform-plugin-vst3` の両 crate に
`[[bin]]` runner を新設した（各 ~128 行・unit test 2 件ずつ）。

- `Cargo.toml`（両 crate・各 +12 行）: `[[bin]] name =
  "ruviform-plugin-clap-l1-runner" / "ruviform-plugin-vst3-l1-runner"
  path = "src/bin/…" required-features = ["host"]` — default
  build では生成されず `--features host` 指定時のみ compile
  （`[features]` 節の直前に配置・役割と `required-features` の
  意図を comment で記録）。
- `src/bin/ruviform-plugin-clap-l1-runner.rs`（新規）: `main() ->
  ExitCode` — fixture 解決 `RUVIFORM_L1_REAL_PLUGIN_DIR`（env
  override・`var_os` で verbatim 取得）→ unset 時 `current_exe`
  sibling（`target/<profile>/`・test bin は `deps/`）→
  `dir.join(fixture_lib_name())`（`{DLL_PREFIX}ruviform_plugin_
  clap{DLL_SUFFIX}` — bridge 公開の共有命名規約）→
  `ClapLibrary::load` → `Err` は stderr diagnostic + exit 1（
  `run` 到達前 = Hello 未送出 → 親 spawn `Err(Ipc)` で収束）→
  `Ok` で `run(lib.factories()…Box<dyn PluginFactory>)`。
- `src/bin/ruviform-plugin-vst3-l1-runner.rs`（新規）: 同型 —
  `Vst3Library::load`・`{DLL_PREFIX}ruviform_plugin_vst3
  {DLL_SUFFIX}`。
- argv 規約不変 — `--ruviform-shm-key` 必須・`--ruviform-plugin-id`
  は `factories.len() != 1` のときのみ必須・未知引数 → usage
  reject exit 2 の `run` 規約を拡張しない（`spec.args` は空のまま
  消費・plugin path/loader 引数は argv に追加しない — design の
  rejected 決定を遵守）。
- `Cargo.lock` 無変更 — `dep:ruviform-plugin-export` optional は
  T1/T2 で追加済み。

敵対的レビュー 1 ラウンド — **clean verdict**（確定修正 0 件）。
informational 指摘 2 件（空文字 env 語義・empty factories → exit
2）は spec-accepted として記録（「Review findings and
resolutions」参照）。全 gate green 実績 + runner 直接起動 smoke
で defined 挙動 pin。

## Context

approved plan `docs/plans/implement-issue-114-real-adapter-l1-e2e.md`
の Task 3（`docs/tasks/real-adapter-runner-bins.md`・actions
10–11）。upstream: approved design
`docs/designs/issue-114-real-adapter-l1-e2e.md`（決定サマリ a/g/h —
「runner」「`spec.args` 不使用」「fixture 解決」）・spec
`docs/specs/issue-114-real-adapter-l1-e2e.md`（proposed・AC1 —
runner オーサリング・compile-time 固定・argv 規約不変）・
`crates/ruviform-plugin-export/src/lib.rs`（`run`/`RunnerArgs::
parse`/`ArgvReject` — argv 規約・exit code 1/2・Hello 未送出契約）
・`crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs`
（`runner_program()` — `current_exe`→`deps/`→`target/<profile>/`
解決先例）・TSpec `docs/test-specs/real-adapter-l1-e2e-tests.md` と
`docs/test-specs/l1-real-plugin-lane-tests.md`（`verified-by`
edge）。Task 1（CLAP bridge）・Task 2（VST3 bridge）の両方に
`depends-on` — runner が各 bridge の `ClapLibrary`/`Vst3Library`・
`factories()`・`fixture_lib_name` を消費する最初の下游。Task 4
（L1 E2E + lane 配線）の直接前提。

## Implementation

### Changes

- `crates/ruviform-plugin-clap/Cargo.toml`（+12 行）: `[features]`
  節の直前に `[[bin]]` block を新設 — `name =
  "ruviform-plugin-clap-l1-runner"`・`path =
  "src/bin/ruviform-plugin-clap-l1-runner.rs"`・
  `required-features = ["host"]`。runner の役割（fixture cdylib を
  `host` bridge 経由で dlopen し scanned factories を `run` に渡す）
  と `required-features` による default build 除外を comment で
  記録。
- `crates/ruviform-plugin-vst3/Cargo.toml`（+12 行）: 同型 —
  `name = "ruviform-plugin-vst3-l1-runner"`。
- `crates/ruviform-plugin-clap/src/bin/ruviform-plugin-clap-l1-runner.rs`
  （新規・128 行）: module doc に runner 全体契約（compile-time
  固定・argv 規約不変・fixture 解決・exit 1 defined rejection の
  意味）を記述。`main() -> ExitCode` は `fixture_path(env::var_os
  (PLUGIN_DIR_ENV))` → `ClapLibrary::load(&path)` →
  `ruviform_plugin_export::run(factories)` の直線構造 —
  `fixture_path`/`exe_dir` はそれぞれ `Err(reason)` で early
  diagnostic + `ExitCode::FAILURE`。helper を env-injectable な
  純粋関数（`fixture_path(Option<OsString>)`）に分離し unit
  test 可能にした。test 2 件: override verbatim join・sibling
  解決 + fixture 実在 assert（欠落は fail — bridge test と同
  規約）。
- `crates/ruviform-plugin-vst3/src/bin/ruviform-plugin-vst3-l1-runner.rs`
  （新規・129 行）: 同型 — `Vst3Library::load`・module doc に
  「`"Audio Module Class"` のみ scan」の VST3 側注記を追加。
- `Cargo.lock` — 変更なし。両 crate の `ruviform-plugin-export`
  optional dep は T1/T2 で導入済み・`[[bin]]` 追加は lock に
  影響しない。

### Review findings and resolutions（敵対的レビュー 1 ラウンド — clean verdict・確定修正 0 件）

- **clean verdict — 確定修正なし**: adversarial review で指摘
  された項目は全て informational / spec-accepted と判定され、
  コード変更は発生しなかった。argv 規約不変・exit code 写像・
  `required-features` 封じ込め・unsafe/alloc 不在（runner 自身は
  safe コードのみ — unsafe 境界は T1/T2 bridge 内に封じ込め済み）
  の各不変条件が確認された。
- **informational（spec-accepted）: 空文字 `RUVIFORM_L1_REAL_PLUGIN_DIR`
  の語義** — `env::var_os` は空文字に対し `Some("")` を返し、
  `PathBuf::from("")` 経由で `"".join(fixture_lib_name())` =
  cwd 相対の fixture 名に解決される。「env が set されていれば
  verbatim で勝つ」規約どおりの defined 挙動であり、空文字を
  unset 扱いにする特別扱いは spec 未要求（out-of-tree layout の
  escape hatch は呼び出し側が正しい値を注入する前提 — shell-only
  の信頼境界）。file 不在に着地して `load` 失敗 → stderr + exit
  1 の defined rejection で収束するため、silent な別経路への
  fallthrough は存在しない。
- **informational（spec-accepted）: empty factories → exit 2** —
  scan 結果が 0 factory の場合 `run(&[])` は argv 規約上
  `--ruviform-plugin-id` 必須（`factories.len() != 1`）に倒れ、
  id 供給時も「no factory for plugin id」の usage reject で
  exit 2 に収束する。fixture は compile-time 固定（CLAP 1
  plugin・VST3 は `"Audio Module Class"` scan）で empty は
  構造的に到達しないため runner での特別扱いは加えず、`run` の
  defined 応答に委譲する構造を維持した。

### Decisions During Implementation

- **env override は `var_os` + verbatim `PathBuf::from`** — 非
  UTF-8 path を落とさない（`var` だと非 UTF-8 で `Err` に倒れ
  unset と区別不能になる）。validation/canonicalize は行わない
  — env は shell-only の信頼注入境界で、誤値は file 不在 →
  `load` 失敗の defined rejection（exit 1）へ自然収束する。
- **pre-`run` 失敗は `ExitCode::FAILURE`（exit 1）+ `eprintln!`
  のみ** — fixture 解決失敗・`load` 失敗は `run` 到達前に
  終了し Hello frame を送らない。親側は spawn 待ちで
  `Err(HostError::Ipc)` に収束する（hang しない defined
  rejection — task doc・`run` 契約で規定済み）。exit code を
  `ExitCode` 型で表現し `process::exit` は使わない（drop
  実行を保つ — `ClapLibrary`/`Vst3Library` の RAII teardown を
  素直に走らせる）。
- **argv 規約の不拡張を構造で担保** — plugin path・loader 引数を
  argv に追加しない（design rejected 決定 — `spec.args` 経由の
  path 受け渡し・loader 型 runner なし）。runner が消費するのは
  env と `current_exe` のみで、`run` へ渡す argv は host が
  spawn 時に組み立てる既存 grammar のみ。
- **file name は bridge 公開の `fixture_lib_name()` を共有** —
  `{DLL_PREFIX}ruviform_plugin_{clap|vst3}{DLL_SUFFIX}` の命名
  規約を runner 側で重複定義せず、T1/T2 が `pub` 公開した
  `fixture_lib_name()`（feature `host` 内）をそのまま消費
  （単一命名規約の authoritative な実装を維持 — runner と
  bridge test が同一規約を共有）。
- **unit test は bridge と同規約** — fixture 欠落は silent skip
  ではなく `assert!` fail。sibling 解決 test は test bin が
  `deps/` に置かれ cargo が同所に fixture cdylib を emit する
  構造を前提にする（`l1_proxy_e2e.rs::runner_program()` と同じ
  解決先例）。
- **`Box<dyn PluginFactory>` への collect は `run` シグネチャ
  そのまま** — `lib.factories()` は load 時に scan 済みの
  cached factory を返す（`Scan` 応答が `descriptor()` 再実行
  しない構造 — task doc の「`Scan` 応答の性質」どおり、dlopen
  や instantiate を `run`/argv 解決で再実行しない）。

## Related Experiments

### Adopted

なし — approved design/plan の確定実装のみ（実験ログなし）。

### Rejected

なし — 代替実装の評価は design の確定事項（`spec.args` 不使用・
loader 型 runner 不採用・compile-time 固定 factories）をそのまま
実装した。

## Validation

- gate（実績・全 green）:
  - `mise run fmt-check` — exit 0（`cargo fmt --all -- --check`）
  - `mise run check` — exit 0（`cargo check --workspace
    --all-targets` — `host` 未指定の default 構成で両 runner
    bin 非 compile・`required-features` で木に出ない）
  - `mise run lint` — exit 0（`cargo clippy --workspace
    --all-targets -- -D warnings` clean）
  - `mise run test` — exit 0（`cargo test --workspace` —
    default 構成無退行）
  - `mise run test-rt` — exit 0（`rt_` filter — 本 task は
    runner bin のみで RT-suitable API 非追加・既存 RT test
    無退行）
- host feature lane（実績・全 green）:
  - `cargo build -p ruviform-plugin-clap -p ruviform-plugin-vst3
    --features host` — exit 0（両 runner bin + fixture cdylib
    が `target/debug/` に生成）
  - `cargo build -p ruviform-plugin-clap -p ruviform-plugin-vst3`
    — exit 0（default・runner bin 非生成・既存 target 無退行）
  - `cargo check`/`cargo clippy --all-targets`/`cargo test`
    を `-p ruviform-plugin-clap`/`-p ruviform-plugin-vst3`
    `--features host` で — 全 exit 0（runner unit test 各 2
    件 — override verbatim join・sibling 解決 + fixture 実在
    assert — を含む crate 全体の test 無退行・clippy 0
    warning）
- runner 直接起動 smoke（shell 実測 — defined 挙動 pin）:
  - 引数なし起動 → exit 2（`current_exe` sibling で fixture
    load 成功 → `run` argv parse が `missing --ruviform-shm-key`
    の usage reject — fixture load が通った証拠 + argv 規約
    不変の確認）
  - `RUVIFORM_L1_REAL_PLUGIN_DIR=<empty dir>` 起動 → stderr
    diagnostic + exit 1（binary 不在の defined rejection —
    `run` 未到達・Hello frame 未送出）
- 対象 test-spec:
  `docs/test-specs/real-adapter-l1-e2e-tests.md`・
  `docs/test-specs/l1-real-plugin-lane-tests.md`（`verified-by`
  edge — runner の defined 挙動・argv 規約・fixture 解決を担保。
  E2E 実測自体は Task 4 scope）

## Non-Goals

- **L1 E2E test・lane 配線（Task 4）** —
  `tests/l1_real_adapter_e2e.rs`・`test-l1-real-plugin`/
  `lint-l1-real-plugin` mise task・`l1-real-verify` ci job・
  `[dev-dependencies] ruviform-plugin-host` は T4 scope。本 task
  は runner bin の供給と shell 直接起動の defined 挙動 pin
  まで。
- **argv grammar の拡張** — plugin path/loader 引数・`spec.args`
  経由の path 受け渡しは design rejected（`run` argv 規約不変
  — 未知引数は usage reject exit 2）。
- **plugin path の自動探索** — fixture 解決は
  `RUVIFORM_L1_REAL_PLUGIN_DIR` → `current_exe` sibling の 2
  段のみ（PATH 探索・registry・規格標準ディレクトリ走査は対象
  外 — compile-time 固定 runner の定義境界）。
- **macOS `.vst3` bundle 解決** — `dir.join(fixture_lib_name())`
  は raw dynamic library path の join のみ（bundle directory の
  component 解約は VST3 bridge `Vst3Library::load` 側の同じ
  定義境界 — Windows/Linux で検証済み・macOS は T4 の 3-OS
  lane での実測対象）。
- **runner 自身の unsafe / RT 検証** — runner は safe コードのみ
  で構成（dlopen/COM の unsafe 境界は T1/T2 bridge 内に封じ込
  め済み）・child process 全体が短命のため `rt_` lane 対象外。

## Risks

- **env override は verbatim 信頼** — `RUVIFORM_L1_REAL_PLUGIN_DIR`
  は shell-only の注入境界で値を無検証で使う（空文字含む —
  spec-accepted 語義）。誤値は必ず file 不在 → `load` 失敗 →
  stderr + exit 1 の defined rejection に収束し、別 dir への
  silent fallthrough は存在しない。
- **sibling 解決は cargo emit 規約に依存** — `current_exe`
  parent と fixture cdylib の同居は `crate-type = ["cdylib",
  "rlib"]` が `target/<profile>/` と `deps/` に emit する
  cargo 規約が前提（`l1_proxy_e2e.rs::runner_program()` と同
  先例）。cargo が emit 先を変えた場合は sibling 解決が失敗し
  defined rejection（exit 1）に倒れる — escape hatch は env
  override。
- **fixture 欠落時の test 失敗** — unit test
  `sibling_resolution_finds_the_workspace_fixture` は `cargo
  test --features host` 実行前に `cargo build --features host`
  で fixture cdylib が必要（bridge test と同規約 — 欠落は
  skip ではなく fail で検出）。
- **empty factories は `run` 側の exit 2 に委譲** — compile-time
  固定 fixture 下では構造的に到達しないが、仮に scan 0 件でも
  runner 側に特別扱いはなく `run` の usage reject（exit 2）
  に収束する（spec-accepted — 親側は `Err(Ipc)`/exit code 検出
  で defined に処理）。

## Follow-ups

- Task 4 `docs/tasks/real-adapter-e2e-and-lane.md` — 本 runner
  bin を `OutOfProcessSpec.program`/`runner_program()` 経由で
  spawn する L1 E2E（`--ignored` gate・`CARGO_BIN_EXE_*`
  解決）+ `test-l1-real-plugin`/`lint-l1-real-plugin` mise
  task + `l1-real-verify` ci job（3-OS — macOS bundle 経路の
  実測を含む）が直接の下游。
- **`cargo test --features host` の前提順序** — runner unit
  test が fixture cdylib 実在を要求するため、lane task（T4）
  側で `cargo build --features host` → `cargo test` の順序を
  pin する責務を持つ（T1/T2 bridge test と同じ前提 —
  `required-features` により default `mise run test` には無関係）。
