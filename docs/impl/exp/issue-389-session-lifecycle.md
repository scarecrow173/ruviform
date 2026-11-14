# Issue #389 dock-test session-lifecycle — 実装実験記録（2026-11-15）

`docs/tasks/issue-389-session-lifecycle.md` の実装で試行・観測した
事項の記録。実装結果の正本は
[`docs/impl/ir/issue-389-session-lifecycle.md`](../ir/issue-389-session-lifecycle.md)、
イベントログは
[`issue-389-session-lifecycle-exp.jsonl`](issue-389-session-lifecycle-exp.jsonl)
を参照。

## 試行 1: session case の `too_many_lines`（clippy）

- **目的**: `session-lifecycle`/`host-terminate`/`open-or-degrade`
  の step 関数に begin → fake slot declare/lease → redeem → refused
  open → state/event 断言 → close/end → teardown を直列に書く。
- **観測**: 初回 lint で `too_many_lines` が発火（`host_terminate_steps`
  が特に長い）。setup 失敗ごとの部分 teardown が match arm に散在
  していた。
- **結論**: `SessionCaseState`（host/sessions/lease/slot を束ねる
  teardown 単位）・`begin_case_session`・`declare_fake_dock_lease`・
  `drain_case_events`・`teardown_editor_case`・`editor_size`・
  `editor_ops`/`rejecting_editor_ops`・`CountingEditorOps` に分割。
  refused-docked-open の共通逐次（lease → `open(Docked)` →
  `Failed{Unsupported}` 断言）も helper 化し、3 case が同じ観測
  経路を共有する構成にした。assertion 失敗時も必ず teardown を通る
  best-effort 単一経路に集約。挙動は変えず lint green。

## 試行 2: `HostCommand::Destroy` の illegal lifecycle transition

- **観測**: 初回 `cargo test` で `session-lifecycle` が
  `destroy: lifecycle rejected command: illegal lifecycle transition:
  Destroy is not legal from Activated` で Fail。
- **原因**: 新 case が `activate_test_gain` 後に `Deactivate` を
  挟まず `Destroy` を直接発行していた。host lifecycle case は
  `Deactivate` → `Destroy` の順を踏んでいた。
- **結論**: 全 session case の teardown で `Deactivate` → `Destroy`
  の順序を明示。`host-terminate` case は「host が先に死ぬ」状況を
  模すため instance を明示 destroy してから `notify_terminated`
  を呼ぶ構成は維持（session 側の収束は host command の成否に
  依存しない）。

## 試行 3: `Terminated` event を得る live session の条件

- **観測**: born-`Closed`（open 未実施）の session に
  `notify_terminated` しても `Terminated` event は発火しない
  （`Closed` からの遷移のみ event を発する規範）。
- **結論**: `host-terminate` は refused open で `Failed{Unsupported}`
  に落ちた live session を使う honest 経路を採用 — test_gain は
  editor surface を持たないためこれが今日到達可能な非終端 session。
  `Terminated{HostLost}` がちょうど 1 回・再 notify で 0 件・
  `Closed`+`concluded` 収束を実 binary で観測した。

## 試行 4: `open_or_degrade` の double-refusal honesty

- **観測**: test_gain は Docked/Floating 両 mode の open を拒否。
  `open_or_degrade(Docked → Floating)` は fallback も拒否され
  `Err(EditorSessionError::Ops(Unsupported))` を返し、`Failed
  {Unsupported}` へ収束。`Degraded`/`Attached`/`Recovered`/
  `Reattached`/`Shown` event は 0 件。
- **結論**: 「拒否を degrade 成功と偽らない」契約を実 binary で
  観測可能にした。`OpenOutcome::Degraded` の成功経路は editor
  surface を持つ fixture が必要で本 slice 対象外 — fixture 拡張時に
  同 case が正直に `Degraded` を記録する構成（verdict は fake
  success を主張しない）。

## 試行 5: verdict の実測確認

- `mise run test-ui-plugin-dock` 実走で 8 case 全 verdict を確認:
  `view-mount=pass`(fake)・`artifact=pass`・`host-lifecycle=pass`・
  `editor-attach=unsupported`・`session-lifecycle=pass`・
  `host-terminate=pass`・`open-or-degrade=pass`（いずれも
  `real-abi-test-plugin`）・`dock-pixels=not_run`。
- artifact は `target/debug/deps/ruviform_plugin_test_vst3.dll`
  （sha256 `706a8669…`）。session 3 case の `Pass` は session
  registry 契約の実走証拠であり、実 editor attach・Native Dock・
  画素の証拠ではない（test_gain `createView` null）。
