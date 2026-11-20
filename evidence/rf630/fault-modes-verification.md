# RF-VTEST fault-modes slice — 検証記録

- Issue: `scarecrow173/ruviform#630`（test_fault 非致命 fault mode 拡張 slice）
- Slice: `test_fault` の `Fault` param を plain `[0.0, 8.0]`・8 mode に拡張し
  `rt_alloc` / `bad_time` / `invalid` を実 ABI + L1 out-of-process で観測可能にする。
  coordinator 承認の scope 拡張で、`test_stateful` codec v2
  (checksum/migration/degraded/typed rejection) と `test_editor`
  popup 面 (VST3 `VK_P` toggle / CLAP floating window) を同
  lease に畳み込み済み
- OS: Windows (OS-L1) / toolchain: mise 管理 rust（workspace.lints 正本）
- 記録日: 2026-11-21
- deterministic evidence と real-run evidence を分離。

## 実装概要

- `crates/ruviform-plugin-test-{vst3,clap}/src/fault.rs`:
  `FAULT_PARAM.max_value = 8.0`（VST3 `FAULT_STEP_COUNT = 7` → 8 mode）、
  `FaultMode::{RtAlloc, BadTime, Invalid}`、`decode_fault` を
  `floor(plain)` clamp `[0, 8]` の 8 帯域に拡張。armed block 末尾の
  dispatch 契約・last-event-wins・cross-block persistence は不変。
  - `rt_alloc`: `vec![0xC8; 4096]` を `process()` 内で確保・充填、
    `black_box` で除去防止、先頭出力 sample に `200.0` marker
    （fixture 発行の heap round-trip 証拠 — allocator 越境検出は
    観測不能のため不採用）。
  - `bad_time`: `Continue + TAIL_INFINITE` を返す timing lie。
    VST3 は `getTailSamples() == u32::MAX`、CLAP は
    `ClapProcessStatus::Tail` → host 再構成で wire `tail_frames`
    が `u32::MAX`。
  - `invalid`: 全出力 channel/sample を `NaN` fill、status は
    `Continue`（出力と status の矛盾する不正状態）。
  - CLAP のみ追加 leg: `events.output` に `bad_time` → block 外
    時刻 `ParamValue`、`invalid` → NaN 値 `ParamValue` を
    `try_push`（v1 host bridge は void — 発行面の honest record）。
- `tests/host_path.rs`（両 crate）: descriptor `max_value == 8.0`
  pin、`error` arm を normalized `0.5` に rescale、新 3 mode の
  ABI 実走 + disarm 回復 test。
- `tests/l1_fault_test_plugin_e2e.rs`（両 crate）: normalized
  対応表・既存 5 case arm 値を `[0.0, 8.0]` law に rescale、
  新 3 L1 case（marker / `tail_frames == u32::MAX` / 全 NaN
  観測 → `Processing` 維持 → disarm 回復 → clean teardown）。
  CLAP に `submit_block_full` helper を新規追加。
- `src/stateful.rs`（両 crate）: codec を v2 化 —
  `RFST` + version=2 + f64 + FNV-1a-32 checksum (20B canonical)。
  `inspect_state_blob` が `Canonical`/`Migrated`/`Degraded`
  provenance と `Malformed`/`UnsupportedVersion`/
  `ChecksumMismatch` typed rejection を返す単一 decode 入口。
  `decode_state_blob` の `Option<f64>` 互換面は維持
  （`component.rs`/`controller.rs`/`preset.rs`/`single.rs`
  shared caller は zero-diff）。CLAP `load` は stream を
  `read_to_end` 全消費し inspect 成功時のみ gain mirror 更新 +
  params rescan 通知。
- `src/editor_view.rs`（VST3）: `ViewState.popup`/`popup_seq`、
  `POPUP_VK = 0x50`。`onKeyDown` は `kd:` 記録後 `VK_P` のみ
  toggle — open `po:{seq}` / close `pc:{seq}` を event log
  (= window title) に記録、`win32::popup` が root ancestor
  owner の `WS_POPUP|WS_VISIBLE|WS_CLIPSIBLINGS` HWND
  (`rvf-test-editor-popup-{seq}`) を生成。`removed()` は popup
  を畳んでから child を破棄。
- `src/editor.rs`（CLAP）: `GuiState.floating`/`transient`/
  `title`。`is_api_supported` は Windows で `is_floating` 両方
  を受理、`create` が mode 記録、floating 時 `set_transient`
  （live HWND gate）+ `suggest_title` + `show`（`WS_POPUP`
  実生成、transient owner + 提案/default title）。混在 verb・
  dead handle・非 Win32 は `PluginError` rejection。

## 受入条件の照合

| 受入条件 | 結果 | 証拠 |
| --- | --- | --- |
| `Fault` `[0.0, 8.0]`・stepped を両 ABI で報告、既存 5 mode 不変 | Passed | host_path descriptor pin（`max_value == 8.0`）+ 既存 L1 5 case 全緑 |
| `rt_alloc` bounded 確保 + `200.0` marker + pass-through 残部 | Passed | unit `alloc_fault_marks_armed_block` + host_path + L1 `real_test_fault_l1_rt_alloc_marks_armed_blocks`（両 ABI） |
| `bad_time` → wire `tail_frames == u32::MAX`、disarm で 0 | Passed | unit + host_path + L1 `real_test_fault_l1_bad_time_claims_infinite_tail`（両 ABI） |
| `invalid` 全 NaN、`Processing` 維持、disarm 回復 | Passed | unit + host_path + L1 `real_test_fault_l1_invalid_emits_nan_block`（両 ABI） |
| in-process test + L1 e2e が両 format で揃う | Passed | 下記 deterministic / real-run 表 |
| `test_stateful` codec v2: canonical save / v1 migrate / degraded 受理 / typed rejection + 状態不変 | Passed | unit `inspect_reports_provenance_and_typed_rejections`・`v1_and_degraded_blobs_load_and_resave_canonical` 等（両 crate）+ host_path `stateful_restore_*` + L1 `real_stateful_test_plugin_l1_migrates_and_degrades_state` |
| `test_editor` popup: VST3 `VK_P` toggle HWND + `removed()` 畳み込み、CLAP floating `show` 実 HWND | Passed | `popup_key_toggles_plugin_owned_window` / `floating_popup_surface_is_honest` unit（Windows で実 HWND 観測 — real-HWND host harness 不在のため L1 面なし、AC8 どおり plugin-impl が oracle） |
| L1 stateful e2e が v2 20B + migration/degraded/checksum/unsupported を out-of-process 観測 | Passed | `l1_stateful_test_plugin_e2e` VST3 5/5・CLAP 3/3（下記 real-run 表） |
| 既存 lane 緑のまま | Passed | fmt-check / lint / test / test-rt / prek 全緑。VST3 `hang_converges_on_timeout` は並列実行時 spawn タイムアウト flake を 1 回観測（単独再実行で pass — spawn handshake 経路は本変更と無関係） |

## deterministic evidence

正規 lane（`mise.toml` のタスク）経由。

| コマンド | 結果 |
| --- | --- |
| `mise run fmt-check` | Passed |
| `mise run lint ruviform-plugin-test-vst3` | Passed（clippy `-D warnings`） |
| `mise run lint ruviform-plugin-test-clap` | Passed（clippy `-D warnings`） |
| `mise run test ruviform-plugin-test-vst3` | Passed（unit 107 + host_path 24+1 ignored 含む — fault 新 3 mode + `inspect_reports_provenance_and_typed_rejections`・`v1_component_blob_migrates_on_next_save`・`popup_key_toggles_plugin_owned_window` 等） |
| `mise run test ruviform-plugin-test-clap` | Passed（unit 77 + host_path 21+1 ignored — fault 新 3 mode + `inspect_reports_provenance_and_typed_rejections`・`v1_and_degraded_blobs_load_and_resave_canonical`・`floating_popup_surface_is_honest` 等） |
| `mise run check-l1-real-plugin` | Passed（`l1-runner` feature の全 target 型検査） |
| `mise run lint-l1-real-plugin` | Passed（`-D warnings`） |
| `mise run test-rt` | Passed（workspace `rt_` filter — 両 crate の `rt_unarmed_block_is_alloc_free` 含む） |
| `mise exec -- prek run --all-files` | Passed |

## real-run evidence

`mise run test-l1-real-plugin`（実 cdylib + l1-runner spawn、
`#[ignore]` gated）:

| lane | 結果 |
| --- | --- |
| adapter bridge + runner-bin unit tests | Passed（71 + 2 / 86 + 2） |
| `l1_real_adapter_e2e` | Passed（3 + 3） |
| `l1_real_test_plugin_e2e` | Passed（10 + 10） |
| `l1_fault_test_plugin_e2e` (CLAP) | Passed（19/19 — 新規 3 case 含む） |
| `l1_fault_test_plugin_e2e` (VST3) | 18/19 pass — `hang_converges_on_timeout` が spawn handshake Timeout で 1 回失敗。並列 child spawn 負荷下の flake であり、単独再実行（`cargo test -p ruviform-plugin-test-vst3 --features l1-runner --test l1_fault_test_plugin_e2e real_test_fault_l1_hang_converges_on_timeout -- --ignored --exact`）で pass を確認 |
| `l1_stateful_test_plugin_e2e` | Passed — 拡張 scope 後は VST3 5/5・CLAP 3/3（v2 20B payload、`_migrates_and_degrades_state`・`_rejects_malformed_state` が migration/degraded/checksum/unsupported-version + reject 後生存・canonical re-save・restored gain audio 反映を網羅） |

## 観測境界（honest boundary）

- `rt_alloc` の確保は marker のみが証拠 — host allocator が
  fixture dylib 内の確保を直接観測したわけではない。
- CLAP `events.output` の malformed event は v1 host bridge が
  `void` に捨てるため host 側未観測 — 発行面のみ。
- `bad_time` は tail 値の虚偽のみ — block timestamp/時刻系の
  他面（`ProcessData.time`/steady-time ずらし等）は未注入。
- VST3 `hang_converges_on_timeout` の並列実行 flake は既存
  child-spawn タイミング依存 — 本変更と無関係だが再現性の
  ある環境では再観測余地あり。

## 残件（本 slice 外）

- real dock/DComp gate、consumer oracle 照合。
- editor popup の real-HWND host harness / L1 面（AC8 は
  plugin-impl 単体 test が oracle — production `open_gui` 経由
  の実 dock は #631 real-dock lane の対象）。
- output event injection の host 側観測経路（v1 bridge の
  `void` ではなく drain する後続 slice 対象）。
