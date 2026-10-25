---
id: "TASK-034LNYeFTd7EgCwbDA0GWD"
type: "task"
status: "done"
title: "ruviform-plugin-export 新設 + run() ランナー + l1-runner bin"
created: "2026-10-25"
updated: "2026-10-25"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-export/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-export/README.md"
      - type: "file"
        path: "crates/ruviform-plugin-export/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/src/shm.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/src/bin/ruviform-plugin-l1-runner.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/tests/l1_runner_e2e.rs"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/wire.rs"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-76-plugin-l1-out-of-process.md"
  implemented-by:
    - "docs/impl/ir/issue-76-plugin-export-task-2-implementation.md"
  depends-on:
    - "docs/plans/implement-issue-76-plugin-l1-out-of-process.md"
    - "docs/tasks/l1-core-contracts-and-wire-codec.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/plugin-l1-runner-e2e-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# ruviform-plugin-export 新設 + run() ランナー + l1-runner bin

## Work

approved plan `implement-issue-76-plugin-l1-out-of-process.md` の **actions 5–8 を 1 task の ordered substeps として実行**。`ruviform-plugin-export` クレートを新設し、子プロセス側の薄いランタイム `run(factories)`・内蔵モック `GainPlugin` を持つ参照 runner `ruviform-plugin-l1-runner`・`CARGO_BIN_EXE_*` spawn E2E を実装する。

**先行確認**: `shared_memory` の実効 API 形状（`ShmemConf`/`os_id`/`as_slice`/`as_mut_slice`・attach の API 名）を docs.rs/`cargo doc` で確認 — design 記載と相違なら blocker を記録（fake success なし・自前 `unsafe` 代替禁止）。

### A5 — クレート骨格

- `crates/ruviform-plugin-export/Cargo.toml`: `ruviform-plugin-core = { path = "../ruviform-plugin-core" }` + `shared_memory = "0.12"`（子側 shm attach — plan「依存の確定」どおり・採用記録/隔離条件は host と同一）・`publish.workspace = true` 継承・`[lints] workspace`
- `src/lib.rs` crate doc — 責務（薄いランナー・`run` が唯一の公開エントリ）・argv 規約・runner 内 unsafe ゼロを明記
- `README.md`（crate 索引どおりの責務・許可依存・RT 制約の記述）・root `Cargo.toml` `members` += `crates/ruviform-plugin-export` + 先頭コメントのクレート列記同期・`crates/README.md` 索引行追加（計 21 クレート）

### A6 — `run(factories) -> ExitCode`

- `pub fn run(factories: Vec<Box<dyn PluginFactory>>) -> std::process::ExitCode` — `Box<dyn>` でアダプタ非依存
- argv 解析（`--ruviform-plugin-id <id>`・`--ruviform-shm-key <key>` — 不足・未知引数は非ゼロ終了・ハングしない）→ `Hello { version: WIRE_VERSION, capabilities }` を最初のフレームで送信 → shm attach + `[header][in-slots][out-slots]` header 検証（内部 `shm` モジュール — `shared_memory` を非公開閉じ込め・`as_slice`/`as_mut_slice` のみ・自前 `unsafe` ゼロ）
- シングルスレッドメッセージループ: stdin frame → `wire::Command` を `ErasedPlugin` へ dispatch → `Reply { seq, result: Result<wire::Reply, HostError> }`。`BlockIn { seq }` → `Lifecycle` が `Processing` を確認後 `process_host`（in-slot `BufferView`・out-slot `BufferViewMut`）→ `BlockDone { seq, result }`。`Shutdown`/stdin EOF → lifecycle 正規化 → exit(0)
- `ErasedPlugin` dispatch の panic は catch して `Err(HostError::PluginFailed)` として `Reply`/`BlockDone` で返す — 子プロセスを殺さない
- 違法遷移は `Lifecycle` が `Err(HostError::Lifecycle(_))` — wire `Reply` で返す（panic しない）

### A7 — `src/bin/ruviform-plugin-l1-runner`

- 内蔵モック `GainPlugin`（core `Plugin` 実装 — planar `f32` に `gain` を乗算・`PluginDescriptor` 保持・`Initialize`/`Activate`/`StartProcessing`/`SaveState`/`RestoreState`/`Deactivate`/`Destroy` の state 往復・param event 反映）+ `run` 呼出し
- fixture — `CARGO_BIN_EXE_ruviform-plugin-l1-runner` で spawn 検証

### A8 — E2E（tests/l1_runner_e2e.rs）

- `tests/l1_runner_e2e.rs`: `env!("CARGO_BIN_EXE_ruviform-plugin-l1-runner")`/`option_env!` で spawn（`tests/` 内に閉じる — cross-package bin 解決に依存しない）・test 側最小 host で codec/shm を組み立て
- 経路: `Hello` → `Scan`（`Scanned` の `wire::Descriptor` が `GainPlugin` descriptor とフィールド等価）→ `Initialize`/`Activate`/`StartProcessing` → shm in-slot に既知入力 → `BlockIn` → `BlockDone` → out-slot gain 正当性 → `SaveState`/`RestoreState` 往復 → `StopProcessing`/`Deactivate` → `Shutdown` exit(0)
- 異常系: argv 不足/未知引数・shm header 不一致・panic→`PluginFailed`（子生存）・非 `Processing` `BlockIn`→`Err(Lifecycle(_))`・version 不一致（親側 kill — test-spec `plugin-l1-runner-e2e-tests.md` の契約）

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-76-plugin-l1-out-of-process.md](../plans/implement-issue-76-plugin-l1-out-of-process.md)
- [../specs/plugin-l1-out-of-process.md](../specs/plugin-l1-out-of-process.md)
- [../designs/plugin-l1-out-of-process.md](../designs/plugin-l1-out-of-process.md)
- [../test-specs/plugin-l1-runner-e2e-tests.md](../test-specs/plugin-l1-runner-e2e-tests.md)
- [../test-specs/plugin-l1-wire-codec-tests.md](../test-specs/plugin-l1-wire-codec-tests.md)

## Dependencies

- 先行 task: [l1-core-contracts-and-wire-codec.md](l1-core-contracts-and-wire-codec.md) — wire codec・`ErasedPlugin`・移設契約（`Plugin`/`PluginFactory`/`HostError`/`PluginHandle`）が前提
- `shared_memory = "0.12"` — plan「依存の確定」どおり（design の採用記録を子側 attach に同一条件で適用）

## Non-Goals

- proxy/host 側の `Failed` 遷移・pass-through・RT 脚・`PluginHost` コンポジット（後続 task）
- 実プラグイン（CLAP/VST3/OFX）の L1 組込み — `GainPlugin` モックで検証
- bundle 子・wire バージョン進化・stderr 配管・自動再起動・shm slot レイアウトの所有（host `l1/shm.rs` が作成 — runner は attach/検証のみ）

## Affected Files

- `crates/ruviform-plugin-export/Cargo.toml`・`src/lib.rs`・`README.md`（新規 — `run`・内部 `shm` モジュール・crate doc）
- `crates/ruviform-plugin-export/src/bin/ruviform-plugin-l1-runner.rs`（新規 — `GainPlugin` fixture）
- `crates/ruviform-plugin-export/tests/l1_runner_e2e.rs`（新規 — spawn E2E）
- `Cargo.toml`（root — `members` += export・コメント同期）
- `crates/README.md`（索引行追加・計 21 クレート）
- `Cargo.lock`（`shared_memory` 等の依存追加を記録）

## Completion Gate

3 entry 全 green 実 log（`check`/`lint`/`test`）+ lead review の後にのみ task 完了・buildable scope commit。review 焦点: `run` が唯一の公開エントリ・argv/shm header 異常の定義済み拒否・panic→`PluginFailed`（子生存）・`shared_memory` が内部 `shm` モジュールに閉じ込められ公開 API に露出しない・自前 `unsafe` ゼロ — fake success なし。

## Verification

- [x] `mise run check` — exit 0（新規 crate の型チェック・実 log）
- [x] `mise run lint` — exit 0（clippy `--all-targets -D warnings` clean）
- [x] `mise run test` — exit 0（runner E2E 11 tests 全 pass — 正常系 lifecycle + gain 正当性 + 異常系拒否群 — workspace 55 suites・実 log）
- [x] `rg -n "ruviform-plugin-export" Cargo.toml crates/README.md` — workspace member + 索引行の存在（計 21 クレート）
- [x] `cargo tree -p ruviform-plugin-export` — `ruviform-plugin-core` + `shared_memory`（+ transitive）のみ（dev-deps: audio-core/media-core）
- [x] `rg -n "unsafe" crates/ruviform-plugin-export/src` — scoped `#[expect(unsafe_code)]` ×2（`shm.rs` `bytes`/`bytes_mut`・SAFETY コメント付き — deviation: `shared_memory` 0.12.4 の `as_slice`/`as_slice_mut` は `unsafe fn`。task doc の「自前 unsafe ゼロ」想定は設計記載と異なり、局所許可として記録 — 詳細は IR）

補足（task doc の Verification 範囲外・IR に記録）: `mise run fmt-check` exit 0・`mise run test-rt` exit 0（既存レーン無退行 — export は非 RT のみ）。レビュー修正（guarded `descriptor()` panic・`staging_out` 再ゼロ化・argv exit-2 pin・bounded inbound label・`wire::shm` 正本化）は mutation 検証済み — 詳細は `docs/impl/ir/issue-76-plugin-export-task-2-implementation.md`。
