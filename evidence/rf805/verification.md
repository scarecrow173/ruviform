# rf805 — verify-ruviform skill・rvf-ctl control surface verification record

Issue `scarecrow173/ruviform#805` の検証記録。slice 2
（`features/` フィーチャーマップ + `rvf-ctl drive` 系サブコマンド +
interactive live proof）を正本とし、slice 1
（skill 骨組み + `rvf-ctl doctor` + headless live proof）は履歴として
末尾に要約で残す。canonical = `mise` lane。補助証拠（直接バイナリ
呼び出し）は明記済み。

- Date: 2026-11-20
- OS: Windows (x86_64) — interactive desktop session 有り（
  `UserInteractive=True`、slice 2 は real-window lane を実走）
- Toolchain: rustc 1.99.1 (58cf8f9c3 2026-10-13) / cargo 1.99.1
- Repo SHA: `e11854c28335c359a46065b08683d83cd1e67866`（slice 1 merge
  起点 + 本 slice の未コミット変更 — 各報告の `repo.sha` は実行時
  HEAD を記録）
- ローカル検証 = 変更範囲の検証。統合検証は merge 後の対象 SHA に
  対する必須 CI 結果が証拠（本記録はそれを代替しない）。

## Actions → resulting state（dry-run → drive → mutation/observation → 2nd read-only → cleanup）

| # | Action (command) | Resulting state |
| --- | --- | --- |
| 1 | `cargo build -p ruviform-ui-test --bin dummy_app --bin rvf-ctl` | 両バイナリ生成・exit 0 |
| 2 | `rvf-ctl drive --dry-run 'observe.windows' 'fixture.prepare={"name":"windows","params":{"count":1}}' --out evidence/rf805/rvf-ctl-drive-dry-run.json` | `type:"drive-plan"`・validated step 2 件・`app_exists:true`・AUT 未 spawn・exit 0（補助証拠） |
| 3 | `mise run test-ui-rvf-ctl-drive` → `rvf-ctl drive <13 steps> --out evidence/rf805/rvf-ctl-drive.json` | spawn → hello → 13 step 順次実行 → graceful shutdown → child exit 0。`verdict:"pass"`・exit 0（canonical 証拠: `rvf-ctl-drive.json`） |
| 4 | 同 drive 内 mutation 観測（step 3 fixture + step 11 request_close + 前後 `observe.windows`） | `windows` fixture が id 2 のウィンドウを生成（`applied.windows:[2]`→ 4 番目の `observe.windows` で 2 窓）→ `close_policy:"veto"` で `request_close` 後も生存（12 番目 `observe.windows` で 2 窓のまま、`close_pending:false`）— action → resulting state が report 内で完結 |
| 5 | `rvf-ctl drive 'observe.windows' 'observe.receipts={"since":0,"limit":16}' 'notices' --out evidence/rf805/rvf-ctl-drive-observe.json` | 2 回目 read-only 観測 — read-only step のみで `verdict:"pass"`・exit 0・副作用ゼロ（`cleanup.windows_open:1→closed:1`） |
| 6 | misuse / failure 経路（直接実行・exit コード実測） | `bogus.op` → exit 2。`clock.advance domain:"gpu"` → exit 2（known: host_tick）。`fault.set kind:"ipc_down"` → exit 2（known: window_crash, ipc_disconnect）。step 無し → usage + exit 2。いずれも spawn 前に拒否・AUT 未起動 |
| 7 | `rvf-ctl drive --app target/debug/nonexistent_app.exe 'observe.windows'` | `verdict:"not_run"` + `error.environment_blocked:true` + `stage:"spawn"` + `next_step`（build 提示）・exit 1 |
| 8 | `rvf-ctl drive --app-args "--fail-init" 'observe.windows'` | handshake 失敗を `fail` + `stage:"handshake"` で記録・exit 1（dummy_app の failure-path 証拠） |
| 9 | `rvf-ctl drive --app-args "--windows 0" 'observe.screenshot={"window":"last"}'` | `"last"` が解決不能 → 当該 step `not_run` + detail（「observe.windows または窓生成 fixture が先」）・report `fail`・exit 1 |
| 10 | cleanup 観測（`tasklist` で dummy_app / windowed_test_app / plugin-dock-real-scenario を確認） | 全プロセス終了・残存ゼロ。`cleanup{windows_open:2→windows_closed:2, exited:true}` + `child_exit.success` と一致。`evidence/rf805/` の報告ファイルは teardown 後も残存 |

## Drive report（`rvf-ctl-drive.json` 抜粋 — headless `dummy_app`）

- `type:"drive"`, `lane:"rvf-ctl-drive"`, `verdict:"pass"`,
  `environment_kind:"deterministic-backend"`
- steps（argv 順 = wire 順、全 13）:
  `capabilities`→pass、`observe.windows`→pass、
  `fixture.prepare(windows,count:1,close_policy:veto)`→pass、
  `observe.windows`→pass、`clock.advance(host_tick,2)`→pass、
  `observe.receipts`→pass、`fault.set(ipc_disconnect,w1)`→pass、
  `observe.windows`→pass（`ipc_down:true` を観測）、
  `fault.clear`→pass、`notices`→pass、
  `window.request_close("last"=2)`→pass（veto で生存を観測）、
  `observe.windows`→pass、
  `observe.screenshot("last")`→**unsupported**
  （`unavailable: capability "screenshot" is not declared` —
  headless-fake の未宣言 capability を fail にせず正直記録）
- checks: `spawn`/`handshake`/`shutdown`/`child-exit` — 全 `pass`
- `cleanup`: `windows_open:2 → windows_closed:2, exited:true`;
  `child_exit.code:0`

## Interactive live proof（real-window lane — interactive desktop session で実走）

| Action | Resulting state |
| --- | --- |
| `mise run test-ui-windowed-uitest` | exit 0。`evidence/issue-313-windowed-uitest/verdict-windowed-uitest.json`: `verdict:"pass"`, `desktop_session:true`, backend `gpui-windowed`, capture_backend 有り。hello→observe→fixture→clock→ipc_emit→fault→screenshot→request_close→shutdown 全 pass + `windowed_fail_init` 経路も pass |
| `rvf-ctl doctor --app target/debug/windowed_test_app.exe --out evidence/rf805/rvf-ctl-doctor-windowed.json` | `verdict:"pass"`・backend `gpui-windowed`・`environment_kind:"real-window"`・5 check 全 pass・exit 0（補助証拠 — AUT 差し替えが同じ制御面で通ることの実証） |
| `rvf-ctl drive --app target/debug/windowed_test_app.exe --app-args "--windows 1" <10 steps> --out evidence/rf805/rvf-ctl-drive-windowed.json` | `verdict:"pass"`・exit 0。`observe.screenshot("last")` が **pass** — `format:"rgba-raw"` 320x240・`decoded_bytes:307200`（=320×240×4、実ピクセル取得）。`window.request_close` で実窓を閉じ `observe.windows` が縮小を観測。cleanup 1→1 + child exit 0 |
| `mise run test-ui-plugin-dock-real` | exit 1 — 設計どおりの正直シグナル。`evidence/rf631/plugin-dock-real-scenario.jsonl`: 23 case（`pass` 19・`fail` 1・`unsupported` 3）。`fail` は `real-dcomp-pixel`（`scarecrow173/ruviform#532` の occlusion 証拠、EDITOR_COLOR 未検出を報告 — 本 slice は改変しない）。`unsupported` は `real-window-ime`（Ime 未対応）・`real-session-crash`/`real-session-hang`（in-process lane は kill/hang fault を生存できない — 正直記録）。`real-editor-attach` は test_editor 実 child HWND の attach/detach で pass |

## Lane results（canonical = mise、本 slice 実走分）

| Command | Result |
| --- | --- |
| `mise run fmt-check` | pass (exit 0) |
| `mise run check ruviform-ui-test` | pass (exit 0) |
| `mise run lint ruviform-ui-test` | pass (exit 0, `-D warnings`) |
| `mise run test ruviform-ui-test` | pass (exit 0 — unit + integration suites) |
| `mise run test-ui-driver-contract` | pass (exit 0 — `manifests_are_consistent` 含む) |
| `mise run test-ui-plugin-dock-console` | pass (exit 0 — `--list` + `--run` 非回帰; JSONL は pass 25・`unsupported`(editor-attach) 1・`not_run`(dock-pixels) 1 で変化なし) |
| `mise run check-ui-driver-boundary` | pass (exit 0 — dep-graph/signature/allowlist) |
| `mise run test-ui-rvf-ctl` | pass (exit 0 — doctor 報告再生成) |
| `mise run test-ui-rvf-ctl-drive` | pass (exit 0 — **新規 lane**、上表 #3-4) |
| `mise run test-ui-windowed-uitest` | pass (exit 0 — interactive、上表) |
| `mise run test-ui-plugin-dock-real` | exit 1 — `real-dcomp-pixel` 正直 `fail`（#532、上表。`unsupported`/`not_run` への改変なし） |
| `mise exec -- prek run --all-files` | pass（初回 `end-of-file-fixer` が生成 JSON の末尾改行を自動修正 → 再実行で全 hook 緑） |

## 受入条件との対応（Issue #805）

| AC | 状態 | 証拠 |
| --- | --- | --- |
| `features/` feature map 整備 | pass | `.agents/skills/verify-ruviform/features/` — README index + 5 feature（headless-aut-drive / plugin-dock-console / windowed-uitest / plugin-dock-real / driver-lab）、各 4-H2 形式（Sub-features / How to get to it / Driving it with / Gotchas） |
| `rvf-ctl` drive 系サブコマンド | pass | `rvf-ctl drive` — closed op set を 1 session で順次実行、`"last"` selector、事前 validation（misuse exit 2）、per-step verdict、dry-run、`--app`/`--app-args` |
| SKILL.md Drive/Helpers 更新 | pass | Drive 節が実コマンド化、Helpers に drive + lane + features/ 参照、旧「doctor のみ」記述を撤去 |
| headless live proof | pass | 上表 #1-10 + `rvf-ctl-drive.json`（dry-run・mutation/observation・unsupported・failure path・2nd read-only・cleanup すべて実走） |
| interactive live proof | pass | `test-ui-windowed-uitest` + `test-ui-plugin-dock-real` + windowed AUT への `rvf-ctl doctor`/`drive`（実 `rgba-raw` ピクセル取得） — `environment_blocked` ではなく実走 |

## NotRun / Unsupported（正直記録・意図的）

- `real-dcomp-pixel`: `fail`（#532 既知） — 本 slice は verdict を変えない。
- `editor-attach`（headless console）: `unsupported` — test_gain に editor
  surface 無し（#630 remainder）。
- `real-window-ime`・`real-session-crash`/`real-session-hang`:
  `unsupported` — capability/ in-process 限界の正直記録。
- `dock-pixels`（headless）: `not_run` — docked editor surface + capture
  無し。
- `test-ui-plugin-dock-lab`・`test-ui-real-os-scenario` 等のその他
  interactive lane: 本 slice の必須範囲外（NotRun）。
- 3OS・`*-msrv`・`*-full` 系: 本 slice 対象外。
- docs index（`docs/**/README.md`）: 新規追加ファイルがあるため
  Coordinator 統合時に整備（本 worker では触らない）。

## Slice 1 record（PR #808 で merged — 要約）

- `rvf-ctl doctor` lane 追加（`test-ui-rvf-ctl`）+ headless proof:
  spawn → hello+capabilities → graceful shutdown → child exit 0 を
  `verdict:"pass"` で実走。`--dry-run` plan・`--transport tcp`
  loopback ・2nd read-only rerun・missing-AUT `not_run` +
  `environment_blocked`（exit 1）・非 uitest handshake `fail`・
  未知 subcommand exit 2 を実証。
- `plugin-dock-console` `--list`/`--run` を `evidence/rf805/` に保存
  （9 case: pass 7・`unsupported` 1・`not_run` 1）。
- interactive windowed lane は当時未実走（slice 2 で実走済み — 上表）。
- 詳細は git history（`dbe35978` 時点の本ファイル）を参照。
