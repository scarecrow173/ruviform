---
id: "DESIGN-034XGratfpozML30kehPqZ"
type: "design"
status: "approved"
title: "Issue #389 dock-test session-lifecycle cases 設計（PluginEditorSessions 消費・fake-slot redeemer seam 再利用・正直 verdict）"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-389-session-lifecycle.md"
    - "https://github.com/scarecrow173/ruviform/issues/389"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-389-session-lifecycle.md"
    modified:
      - type: "doc"
        path: "docs/designs/issue-389-plugin-editor-session.md"
  deleted: []
  renamed: []
  moved: []
  generated: []
  implements:
    - "docs/specs/issue-389-session-lifecycle.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-389-session-lifecycle.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-389-plugin-editor-session.md"
    - "docs/designs/issue-631-plugin-dock-scenario.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-389-session-lifecycle.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-389-session-lifecycle.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #389 dock-test session-lifecycle cases 設計（PluginEditorSessions 消費・fake-slot redeemer seam 再利用・正直 verdict）

## Context

`ruviform-plugin-dock-test` は test-only の consumer composition root で、plugin crate と UI crate が互いに依存できない制約の下で両者を結合する。既存 harness は `PluginHostProbe` driver・`probe_spec`/`finalize`/`case_environment` の共通パターンで、artifact 解決（env var → `deps/` → 実行ファイル dir）→ SHA-256 → `Vst3Library` load → `InProcessHost` instantiate/initialize/activate → destroy の実バイナリ経路を fail-closed（artifact 不在 = 全 dependent case `NotRun`）で観測する。

session API（`crates/ruviform-plugin-host/src/editor_session.rs`、読み取りのみ）は driver-neutral で UI-thread/non-RT。`PluginEditorSessions` registry が instance ごとの live session を管理し、`begin_session`/`end_session`/`session_for_instance`/`notify_terminated` を提供する。`attempt_editor_attach`（`scenario.rs`）が既に確立した seam — `FakeNativeSlot::declare_slot` → `lease_parent` → `SlotLease` → `ParentSlotLease` → issuer 側 redeemer が `validate_lease` で `ParentLease` を鋳造 → `InProcessEditorOps::with_redeemer` → `session.open` — を lifecycle case も再利用する。

## Scope

### In Scope

- `scenario.rs` への 3 case 追加と共有 helper（`SessionCaseState`・redeemer seam helper・`CountingEditorOps`・`teardown_editor_case`）
- `attempt_editor_attach` の redeemer closure を共有 helper へ集約（挙動不変の局所リファクタ）
- `tests/plugin_dock.rs`・`README.md` の追従
- `docs/designs/issue-389-plugin-editor-session.md` の verification-status reconcile

### Out of Scope

- session API・plugin-host/plugin-core/plugin-vst3/plugin-clap/ui-test の変更
- 実 attach 成功経路・実 window・実画素・L2/L3
- `mise.toml`・`Cargo.toml`/`Cargo.lock`（新規依存なし想定）

## Components and Boundaries

- `SessionCaseState`（crate-private）: `PluginEditorSessions`・`FakeNativeSlot`・begun session id 一覧・`SurfaceSlotId`/`SlotLease` Option を保持し、どの exit path からも `teardown_editor_case` が全リソースを回収できるようにする。
- `editor_ops`/`rejecting_editor_ops`: `InProcessEditorOps::with_redeemer` の 2 種の redeemer（validate-lease / 常時拒否）を関数化。`attempt_editor_attach`・新 case が共有。
- `CountingEditorOps`: `EditorSessionOps` decorator で `open`/`close` dispatch 回数を数える。refused open 後の `close` が adapter（`CloseGui`）を呼ばないことを直接観測する証拠になる。
- `teardown_editor_case`: `notify_terminated(HostLost)` → begun id 全て `end_session` → lease/slot release → `Destroy` を best-effort で実行し notes を `observed.cleanup` に記録。assertion 失敗時も必ず通る単一 teardown 点。
- 各 case の step 関数は `Result<(), String>` を返し、`Err` = assertion 不成立 → `Fail` verdict。期待される拒否（`SessionBusy`・`Ops(Unsupported)`）は step として `ok` 記録し、想定外の成功・別 error は `Fail`。

## Data and Control Flow

- `session-lifecycle`: `begin_session` → `begin_session`（`SessionBusy` 期待）→ `declare_slot`/`lease_parent` → `open(Docked)`（`Ops(Unsupported)` 期待 → `Failed{Unsupported}`）→ event drain `[Failed{Unsupported}]` → `close`（`Closed`+`concluded`・adapter close 0 回）→ drain `[Closed]` → `end_session` → `session_for_instance` = None → 再 `begin_session`（同 slot・generation+1）→ `end_session` → `Destroy`。
- `host-terminate`: `begin_session` → slot lease → refused `open` → `Failed{Unsupported}`（live・non-`Closed`）→ `Destroy` → `notify_terminated(HostLost)` → drain に `Terminated{HostLost}` ちょうど 1 回 → 再 notify no-op（追加 event 0）→ `Closed` → `end_session` → binding None。
- `open-or-degrade`: `begin_session` → slot lease → `open_or_degrade(Docked→Floating)` → 両 mode 拒否で `Err(Ops(Unsupported))`・`opens==2` → `Failed{Unsupported}`・`Degraded`/`Attached`/`Recovered`/`Reattached`/`Shown` event 0 件 → `close` 収束（adapter close 0 回）→ `end_session` → `Destroy`。
- born-`Closed` session には `Terminated` event が出ないため、host-terminate は refused open で `Failed` へ落ちた live session を使う — これが今日の honest 経路。

## Non-Functional Constraints

- 全 host/session 操作は UI-thread・non-RT。`unsafe` 追加なし。新規公開 enum なし（`#[non_exhaustive]` 対象外）。新規依存なし。
- `environment_kind`=`real-abi-test-plugin` は実バイナリ実走 case のみ。artifact 不在は `DeterministicBackend`+`NotRun`。
- verdict 正直性: 推測 Pass 禁止・setup 失敗は `Fail`・期待拒否と想定外を区別して `observed` に記録。

## Risks and Trade-offs

- `test_gain` が将来 editor surface を持った場合、open が `Unsupported` 以外を返し case が `Fail` になる — これは fixture 変更の検知として正直な挙動（case は refused-open 前提を明示）。
- `CountingEditorOps` は `InProcessEditorOps` を包むだけで session ロジックに介入しない — 観測のみで verdict への影響なし。
- teardown の `Destroy` 二重呼出（asserted step 後）は `InvalidHandle` を無視 — 既存 `host-lifecycle` の `let _ =` cleanup 慣行と同等。

## Deferred Design Concerns

- 実 attach 成功経路（`OpenOutcome::Attached`/`Degraded` の実観測）・`SessionDeadlinePolicy` の wall-clock 監視 driver 接続 — 後続 slice | reason: `test_editor` fixture 未整備 | re-engagement trigger: `scarecrow173/ruviform#630` 残存 scope 完了 | risk if ignored: degrade 実経路が未検証のまま残る

## Verification Notes

- [x] `mise run check|lint|test ruviform-plugin-dock-test`（fmt/clippy `-D warnings` 含む）
- [x] `mise run test-ui-plugin-dock` 実走 — 全 8 case verdict を `evidence/rf389-dock/` に記録
- [x] `mise run test ruviform-plugin-host --test editor_session --test editor_session_driver` 非回帰
- [x] `mise run check-ui-driver-boundary`（GPUI 型漏れなし）
