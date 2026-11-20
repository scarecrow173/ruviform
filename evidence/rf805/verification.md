# rf805 slice 1 — verify-ruviform skill + rvf-ctl doctor live proof

Issue `scarecrow173/ruviform#805` の最初の実装 slice（skill 骨組み +
制御 CLI 最小面 `doctor` + headless live proof）のローカル検証記録。
canonical = `mise` lane。補助証拠（直接バイナリ呼び出し）は明記済み。

- Date: 2026-11-20
- OS: Windows (x86_64)
- Toolchain: rustc 1.99.1 (58cf8f9c3 2026-10-13) / cargo 1.99.1
- Repo SHA: `1a696064bbbc9c2310a27e35492a8dd5f9ab49d0`（`origin/main` 起点
  + 本 slice の変更 — 報告内の `repo.sha` は doctor 実行時 HEAD を記録）
- ローカル検証 = 変更範囲の検証。統合検証は merge 後の対象 SHA に対する
  必須 CI 結果が証拠（本記録はそれを代替しない）。

## Actions → resulting state（launch → doctor → drive → evidence → cleanup）

| # | Action (command) | Resulting state |
| --- | --- | --- |
| 1 | `cargo build -p ruviform-ui-test --bin dummy_app --bin rvf-ctl`（`mise run test-ui-rvf-ctl` 内） | `target/debug/dummy_app.exe`・`target/debug/rvf-ctl.exe` 生成（exit 0） |
| 2 | `mise run test-ui-rvf-ctl` → `rvf-ctl doctor --out evidence/rf805/rvf-ctl-doctor.json` | 子プロセス spawn → `test.hello` Welcome → `test.capabilities` → `test.shutdown` graceful → 子 exit 0。`verdict: "pass"`、5 check 全 pass、exit 0（canonical 証拠: `rvf-ctl-doctor.json`） |
| 3 | `rvf-ctl doctor --dry-run --out evidence/rf805/rvf-ctl-doctor-dry-run.json` | `type:"doctor-plan"`・ops `[test.hello, test.capabilities, test.shutdown]`・AUT 未 spawn・exit 0（補助証拠） |
| 4 | `rvf-ctl doctor --transport tcp --out evidence/rf805/rvf-ctl-doctor-tcp.json` | loopback TCP + run-bound secret で同 5 check 全 pass・exit 0（補助証拠 — transport 分岐が dead code でない実証） |
| 5 | `rvf-ctl doctor --out evidence/rf805/rvf-ctl-doctor-rerun.json` | 2 回目 read-only 観測 — verdict 不変 `pass`・exit 0。証拠ファイルは消失せず、副作用を残さないことを確認 |
| 6 | `plugin-dock-console --list > evidence/rf805/plugin-dock-console-list.jsonl` | `type:"catalog"` 9 行の capability projection（実行証拠ではない）・exit 0（補助証拠 — canonical = `mise run test-ui-plugin-dock-console`） |
| 7 | `plugin-dock-console --run --out evidence/rf805/plugin-dock-console.jsonl` | `type:"run"` 1 行 + `type:"case"` 9 行（verdict 下表）・exit 0（補助証拠 — 同上） |
| 8 | cleanup 観測（`tasklist` で dummy_app / rvf-ctl / plugin-dock-console を確認） | 全プロセス終了・残存ゼロ。`test.shutdown` の `CleanupReport{windows_open:1, windows_closed:1, exited:true}` と一致。`evidence/rf805/` の 6 ファイル + 本記録は teardown 後も残存 |

## Doctor report（`rvf-ctl-doctor.json` 抜粋）

- `type: "doctor"`, `lane: "rvf-ctl-doctor"`, `transport: "stdio"`,
  `verdict: "pass"`
- checks: `spawn`/`handshake`/`capabilities`/`shutdown`/`child-exit`
  — 全 `pass`
- `welcome`: `v:1`, backend `headless-fake`, `windows: 1` ready,
  `ruviform_revision`/`session_id`/`run_id`/`os`/`arch` 記録
- `capabilities`（正直値）: `input_injection:false`,
  `dom_access:false`, `screenshot:false`、observation =
  windows/receipts/notices、fixture = true、clock = host_tick、
  faults = window_crash/ipc_disconnect
- `cleanup`: `windows_open:1 → windows_closed:1, exited:true`;
  `child_exit.success: true`
- failure 経路実証（本記録外だが実走済み）: `--app` が存在しない
  path → `verdict:"not_run"` + `error.environment_blocked:true` +
  `stage:"spawn"` + `next_step`（build コマンド提示）・exit 1。
  非 uitest バイナリ → handshake `fail` + next_step・exit 1。
  未知の subcommand/flag → usage 表示・exit 2。

## `test-ui-plugin-dock-console` 実走 verdict（`plugin-dock-console.jsonl`）

| case | driver | verdict | 備考 |
| --- | --- | --- | --- |
| `view-mount` | fake | pass | semantic mount only — dock/window/pixel 証拠ではない（detail 明記） |
| `command-receipt` | fake | pass | Begin→Preview→Commit minted intents → Applied/Rejected receipt settle |
| `artifact` | plugin-host-probe | pass | `test_gain` cdylib 自動解決 + SHA-256 digest 記録 |
| `host-lifecycle` | plugin-host-probe | pass | instantiate→…→destroy 実走 |
| `editor-attach` | plugin-host-probe | unsupported | `createView` null → `HostError::Unsupported`（test_gain は editor surface を持たない — 正直記録を維持） |
| `session-lifecycle` | plugin-host-probe | pass | |
| `host-terminate` | plugin-host-probe | pass | |
| `open-or-degrade` | plugin-host-probe | pass | |
| `dock-pixels` | plugin-host-probe | not_run | docked editor surface 無し + VisualCapture 無し — 画素未取得を正直に記録（`real-dcomp-pixel` 系の fail-closed 意味論は不変） |

## Lane results（canonical = mise、本 slice 実走分）

| Command | Result |
| --- | --- |
| `mise run fmt-check` | pass (exit 0) |
| `mise run check ruviform-ui-test` | pass (exit 0) |
| `mise run lint ruviform-ui-test` | pass (exit 0, `-D warnings`) |
| `mise run test ruviform-ui-test` | pass (exit 0 — 83 unit + integration suites) |
| `mise run test-ui-driver-contract` | pass (exit 0 — 7 tests, `manifests_are_consistent` 含む) |
| `mise run test-ui-plugin-dock-console` | pass (exit 0 — `--list` + `--run` 非回帰) |
| `mise run check-ui-driver-boundary` | pass (exit 0 — dep-graph/signature/allowlist) |
| `mise run test-ui-rvf-ctl` | pass (exit 0 — 新規 lane、上表 #1-2) |

## NotRun / Unsupported / Deferred（意図的・後続）

- 実 Window 系 lane（`test-ui-windowed-uitest`/`test-ui-plugin-dock-real`/
  `test-ui-plugin-dock-lab`/`plugin-dock-lab` GPUI window）: interactive
  desktop session が gate — 本 slice では未実走（NotRun）。SKILL.md には
  実コマンド + `environment_blocked` 条件を記載済み。headless host では
  全 case `not_run` + `environment_blocked`・exit 0 の fail-closed。
- `features/` feature map・rvf-ctl の drive 系サブコマンド・interactive
  live proof: 後続 slice（Issue #805 OPEN 維持、`Refs #805`）。
- 3OS・`*-msrv`・`*-full` 系: 本 slice 対象外。
- `mise exec -- prek run --all-files`: 実施結果は PR 本文に記録。
- docs index（`docs/**/README.md`）: 未同期のまま — Coordinator が統合時に
  整備。
