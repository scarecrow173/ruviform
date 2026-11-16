---
id: "DESIGN-034YtyecE0L2ybG7Hp86sj"
type: "design"
status: "approved"
title: "Issue #389 editor-session real-session-terminate 設計"
created: "2026-11-16"
updated: "2026-11-16"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "docs/specs/issue-389-editor-session.md"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-389-editor-session.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-389-editor-session.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-389-editor-session.md"
    - "docs/specs/issue-631-real-row.md"
  blocks:
    - "docs/plans/issue-389-editor-session.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-389-editor-session.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-389-editor-session.md"
  derived-by:
    - "docs/plans/issue-389-editor-session.md"
    - "docs/test-specs/issue-389-editor-session.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-389-editor-session.md"
  references:
    - "evidence/rf389/"
  defers: []
  deferred-by: []
---

# Issue #389 editor-session real-session-terminate 設計

## Context

`rf631.plugin-dock-real` は rf631 real-row 拡充で 12 行の catalog
（`REAL_CATALOG` metadata + feature-gated `real_entries()` +
`--blocked-probe` fail-closed lane）を持つ。Issue #389 の
real-window session lifecycle L2 検証のうち、host loss 経路
（attach 中の instance 破棄 → `Terminated` exactly-once → child
HWND teardown → host 存続）は未実装である。

headless lane の `host-terminate` case（`scenario.rs`）が同じ
registry 契約を fake-slot 経路で既に検証している:
`Destroy` → `notify_terminated(HostLost)` → `Terminated{HostLost}`
exactly-once → `Closed`。本 slice はこれを real-window lane の
`observe_open` seam に写す — 相違点は (a) session が refused
`Failed{Unsupported}` ではなく実 `Attached{Docked}` であること、
(b) child HWND の OS teardown を `IsWindow==0` で観測すること、
(c) host 存続を command surface で観測することの 3 点。

## Scope

### In Scope

- `real_catalog.rs`: `REAL_CATALOG` 末尾へ `real-session-terminate`
  行追加（12 → 13）。
- `real_dock.rs`: `real_entries()` 13 entry 化、case 本体
  （observe + body + case fn）、共有 verdict 写像への集約、
  blocked/test の行数追従、doc 記述追従。
- `tests/console.rs`: 21 行・期待 case 一覧の追従。
- crate doc（`lib.rs`/bin/`README.md`）の行数・verdict 表追従。
- doc chain と `evidence/rf389/` 記録。

### Out of Scope

- session move/degraded/crash/hang/recovery、tick/deadline、RT
  非干渉、adapter verb 実装、`real-dcomp-pixel` 画素、consumer
  gates、Issue close、`mise.toml`/Cargo 変更、docs index。

## Components and Boundaries

- `real_catalog.rs`（GPUI-free metadata）と `real_dock.rs`
  （feature `real-dock`、executable entries）は case/scenario/
  required が pairwise 一致する既存契約を維持する — 片方だけの
  追加は `real_catalog_matches_entries` が検出する。
- `attempt_real_attach` は不変: slot declare→bind→project→
  `lease_parent`→`container_token`→`begin_session`→
  `session.open(Docked)`→`observe_open`→teardown
  （`end_session`→slot/lease release→`Destroy`）。本 case は
  `observe_open` 内で host instance を破棄するため、teardown の
  2 回目 `Destroy` は `InvalidHandle` で `let _ =` に吸収される
  （実装で確認済み — `instance_mut` が stale handle を
  `InvalidHandle` で拒否）。
- `PluginEditorSessions::notify_terminated(handle, cause)` は
  `by_instance` binding を除去してから session を `Closed`+
  `concluded` に収束させる — 2 回目以降の notify は lookup miss
  で no-op。exactly-once の根拠はこの binding 除去であり、
  `PluginEditorSession::notify_terminated` 自体も `Closed` session
  では event を push しない。
- host teardown 連鎖: `HostCommand::Destroy` →
  `InProcessHost::destroy` → `instances[index] = None` →
  `Vst3Plugin::drop` → `close_view`（`removed()` → fixture の
  `DestroyWindow`）→ `setActive(false)` → `terminate`。
  instance が `Activated` のため `Destroy` 直打ちは lifecycle
  上不正 — `Deactivate` を先行させる（headless `host-terminate`
  と同順）。
- 破棄済み handle への後続 command は `instance_mut` が
  `HostError::InvalidHandle` で拒否 — host 生存と instance 死亡の
  両方を 1 つの観測で証明できる。

## Data and Control Flow

`real-session-terminate` の `observe_open` 内 step 列:

1. `child_hwnd_observation`/`child_enumerated`/`child_contract`/
   `thread_affinity`（`observe_attached_children` — attach と同一）。
2. `deactivate` / `destroy`（`host.execute` — 各 `pass`/`fail` row）。
3. `notify_terminated`（`sessions.notify_terminated(handle,
   HostLost)` — infallible、`pass` row + `cause: "host-lost"`）。
4. `session_concluded`（`Closed`+`concluded`）。
5. `session_events`（drain = ちょうど `[Attached{Docked},
   Terminated{HostLost}]`）。
6. `notify_terminated_idempotent`（2 回目 notify 後の drain が空）。
7. `child_hwnd_detached`（`check_detached_children` —
   `IsWindow(child)==0`）。
8. `host_survives`（`HostCommand::Scan` → `Scanned`）。
9. `destroyed_handle_rejected`（破棄 handle への `Initialize` が
   `InvalidHandle`）。

event 列の期待値は実イベント順で `[Attached{Docked},
Terminated{HostLost}]` — `Terminated` が唯一の終端 event で
`Closed` event は graceful close 専用のため列に含まれない
（`notify_terminated` の実装で確認）。

## Non-Functional Constraints

- 非 RT・test-only。新規 `unsafe` なし（`oschild` の既存
  query-only Win32 を再利用）。
- GPUI/Win32 型を公開 API・evidence schema に漏らさない
  （`check-ui-driver-boundary` green — HWND は整数記録）。
- `EnvironmentKind::RealWindow` は実 session 内のみ。
- verdict の正直性: 実走 step のみ `Pass`、defined refusal は
  `Unsupported`、未定義失敗は `Fail`、環境阻害は
  `NotRun`+`environment_blocked`。

## Risks and Trade-offs

- **`editor_attach_body` の verdict 写像を共有 helper 化**:
  `real-session-terminate` も `ChildVerification`
  （supported/verified/failure/extras）を返すため、
  `editor_attach_body` を `observe_open` + pass detail/evidence を
  取る `editor_child_body` へ一般化し、attach/terminate 両 case
  が同一の verdict 写像を共有する。`editor_op_body`
  （`EditorOpVerification`/`OpLanding` 系）は別形のため対象外。
- **Deactivate 失敗時も Destroy を試行**: lifecycle が `Failed`
  等へ落ちている場合 `Destroy` は合法になり得るため、両 command
  を独立に実行して各々の結果を step row に記録する（失敗は
  `host_loss_ok=false` で `Fail` へ収束）。
- **host 存続 oracle**: `Scan` 成功 + 破棄 handle の
  `InvalidHandle` 拒否で「command surface 生存」と「instance
  死亡」を同時に観測する。第 2 instance activate は
  `plugin_id` が observe seam に渡らないため採用しない
  （`Scan` で十分かつ追加状態を持ち込まない）。
- **`real-dcomp-pixel` の既知 `Fail`**: lane exit 1 は設計どおりの
  正直な regression 信号であり、本 slice で緩和しない。

## Deferred Design Concerns

- session move/degraded/crash/hang/recovery・tick/deadline・
  実 Window RT 非干渉 — verb/観測面の整備が前提 | 後続 slice |
  verb 未到着のまま Pass を主張するリスク。
- `real-dcomp-pixel` 画素 — `scarecrow173/ruviform#532` 決裁待ち。
- capability matrix reconcile — `issue-389-plugin-editor-session`
  design は本 slice のリース外、Coordinator 判断。

## Verification Notes

- [x] `real_catalog_matches_entries`/`blocked_reports_fail_closed`
  が 13 行で green（feature `real-dock` test）。
- [x] `plugin-dock-console --list --include-real` が 21 行を
  projection（`tests/console.rs`）。
- [x] `--blocked-probe` が 13 行 `NotRun`+`environment_blocked`・
  violation 0・exit 0（`test-ui-plugin-dock-real-headless`）。
- [x] interactive 実走で新 case の実 verdict を
  `evidence/rf389/` に記録（実走不可なら `NotRun`+
  `environment_blocked` をそのまま証拠化）。
- [x] `check`/`lint`/`test`（default・`real-dock`・`lab-gui`）、
  `build ruviform-plugin-test-vst3`、plugin-host session test
  非回帰、`test-ui-plugin-dock`/`console`/`real-headless`/
  `check-ui-driver-boundary`/`prek` green。
