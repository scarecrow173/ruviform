# RF630 CLAP `test_editor` Fault arm 実装 report — `real-clap-session-{crash,hang}` 有効化

- Issue: https://github.com/scarecrow173/ruviform/issues/630 (AC2 CLAP crash/hang leg + AC4 implementable 残件 — VST3 `scarecrow173/ruviform#885` のミラー)
- Branch: `feat/630-clap-editor-fault`
- Base: `scarecrow173/main` `f6b6d0e2`

## 実施内容(本 slice)

| 区分 | 内容 |
| --- | --- |
| fixture | `ruviform-plugin-test-clap/src/editor.rs` — `TestEditorProcessor(TestFaultProcessor)` への `PluginProcessor` 全委譲(VST3 `editor.rs` と同型)。`TestEditorShared` に `fault_mirror: ParamMirror`、`TestEditorMainThread` に `PluginMainThreadParams`、`TestEditorAudioProcessor` に `PluginAudioProcessorParams::flush` を実装。`declare_extensions` に `PluginParams` 登録。`process()` は `test_fault` と同一 flow: mirror sync-in → `block::translate_param_event(&FAULT_PARAM)` → `processor.process` → mirror sync-out → `bad_time`/`invalid` output `ParamValue` push → `block::map_process_status`。`activate` で armed を default に reset、`reset` で mirror から resync |
| law 共有 | `fault.rs` の `bad_time_output_event`/`invalid_output_event`/`RT_ALLOC_FILL` を `pub(crate)` 化して editor 側から再利用 — fault law の意味変更・fixture 間共有リファクタなし |
| dock-test | `probe_out_of_process_route` の `PluginFormat::Clap` early return 除去 — CLAP も runner resolve → `test_editor` descriptor → `Dedicated` child spawn → `GUI_FLOATING` mask 確認の同一 session-probe を通す。`real_clap_session_crash_case`/`hang_case` を `oos_session_body(..., OosFaultKind::{Crash,Hang})` 呼出に変更(VST3 `real_session_*_case` と同型、ctx から直接 `oos`/`plugin`/`plugin_error`/`window` を渡す)。`clap_oos_unsupported_body` は dead code のため削除 |
| capability | `REAL_CLAP_CATALOG` + `real_clap_entries` の crash/hang rows `required` に `ScenarioCapability::OutOfProcessPluginHost` を追加、`spec_extras` を `real_window_extras` に変更 — VST3 rows と同じ capability-gate(route 未証明環境では `Unsupported` pre-flight verdict を維持) |
| mise | `[tasks.test-ui-plugin-dock-real-clap]` の build 行に `--features l1-runner` 追加(VST3 lane `test-ui-plugin-dock-real` と同形 — runner binary を lane が self-contain)。stale な comment/description を実態に更新(Coordinator 承認済みの lease 拡張、当該 task block のみ) |
| stale 記述 | `real_dock.rs`/`real_catalog.rs`/`README.md`/`lib.rs`/`editor.rs` module docs の "parameterless"・"structural Unsupported"・"co-resident arm 不可"(decision 2(c))記述を実態に更新 |

## 実機実行証拠(Windows、interactive desktop)

実行 tuple:

| 項目 | 値 |
| --- | --- |
| TestID | rf630-plugin-dock-real-clap |
| OS | Windows x86_64(Git Bash 経由の mise/cargo) |
| format | CLAP cdylib `ruviform_plugin_test_clap.dll` + `ruviform-plugin-test-clap-l1-runner.exe`(lane build 行で `--features l1-runner` 付きに配置) |
| command | `mise run test-ui-plugin-dock-real-clap`(fixture + runner build → `plugin-dock-real-scenario --format clap`) |
| exit | 0 |
| evidence | `evidence/rf630/plugin-dock-real-clap.jsonl`(14 case rows)・`evidence/rf630/plugin-dock-real-clap-headless.jsonl`(blocked-probe) |
| repo sha | `f6b6d0e2`(working tree 実走 — 本 slice diff 適用済み。ledger header の慣例どおり base sha を記録) |

観測 verdict(全 14 rows、exit 0):

| verdict | cases |
| --- | --- |
| `Pass` (11) | `real-clap-editor-attach`・`real-clap-dcomp-pixel`・`real-clap-editor-resize`・`real-clap-editor-redock`・`real-clap-editor-visibility`・`real-clap-session-terminate`/`-degraded`/`-recovery`/`-timeout`・**`real-clap-session-crash`**・**`real-clap-session-hang`** |
| `Unsupported` (3) | `real-clap-editor-input`/`-focus`/`-popup`(defined `EditorOpError::Unsupported` refusal through real lease — 本 slice 対象外残件) |

verdict 差分: `real-clap-session-crash`/`real-clap-session-hang` が旧 parameterless 構造根拠の `Unsupported` から `Pass` に変化。他 12 rows は既存 verdict を維持(9 pass + 3 unsupported → 11 pass + 3 unsupported)。

crash/hang の実走中身(ledger `steps`/`observed` から抜粋):

| case | 要点 |
| --- | --- |
| `real-clap-session-crash` | dedicated L1 child 上の `org.ruviform-test.clap-test-editor` を `open(Floating)` attach(real container/child HWND 観測)→ normalized `0.125`(plain `1.0`)で `abort` arm → child `exit_code=-1073740791`(0xC0000409)→ `InstanceFailed` exactly-once → `Terminated{HostLost}` → `Closed` → host/lane window 生存確認 |
| `real-clap-session-hang` | 同上の attach → normalized `0.375`(plain `3.0`)で `hang` arm → wedge probe streak 12 連続 + `SaveState` probe `Err(Ipc)` timeout → watchdog kill(`exit_code=1`、`cause=Timeout`)→ `InstanceFailed` exactly-once → `Terminated{HostLost}` → `Closed` → host/lane window 生存確認 |

## 検証(実 command/exit — canonical = mise lane)

| command | 結果 |
| --- | --- |
| `mise run fmt` / `mise run fmt-check` | exit 0 / exit 0 |
| `mise run check ruviform-plugin-test-clap` | exit 0 |
| `mise run lint ruviform-plugin-test-clap` | exit 0 |
| `mise run test ruviform-plugin-test-clap` | exit 0 — host_path 24 tests + lib tests(`editor_reports_fault_param_in_plain_domain`/`editor_processes_audio_as_pass_through`/`editor_fault_error_mode_is_defined_rejection` 新規、editor.rs in-file fault tests 11 件 新規) |
| `mise run check ruviform-plugin-dock-test` | exit 0 |
| `mise run lint ruviform-plugin-dock-test` | exit 0 |
| `mise run test ruviform-plugin-dock-test` | exit 0 — catalog/entries agreement test 含む全件 |
| `mise run check-l1-real-plugin` | exit 0 |
| `mise run lint-l1-real-plugin` | exit 0 |
| `mise run test-l1-real-plugin` | exit 0 — `--test l1_real_test_plugin_e2e` 20 件・`--test l1_fault_test_plugin_e2e` 21 件・`--test l1_stateful_test_plugin_e2e -- --ignored` 4+6 件 |
| `mise run test-ui-plugin-dock-real-clap` | exit 0 — 上記 verdict 分布(11 pass + 3 unsupported) |
| `mise run test-ui-plugin-dock-real-clap-headless` | exit 0 — 14 rows 全て `not_run`+`environment_blocked`、violation なし |
| `mise exec -- prek run --all-files` | exit 0 |
| `git diff` lease 外パス | 差分なし(mise.toml は Coordinator 承認の範囲拡張) |

## 未検証(NotRun / Unsupported)

| 項目 | 状態 | 理由・後続 |
| --- | --- | --- |
| `real-clap-editor-{input,focus,popup}` | `Unsupported` 継続 | `SendGuiKey`/`SetGuiFocus`/popup verb 実装は設計決裁要・GUI 必要のため本 slice 対象外(census の残件分類どおり) |
| `abort`/`exit`/`hang` in-process unit test | 未対象 | テスト binary 自身が死ぬ/固まる — `none`/`error`/`rt_alloc`/`bad_time`/`invalid` のみ in-process 検証し、crash/hang は L1 out-of-process lane に委譲する既存の責務分割 |
| 統合検証 | NotRun | merge 後 main SHA の CI 証拠は Coordinator が確認 |
