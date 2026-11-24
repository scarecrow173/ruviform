# Issue #912 検証記録 — plugin→host param 値報告の全面配線

- ブランチ: `feat/912-param-value-report`（ベース: `origin/main` c7af983b への fast-forward 取り込み済み。取り込み差分は `.github/`・`docker/`・`docs/`・`evidence/` のみで、本 slice の crate に非接触 — 検証結果をそのまま流用）
- 環境: Windows（本 Worker worktree `rf912-param-report`）、Rust は `mise.toml` pin の stable
- 方針: 正規コマンドは `mise run <task>` 経由。cargo 直接実行は補助証拠として扱う。`POLICIES.md` §5 の比例原則に従い、変更クレートとその直接影響面を lane 実走した。

## 検証結果サマリ

| lane | コマンド | 結果 |
| --- | --- | --- |
| fmt | `mise run fmt` / `mise run fmt-check` | Pass（exit 0、差分なし） |
| plugin-core | `mise run check ruviform-plugin-core` | Pass |
| plugin-core | `mise run lint ruviform-plugin-core` | Pass |
| plugin-core | `mise run test ruviform-plugin-core` | Pass（151 + 14 + 8 + doctest 37） |
| plugin-vst3 | `mise run check/lint/test ruviform-plugin-vst3` | Pass（44 lib tests） |
| plugin-clap | `mise run check/lint/test ruviform-plugin-clap` | Pass（default feature。`host` 面は下記 l1 lane が canonical） |
| plugin-host | `mise run check/lint/test ruviform-plugin-host` | Pass（237 lib + 58 + 18 + 5 e2e-suite + doctest 3） |
| plugin-export | `mise run check/lint/test ruviform-plugin-export` | Pass |
| plugin-test-vst3 | `mise run check/lint/test ruviform-plugin-test-vst3` | Pass（121 + 26） |
| plugin-test-clap | `mise run check/lint/test ruviform-plugin-test-clap` | Pass（90 + 26） |
| L1 real-plugin check | `mise run check-l1-real-plugin` | Pass |
| L1 real-plugin lint | `mise run lint-l1-real-plugin` | Pass（`-D warnings`） |
| L1 real-plugin test | `mise run test-l1-real-plugin` | **全 step Pass**（下記詳細） |
| prek | `mise exec -- prek run --all-files` | Pass（全 hook green / 対象外 Skipped） |

## `test-l1-real-plugin` 詳細（Issue #912 oracle を含む）

全 step exit 0。特記:

- `l1_real_adapter_e2e -- --ignored`（両 ABI）: `real_adapter_l1_full_series` 含め 3/3 Pass × 2 — Spec が「base 既存破損（`CloseGui` 語彙差）」として記録していた step は本環境・現行 main では再現せず Pass。
- `l1_real_test_plugin_e2e -- --ignored`（両 ABI）: 11/11 Pass × 2 — `real_test_plugin_l1_full_series` も Pass（同上）。
- `l1_fault_test_plugin_e2e -- --ignored`: 20/20 + 21/21 Pass。
- `l1_stateful_test_plugin_e2e -- --ignored`:
  - CLAP 5/5 Pass — 新規 oracle `real_stateful_test_plugin_l1_state_restore_requests_param_rescan` Pass（`RestoreState` → fixture `rescan(VALUES)` → `PluginHost::drain_param_notices` で `RescanRequested{VALUES}` 到達を確認。非変更 restore では空 drain）。
  - VST3 7/7 Pass — 新規 oracle `real_stateful_test_plugin_l1_state_restore_reports_param_value` Pass（`RestoreState` → fixture `performEdit(0, 0.5)` → `drain_param_notices` で `ValueChanged{param_id:0, value:0.5}` 到達を確認。dirty 面の `ParamEdited`+`StateDirty` は別面で維持 — `announces_dirty` 既存 oracle も継続 Pass）。

## 実行時メモ

- `mise run test ruviform-plugin-host` は `ruviform-plugin-l1-runner` バイナリの実体を前提とする（`ruviform-plugin-export` の `src/bin/`）。fixture 未生成の初回実行では `editor_session_l1` 系 5 件が「runner missing」で失敗する — `cargo build -p ruviform-plugin-export --bins` 後に全 Pass（環境前提であり、本 slice 起因の失敗ではない）。
- 補助証拠（非 canonical、参考のみ）: `cargo test -p ruviform-plugin-clap --all-features --lib` で 82/82 Pass。`#[cfg(all(test, feature = "host"))]` の fixture dlopen 系 16 件は `ruviform_plugin_clap` cdylib（同 crate 自身の cdylib ターゲット、`cargo build -p ruviform-plugin-clap` で生成）の存在が前提。
- `mise run lint ruviform-plugin-clap`（default feature, `host` 無し）は Spec 記録では dead_code 既存破損だったが、本 slice で `translate_output_notice` を `#[cfg(any(test, feature = "host"))]` 下に追加した関係で参照面が変わり Pass に転じた。

## 設計整合の確認（受入条件との対応）

1. 中立語彙 `ParamNotice` + 専用 pull verb（`Command::DrainParamNotices` tag 31 / `Reply::ParamNotices` tag 10 / `WIRE_VERSION=9`）— 案 C（専用 enum + 専用 verb）を採用、`Event`/`DirtyNotice` 拡張案の却下根拠は `design-param-value-report.md` に記録。
2. VST3 `performEdit` の値 → `ParamNotice::ValueChanged`（dirty 面 `ParamEdited` と二面 enqueue）。`beginEdit`/`endEdit` → `GestureBegin`/`GestureEnd`（`frame_offset: None`）。`restartComponent` は全 flag を `RescanRequested` に配送しつつ応答は従来どおり正直（`kParamValuesChanged` exact のみ `kResultOk`）— report≠fulfill。CLAP `rescan`/`clear`/`request_flush`/`loaded`/`on_error` → `RescanRequested`/`ClearReferences`/`FlushRequested`/`PresetLoaded`/`PresetLoadFailed`、process 由来 gesture event → `GestureBegin`/`GestureEnd`（`frame_offset: Some(t)`）。
3. 値到達 oracle: 両 ABI で L1 e2e 実走 Pass（上記）。
4. 設計記録: `evidence/rf912/design-param-value-report.md`（docs/designs は lease 外 — 統合時に Coordinator が移す）。
5. 全 lane 実走・結果正直記録（本ファイル）。NotRun なし。

## 既存破損の再現確認

- Spec の「`test-l1-real-plugin` step3/step6 が base 既存破損」は本環境では再現せず（全 step Pass）。環境差・または現行 main で解消済みと判断し、失敗としては報告しない。
