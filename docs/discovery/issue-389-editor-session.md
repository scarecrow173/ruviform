---
id: "DISC-034YtTASQQBBWA8Mb9PpLe"
type: "discovery"
status: "resolved"
title: "Issue #389 editor-session real-session-terminate 調査記録"
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
        path: "docs/discovery/issue-389-editor-session.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-389-editor-session.md"
    - "docs/designs/issue-389-editor-session.md"
    - "docs/plans/issue-389-editor-session.md"
    - "docs/tasks/issue-389-editor-session.md"
    - "docs/test-specs/issue-389-editor-session.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/389"
  derived-by:
    - "docs/specs/issue-389-editor-session.md"
  verifies: []
  verified-by: []
  references:
    - "evidence/rf389/"
  defers: []
  deferred-by: []
---

# Issue #389 editor-session real-session-terminate 調査記録

## Summary

Issue `scarecrow173/ruviform#389` の real-window session lifecycle
L2 検証について、`rf631.plugin-dock-real` lane に
`real-session-terminate` case を 1 行追加する slice の調査記録。
結論: `attempt_real_attach` の `observe_open` seam で
`HostCommand::Deactivate`+`Destroy` →
`PluginEditorSessions::notify_terminated(HostLost)` →
`Terminated{HostLost}` exactly-once → child HWND `IsWindow==0` →
host 存続（`Scan` 成功 + 破棄 handle `InvalidHandle`）を観測する
実装で成立する。確度: 高（実装で確認済みの API 契約に基づく）。

## Exploration Goal

real-window lane で「attach 中の host 破棄 → session `Terminated`
exactly-once → child HWND teardown → host 存続」をどう観測・判定
するかを確定する。headless lane の `host-terminate` case が
fake-slot 経路で同契約を検証済みだが、real-window 側は未実装。
`HostCommand::Destroy` だけでは session registry の終了通知が
走らないため、registry-level の `notify_terminated` を明示的に
呼ぶ必要があるかを確認した。

## Key Issues and Assumptions

- `HostCommand::Destroy` は instance を drop するが
  `PluginEditorSessions` への終了通知は行わない — session は
  `Attached` のまま残る。`notify_terminated(handle, HostLost)` を
  明示的に呼ぶ必要がある（confirmed:
  `editor_session.rs` `notify_terminated` は `by_instance` binding
  を除去してから session を `Closed`+`concluded` に収束）。
- instance は `Activated` のため `Destroy` 直打ちは lifecycle 上
  不正 — `Deactivate` を先行させる（confirmed: headless
  `host-terminate` case が同順で `Deactivate`→`Destroy`→
  `notify_terminated` を実行）。
- adapter の instance drop が `close_view` → `removed()` →
  fixture `DestroyWindow` を同期的に実行するため、`Destroy`
  直後に `IsWindow(child)==0` が観測できる（confirmed:
  `ruviform-plugin-vst3/src/host.rs` `close_view`、
  `ruviform-plugin-test-vst3/src/editor_view.rs`）。
- `attempt_real_attach` 末尾の teardown（`end_session`→slot/lease
  release→`Destroy`）は observe 内 Destroy 後も成立する —
  2 回目 `Destroy` は `InvalidHandle` で `let _ =` に吸収、
  `end_session` は `Closed` session に `Ok`（confirmed:
  `instance_mut` が stale handle を `InvalidHandle` で拒否）。
- event 列の期待値は `[Attached{Docked}, Terminated{HostLost}]` —
  `Closed` event は graceful close 専用で `notify_terminated`
  では emit されない（confirmed: `PluginEditorSession::
  notify_terminated` は `Terminated` のみ push）。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| `observe_open` seam で Destroy+notify を実行 | 既存 attach 機構を再利用・verdict 写像を共有可能 | observe 内で host を破棄するため teardown の 2 回目 Destroy が発生 | yes |
| 専用の attach 経路を新設 | teardown を完全に制御できる | attach 機構の重複・保守コスト増 | no |
| host 存続 oracle に第 2 instance activate | 実 instance での生存証明 | `plugin_id` が observe seam に渡らず追加状態が必要 | no |
| host 存続 oracle に `Scan`+`InvalidHandle` | command surface 生存と instance 死亡を同時観測・追加状態なし | `Scan` は plugin 非依存のため「host が生きている」証明としては間接的 | yes |

## Tentative Conclusions and Hypotheses

- confirmed: `observe_open` seam で `observe_attached_children`
  →`Deactivate`+`Destroy`→`notify_terminated(HostLost)`→
  `Closed`+`concluded`→events 列断言→再 notify no-op→
  `check_detached_children`→`Scan`+`InvalidHandle` の順で全 step
  を観測する。
- confirmed: `editor_attach_body` の verdict 写像を
  `editor_child_body`（observe + pass detail/evidence）へ集約し、
  attach/terminate 両 case が同一写像を共有する。
- confirmed: catalog 末尾（`real-editor-redock` の次）に
  `real-session-terminate` を追加し、report 順も一致させる。
- hypothesis: `Scan` 成功 + 破棄 handle `InvalidHandle` は
  host 存続の十分な oracle — 実装で `HostCommand::Scan` が
  `Scanned` を返し、破棄 handle への command が
  `HostError::InvalidHandle` で拒否されることを確認する。

## Open Questions

- なし（Coordinator 分析で設計確定済み。残る確認は実装時の
  実走で行う）。

## Promotion Candidates

- [x] spec-doc needed: `docs/specs/issue-389-editor-session.md`
- [x] adr-doc needed: なし（既存 session API・adapter 境界の
  範囲内で完結 — 新規 ADR 不要）

## Sources and Relations

- Issue `scarecrow173/ruviform#389`（tracker — auto-close しない）。
- `docs/specs/issue-631-real-row.md`（catalog 構造先例）。
- `crates/ruviform-plugin-host/src/editor_session.rs` —
  `PluginEditorSessions::notify_terminated`（`by_instance` binding
  除去 → exactly-once の根拠）。
- `crates/ruviform-plugin-host/src/command.rs` —
  `HostCommand::Destroy{handle}`。
- `crates/ruviform-plugin-host/src/host.rs` —
  `InProcessHost::destroy` → instance drop。
- `crates/ruviform-plugin-vst3/src/host.rs` —
  `close_view` → `removed()` → `DestroyWindow`。
- `crates/ruviform-plugin-dock-test/src/real_dock.rs` —
  `attempt_real_attach`・`observe_attached_children`・
  `check_detached_children`・`check_session_converged`・
  `editor_attach_body`。
- `crates/ruviform-plugin-dock-test/src/scenario.rs` — headless
  `host-terminate`/`session-lifecycle` case（同契約の先例）。

## Evidence

### 2026-11-16 — session API・teardown 連鎖の確認

- `PluginEditorSessions::notify_terminated`（editor_session.rs
  L1511）は `by_instance` binding を除去（L1516）してから
  session の `notify_terminated` を呼ぶ — 2 回目以降は lookup
  miss で no-op。exactly-once の根拠。
- `PluginEditorSession::notify_terminated`（L981）は
  non-`Closed` → `concluded=true`・`editor_live=false`・
  `state=Closed`・`Terminated{cause}` を 1 回だけ push。
  `Closed` session では早期 return で event を push しない。
- `EditorSessionEvent::Closed` は graceful `session.close()`
  専用 — `notify_terminated` では emit されない。
- `HostCommand::Destroy{handle}`（command.rs L39）は terminal
  lifecycle command。`InProcessHost::destroy`（host.rs）は
  lifecycle 遷移後に `instances[index] = None` で instance を
  drop → `Vst3Plugin::drop` → `close_view`（`removed()` →
  fixture `DestroyWindow`）→ `setActive(false)` → `terminate`。
- instance は `Activated` のため `Destroy` 直打ちは lifecycle
  上不正 — `Deactivate` 先行が必要（headless `host-terminate`
  と同順）。
- `attempt_real_attach` 末尾の teardown（`end_session`→slot/lease
  release→`HostCommand::Destroy`）は observe 内 Destroy 後も
  成立 — `instance_mut` が stale handle を `InvalidHandle` で
  拒否し `let _ =` で吸収、`end_session` は `Closed` session に
  `Ok` を返す。
- `observe_open` seam は `(container, lease_raw, sessions,
  session_id, host, handle, steps)` を受け、post-open 観測を
  担う — `real-editor-attach`/`real-editor-resize`/
  `real-editor-redock` が既にこの seam を使用。
