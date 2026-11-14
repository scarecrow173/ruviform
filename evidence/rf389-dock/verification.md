# rf389-dock session-lifecycle — verification record

Issue `scarecrow173/ruviform#389` dock-test session-lifecycle slice
（`ruviform-plugin-dock-test` への実バイナリ lifecycle case 追加 +
issue-389 設計書 verification-status reconcile）のローカル検証記録。
canonical = `mise` lane。実バイナリ経路の補助証拠として
`plugin-dock-scenario -- --out evidence/rf389-dock/` の直接実行も記録
（canonical lane `test-ui-plugin-dock` は固定 `--out
evidence/rf631/` のため、本 dir の JSONL は同一 binary の再実行出力）。

- Date: 2026-11-15
- OS: Windows (x86_64)
- Toolchain: rustc 1.99.1 (58cf8f9c3 2026-10-13) / cargo 1.99.1
- Repo SHA: `d5aa1b4bfc586de47e542ed1eb931b4f07b2e767`（lane 実走時の
  worktree HEAD — 本 slice の変更は未コミット状態で実走）

## Lane results

| Command | Result |
| --- | --- |
| `mise run fmt-check` | pass (exit 0) |
| `mise run check ruviform-plugin-dock-test` | pass (exit 0) |
| `mise run lint ruviform-plugin-dock-test` | pass (exit 0, `-D warnings`) |
| `mise run test ruviform-plugin-dock-test` | pass (exit 0 — integration 2 tests) |
| `mise run build ruviform-plugin-test-vst3` | pass (exit 0 — test_gain cdylib) |
| `mise run test-ui-plugin-dock` | pass (exit 0) — 全 8 case verdict 下表 |
| `mise run test ruviform-plugin-host --test editor_session --test editor_session_driver` | pass (exit 0 — 49 + 18 tests, session 契約非回帰) |
| `mise run check-ui-driver-boundary` | pass (exit 0 — dep-graph/signature/allowlist) |
| `mise exec -- cargo run --locked -p ruviform-plugin-dock-test --bin plugin-dock-scenario -- --out evidence/rf389-dock/plugin-dock-scenario.jsonl` | pass (exit 0 — 補助証拠: 出力先 capture) |

## `plugin-dock-scenario.jsonl` 実走 verdict（全 8 case）

| case | driver | verdict | environment_kind | 備考 |
| --- | --- | --- | --- | --- |
| `view-mount` | fake | pass | deterministic-backend | semantic `NativeSlot` mount のみ — dock/window/pixel 証拠ではない |
| `artifact` | plugin-host-probe | pass | real-abi-test-plugin | `test_gain` cdylib 解決 + SHA-256 + descriptor |
| `host-lifecycle` | plugin-host-probe | pass | real-abi-test-plugin | Scan→Instantiate→Initialize→Activate→Deactivate→Destroy |
| `editor-attach` | plugin-host-probe | unsupported | real-abi-test-plugin | `createView` null → `EditorOpError::Unsupported`（editor surface 無し） |
| `session-lifecycle` | plugin-host-probe | pass | real-abi-test-plugin | begin→`SessionBusy`→refused `open(Docked)`→`Failed{Unsupported}`→adapter-free `close` 収束→`end_session`→binding 解放→slot/generation+1 再 begin→Deactivate→Destroy（**新規**） |
| `host-terminate` | plugin-host-probe | pass | real-abi-test-plugin | refused open の live `Failed` session → Deactivate→Destroy → `notify_terminated(HostLost)` → `Terminated{HostLost}` exactly-once・再 notify no-op → `Closed`（**新規**） |
| `open-or-degrade` | plugin-host-probe | pass | real-abi-test-plugin | `open_or_degrade(Docked→Floating)` 両 mode 拒否 → `Err(Ops(Unsupported))`・`Failed{Unsupported}`・`Degraded`/attach-family event 0 件（**新規**） |
| `dock-pixels` | plugin-host-probe | not_run | deterministic-backend | docked editor surface 無し + VisualCapture driver 無し — 画素未取得を正直に記録 |

## Artifact evidence（実測値）

- path: `target/debug/deps/ruviform_plugin_test_vst3.dll`（env var 未設定時の自動解決経路）
- plugin: `test_gain` / id `218a4f6d7c3b059ef1a24c68b9d37e51` / vendor `Ruviform Test` / version `0.0.0`
- SHA-256: `706a866957acaac2d41a443180bf00ce50d5b06b4938be0d6c0c022756171e97`
- `environment_kind`: `real-abi-test-plugin`（実 ABI バイナリを実走した case のみが名乗る）

## Session evidence の読み方

- `session-lifecycle` の `observed.steps` に `begin_session` /
  `begin_session_again`(session-busy) / `declare_slot` / `lease_parent` /
  `open_docked`(refused-unsupported, ops_opens=1) /
  `state_after_refused_open`(failed-unsupported) /
  `drain_events_after_open`([Failed{Unsupported}]) /
  `close_converge`(ops_closes=0) / `state_after_close`(closed-concluded) /
  `drain_events_after_close`([Closed]) / `end_session` /
  `session_for_instance`(none) / `rebegin_session` /
  `rebegin_session_check`(slot_reused, generation_bumped) /
  `end_session_second` / `deactivate` / `destroy` が `ok` で記録される。
- `host-terminate` は `drain_events_terminated` に
  `[Terminated{cause:HostLost}]` がちょうど 1 件、
  `drain_events_after_second_notify` が空であることを記録する。
- `open-or-degrade` は `ops_opens=2`（Docked 試行 + Floating retry）と
  `[Failed{Unsupported}]` 1 件のみの event 列を記録する — degrade を
  捏造しないことの証拠。

## NotRun / Unsupported / Deferred（意図的・後続検証が必要）

- 実 Native Dock・実 child HWND・画素・Input・Resize・Focus・ReDock:
  `test_editor` fixture（`scarecrow173/ruviform#630` 残存 scope）到着後に
  同じ case が `Pass`/`Fail` を返すかで判定。本 slice では
  `unsupported`/`not_run` を維持。
- `OpenOutcome::Attached`/`Degraded` の実 attach 成功経路、
  `SessionDeadlinePolicy` の wall-clock 監視 driver 接続: 後続 slice。
- `RealWindow` kind: 実 Window driver lane 未実装のため未使用。
- 3OS・`*-msrv`・`*-full` 系: 本 slice の対象外（task spec 明記）。

## Honest-verdict confirmation

- artifact 未解決時は integration test
  `missing_artifact_fails_closed_as_not_run` が新 3 case を含む
  dependent case の `NotRun` を確認（silent pass 無し）。
- `fixture_run_reports_honest_verdicts` が上表 verdict と
  `real-abi-test-plugin` の environment_kind 主張を assert。
