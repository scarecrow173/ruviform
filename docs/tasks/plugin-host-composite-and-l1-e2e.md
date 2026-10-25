---
id: "TASK-034LNYndN4rzshflcqGHt5"
type: "task"
status: "todo"
title: "PluginHost コンポジット + 選択 API + doc 整合"
created: "2026-10-25"
updated: "2026-10-25"
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
    - "docs/plans/implement-issue-76-plugin-l1-out-of-process.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-76-plugin-l1-out-of-process.md"
    - "docs/tasks/l1-shm-data-plane-and-rt-legs.md"
    - "docs/tasks/plugin-export-crate-and-l1-runner.md"
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
    - "docs/test-specs/plugin-l1-proxy-contract-tests.md"
    - "docs/test-specs/plugin-l1-runner-e2e-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# PluginHost コンポジット + 選択 API + doc 整合

## Work

approved plan `implement-issue-76-plugin-l1-out-of-process.md` の **actions 15–17 を 1 task の ordered substeps として実行**。`PluginHost` コンポジットの公開面を実装する — L0/L1 の opt-in ルーティングと統一ハンドル空間・公開通知面・crate/リポジトリ doc 整合 — そして runner bin を経路に使う公開面 E2E。

**先行確認**: `InProcessHost` のスロット/generation 規則と `PluginHandle` の mint 形（`#[doc(hidden)] pub new`）を確認 — 統一ハンドル空間が L0 と同一規則で成り立つことを確認。

### A15 — `PluginHost` コンポジット（`src/composite.rs`）

- `crates/ruviform-plugin-host/src/composite.rs` 新設 — 公開 API:
  - `PluginHost::new`/`register_factory`（L0 ルート）・`register_out_of_process(OutOfProcessSpec) -> Result<(), HostError>`（L1 ルート — plugin_id → L1 route を登録時に宣言）
  - `execute`/`process`/`lifecycle_state`/`plugin_mut`（L0 スロットのみ — L1 は `ErasedPlugin` を持たない `None`）・`try_recv_notification() -> Option<HostNotification>`・`#[doc(hidden)] kill_child`（検証支援）
  - `OutOfProcessSpec`（`descriptor: PluginDescriptor`・`program: OsString`・`args: Vec<OsString>`・`lookahead_blocks: u32` 既定 2・`command_timeout: Duration` 既定 10s）
  - `HostNotification::InstanceFailed { handle: PluginHandle, exit: ChildExit }`（`#[non_exhaustive]`）・`ChildExit { code: Option<i32>, signal: Option<i32> }`（`signal` unix のみ）
- plugin_id→route 表（L0 `register_factory` と L1 `register_out_of_process` で同一 plugin_id → `Err(HostError::DuplicatePluginId)`）・統一スロット表（`PluginHandle` slot/generation 規則 L0 同一 — `Instantiate{plugin_id}` で呼出し側は層を意識しない）
- `lib.rs` re-export + crate doc — L1 経路・遅延契約（`lookahead_blocks × block_frames / sample_rate` — 2 × 512f @ 48 kHz ≈ 21.3 ms）・RT 契約（try 脚のみ）を明記

### A16 — doc 整合

- `crates/ruviform-plugin-host/README.md`（L1 バックエンド・許可依存 `ruviform-rt`+`shared_memory`・RT 契約・遅延契約・`l1` は `pub(crate)`）・`crates/README.md` Host 層行・root `Cargo.toml` members コメント同期・plugin-export crate doc/README の最終一致

### A17 — 公開面 E2E

- `tests/l1_proxy_e2e.rs` 拡張: `PluginHost` 経由の L0+L1 混在 scan→instantiate→lifecycle→process→state→destroy — `ruviform-plugin-l1-runner`（export fixture）を `OutOfProcessSpec.program` に持つ結合を含む
- L0 無退行（`InProcessHost` の既存テストが全パス・`plugin_mut` が L0 スロットを返す）・L1 の `plugin_mut` は `None`・重複 plugin_id → `DuplicatePluginId`・L1 kill → `InstanceFailed` + 拒否

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-76-plugin-l1-out-of-process.md](../plans/implement-issue-76-plugin-l1-out-of-process.md)
- [../specs/plugin-l1-out-of-process.md](../specs/plugin-l1-out-of-process.md)
- [../designs/plugin-l1-out-of-process.md](../designs/plugin-l1-out-of-process.md)
- [../designs/plugin-isolation.md](../designs/plugin-isolation.md)
- [../test-specs/plugin-l1-proxy-contract-tests.md](../test-specs/plugin-l1-proxy-contract-tests.md)
- [../test-specs/plugin-l1-runner-e2e-tests.md](../test-specs/plugin-l1-runner-e2e-tests.md)

## Dependencies

- 先行 task: [l1-shm-data-plane-and-rt-legs.md](l1-shm-data-plane-and-rt-legs.md)（L1 プロキシ + データ面 + RT 脚）・[plugin-export-crate-and-l1-runner.md](plugin-export-crate-and-l1-runner.md)（runner bin を `OutOfProcessSpec.program` に使う結合 E2E）

## Non-Goals

- エンジン/利用側への L1 選択配線（consumer 側責務 — plugin-host の engine 消費自体が未実装）
- `InProcessHost` の挙動変更（L0 は無変更 — `PluginHost` がコンポジット）・自動再起動・L1 GUI（`Unsupported` 拒否のまま）
- 実プラグインの L1 E2E — `l1-test-worker`/`ruviform-plugin-l1-runner` fixture で検証

## Affected Files

- `crates/ruviform-plugin-host/src/composite.rs`（新規 — `PluginHost`/`OutOfProcessSpec`/`HostNotification`/`ChildExit`・統一スロット表）
- `crates/ruviform-plugin-host/src/lib.rs`（`mod composite` + re-export + crate doc — 遅延契約明記）
- `crates/ruviform-plugin-host/README.md`（L1 バックエンド・許可依存・RT 契約）
- `crates/README.md`（Host 層行更新）
- `Cargo.toml`（root — members コメント同期）
- `crates/ruviform-plugin-export/src/lib.rs`・`README.md`（crate doc/README の最終一致）
- `crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs`（`PluginHost` 経由 L0+L1 混在 E2E — runner bin 結合）

## Completion Gate

5 entry 全 green 実 log（`fmt-check`/`check`/`lint`/`test`/`test-rt` — `test-rt` は `rt_l1_*` 実一致件数記録）+ lead review の後にのみ task 完了・buildable scope commit。review 焦点: `PluginHost` の統一ハンドル空間・登録時ルーティング・`DuplicatePluginId` が「呼出し側は層を意識しない」契約を壊さない・`InProcessHost` 無退行・crate doc の遅延契約明記 — fake success なし。

## Verification

- [ ] `mise run fmt-check` — exit 0
- [ ] `mise run check` — exit 0
- [ ] `mise run lint` — exit 0（clippy `--all-targets -D warnings` clean）
- [ ] `mise run test` — exit 0（`PluginHost` E2E 全 pass — L0+L1 混在・runner bin 結合・`DuplicatePluginId`・L1 kill→`InstanceFailed`・L0 無退行）
- [ ] `mise run test-rt` — exit 0 かつ `rt_l1_*` が実一致（一致件数記録）
- [ ] `rg -n "pub struct PluginHost\|pub struct OutOfProcessSpec\|pub enum HostNotification\|pub struct ChildExit" crates/ruviform-plugin-host/src/composite.rs` — 公開型の存在
- [ ] `rg -n "lookahead_blocks.*block_frames.*sample_rate\|遅延" crates/ruviform-plugin-host/src/lib.rs` — 遅延契約の doc 明記
