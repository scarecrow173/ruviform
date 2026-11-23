# RF630 AC4 CLAP leg 実施 report — plugin-dock real-window lane の CLAP format 対応

- Issue: https://github.com/scarecrow173/ruviform/issues/630 (AC4 CLAP leg: `ruviform_plugin_test_clap.dll` `test_editor` 実走 + JSONL evidence 化)
- Branch: `test/630-clap-realwindow-dock-lane`
- Base: `scarecrow173/rf630-clap-dock` (main `a0e34564`)

## 実施内容(本 slice)

| 区分 | 内容 |
| --- | --- |
| format 抽象 | `PluginFormat::{Vst3,Clap}` — artifact env var・fixture filename・run/scenario id・L1 runner filename・catalog entries・`EDITOR_COLOR` predicate・evidence key を format で選択。`PluginLibrary::{Vst3,Clap}` が `load`/`descriptors()`/`factories()` を統一(`ClapLibrary`/`Vst3Library` 同一 surface) |
| artifact | `RUVIFORM_PLUGIN_DOCK_TEST_CLAP` env var + `fixture_lib_name_clap()` + generic `resolve_artifact_named`/`candidate_paths_named`(VST3 wrapper 不変)。明示 path 優先 → `current_exe()/deps/` → exe dir、missing artifact は `NotRun` fail-closed |
| catalog | `real_catalog::REAL_CLAP_CATALOG` 14 rows + `real_dock::real_clap_entries`(実行対)。`CLAP_RUN_ID=rf630-plugin-dock-real-clap`・`CLAP_SCENARIO=rf630.plugin-dock-real-clap` — VST3 evidence と完全分離(`plugin_path_env` header が format を正直に記録) |
| runner | `plugin-dock-real-scenario --format vst3\|clap`(default `vst3`)。`run_all_with_format` が format 別 catalog/runner/probe/mount 構成を選択 — CLAP は sentinel window のみ mount(VST3 の A–D multiwindow/IME/close/receipt rows は catalog 上不対象) |
| oracles | blue predicate(`b>=200 && r<=80 && g<=80`)+ `{green,blue}_pixels`/`_fraction` dynamic keys。CLAP visibility = live-child hide/show(同一 HWND set で `IsWindowVisible` toggle — VST3 destroy/recreate ではない)。CLAP redock = single-embed `gui already embedded` refusal(`Failed{PluginFailed}`)→ gen-1 close → **fresh session** open で gen-2 disjoint HWND set(`recover` は still-embedded GUI に `ops.open` を再実行するため新 GUI を mint できず、`Closed` は terminal) |
| defined-Unsupported | input/focus/popup — `SendGuiKey`/`SetGuiFocus` が CLAP L0 ops surface の trait-default `Unsupported` refusal(title-stamp event log を持つ VST3 oracle は流用不可) |
| structural-Unsupported | crash/hang — `test_editor` は parameterless(in-band Fault arm なし)かつ `PluginHost` routes が `plugin_id` keyed + per-route Bundled pools のため co-resident `test_fault` arming が既存 API では不可能(Coordinator decision 2(c)) |
| paint drain | `drain_paint` note を shared pixel path 全 children に追加 — child HWND は scenario caller thread 所有で同 thread が queue を pump しないため、fixture の `hbrBackground` fill が `WM_ERASEBKGND` queue に滞留し zero-fraction frame として観測される flake を修正(Issue #532 (a2) `osmut::drain_paint` 再利用。VST3 row にも適用 — 無害・機構の可視化) |
| mise | `test-ui-plugin-dock-real-clap` / `test-ui-plugin-dock-real-clap-headless` tasks |

## 実機実行証拠(Windows、interactive desktop)

実行 tuple:

| 項目 | 値 |
| --- | --- |
| TestID | rf630-plugin-dock-real-clap |
| OS | Windows(x64、Git Bash 経由の mise/cargo) |
| format | CLAP cdylib `ruviform_plugin_test_clap.dll`(`target/debug/deps/` 解決、artifact sha `…809a30bad60c` を全 case に記録) |
| command | `mise run test-ui-plugin-dock-real-clap`(fixture build + `plugin-dock-real-scenario --format clap`) |
| exit | 0 |
| evidence | `evidence/rf630/plugin-dock-real-clap.jsonl`(14 case rows)・`evidence/rf630/plugin-dock-real-clap-headless.jsonl`(blocked-probe) |

観測 verdict(全 14 rows、exit 0):

| verdict | cases |
| --- | --- |
| `Pass` (9) | `real-clap-editor-attach`(real child HWND attach+destroy)・`real-clap-dcomp-pixel`(screen `blue_fraction=0.9999`、`drain_paint` note が `update_before:true→cleared:true` を記録)・`real-clap-editor-resize`(`GetWindowRect` 800×600)・`real-clap-editor-redock`(refusal→close→fresh-session gen-2、HWND sets disjoint)・`real-clap-editor-visibility`(同一 HWND set で hide→show)・`real-clap-session-terminate`/`-degraded`/`-recovery`/`-timeout` |
| `Unsupported` (5) | `real-clap-editor-input`/`-focus`/`-popup`(defined `EditorOpError::Unsupported` refusal through real lease)・`real-clap-session-crash`/`-hang`(structural — co-resident `test_fault` arm 不可) |

非退行: `mise run test-ui-plugin-dock-real`(VST3)→ 全 32 rows `Pass`・exit 0・`green_fraction=0.9999`・`green_*` key 互換維持。

## 検証(実 command/exit)

| command | 結果 |
| --- | --- |
| `mise exec -- cargo check -p ruviform-plugin-dock-test --features real-dock --all-targets` | exit 0 |
| `mise run lint ruviform-plugin-dock-test` | exit 0 |
| `mise run lint ruviform-plugin-dock-test --features real-dock` | exit 0 |
| `mise run test ruviform-plugin-dock-test --features real-dock` | exit 0 — 37 passed・0 failed(`real_clap_catalog_matches_entries`/`blocked_reports_fail_closed` 含む) |
| `mise run fmt-check` | exit 0 |
| `mise run test-ui-plugin-dock-real-clap` | exit 0 — 9 Pass + 5 Unsupported(本 report 参照) |
| `mise run test-ui-plugin-dock-real-clap-headless` | exit 0 — 14 rows 全て `not_run`+`environment_blocked`、violation なし・`plugin_path_env=RUVIFORM_PLUGIN_DOCK_TEST_CLAP` |
| `mise run test-ui-plugin-dock-real` | exit 0 — VST3 32 rows 全 Pass(非退行) |
| `mise run test-ui-plugin-dock-real-headless` | exit 0 — VST3 fail-closed proof 無改変 |

## 未検証(NotRun / Unsupported)

| 項目 | 状態 | 理由・後続 |
| --- | --- | --- |
| `real-clap-session-crash`/`hang` L1 fault 実走 | `Unsupported` | `test_editor` parameterless + per-`plugin_id` route/Bundled pool — co-resident `test_fault` arming には `ruviform-plugin-host` API 変更が必要(本 slice 境界外、Coordinator decision 2(c)) |
| CLAP `test_fault` 経由の RT/fault lanes | 未対象 | 本 catalog は `test_editor` のみ — VST3 `real-session-rt-*` 相当は後続 |
| `--pixel-artifacts` PNG 出力 | 未実走 | opt-in; canonical evidence は digest のみ |
