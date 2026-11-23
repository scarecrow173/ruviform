# rf389 real-session-lifecycle — verification record

Issue `scarecrow173/ruviform#389` の real-window session lifecycle
slice（`rf631.plugin-dock-real` lane 15 → 21 row — degraded /
recovery / timeout pass-oracle、visibility refusal observation、
crash/hang honest `Unsupported` boundary）のローカル検証記録。
canonical = `mise` lane。

- Date: 2026-11-17
- OS: Windows (x86_64) — interactive desktop session あり（real lane 実走）
- Toolchain: rustc 1.99.1 / cargo 1.99.1（mise 管理）
- Repo SHA: コミット済み lane JSONL
  （`plugin-dock-real-scenario.jsonl` /
  `plugin-dock-real-headless.jsonl`、`evidence/rf631/` 側コピー
  含む）の `repo.sha` / `environment.repo_sha` は **両 lane とも**
  `62d595b60e27466b2f714f4211c71b16c47ca1eb`（`origin/main`
  merge commit）— merge 後に required-capability 修正
  （後の commit `82034fcc` の内容）を working tree へ適用した
  状態で両 lane を再実走したため。`62d595b6..HEAD` の `crates/`
  diff は `real_catalog.rs` / `real_dock.rs` の required
  capability 4 行のみで、これは lane 実行時の working tree に
  既に存在した内容と一致する（`d192ae32` は docs index
  revert のみで crate code 差分なし）— lane が実行したコードと
  HEAD の lane 関連コードに機能差分はない。なお commit
  `2810a087` 時点の JSONL は merge 前初回実走の `303db66b`
  （作業開始時 HEAD、commit 前 working tree）を記録していたが、
  `82034fcc` で再実走版に更新済み。

## Lane results

| Command | Result |
| --- | --- |
| `mise run fmt-check` | pass (exit 0) |
| `mise run lint ruviform-plugin-dock-test` | pass (exit 0, `-D warnings`) |
| `mise run check ruviform-plugin-dock-test` | pass (exit 0) |
| `mise run test ruviform-plugin-dock-test` | pass (exit 0) |
| `mise run check ruviform-plugin-dock-test --features real-dock` | pass (exit 0) |
| `mise run lint ruviform-plugin-dock-test --features real-dock` | pass (exit 0, `-D warnings`) |
| `mise run test ruviform-plugin-dock-test --features real-dock` | pass (exit 0 — `real_catalog_matches_entries`・`blocked_reports_fail_closed`・console `list_include_real_appends_real_lane_rows` 29 行を含む全 test green) |
| `mise run check ruviform-plugin-dock-test --features lab-gui` | pass (exit 0) |
| `mise run lint ruviform-plugin-dock-test --features lab-gui` | pass (exit 0) |
| `mise run test ruviform-plugin-dock-test --features lab-gui` | pass (exit 0 — Completion Gate 指摘により追実走・全 test green) |
| `mise run build ruviform-plugin-test-vst3`（`test-ui-plugin-dock-real` 内） | pass (exit 0 — `test_editor`/`test_gain` cdylib 生成） |
| `mise run test ruviform-plugin-host --test editor_session --test editor_session_driver` | pass (exit 0) |
| `mise run test-ui-plugin-dock` | pass (exit 0) |
| `mise run test-ui-plugin-dock-real-headless` | pass (exit 0 — 21 行全て `not_run`+`observed.environment_blocked`、violation 0) |
| `mise run test-ui-plugin-dock-real` | exit 1 — **設計どおりの正直な regression 信号**: 既知 `real-dcomp-pixel` が `fail`（GPUI DComp topmost occlusion、`scarecrow173/ruviform#532` 決裁待ち・本 slice で変更なし・baseline 実走でも同一 fail を確認済み）。新規 6 row は設計どおりの verdict |
| `mise run test-ui-plugin-dock-console` | pass (exit 0) |
| `mise run check-ui-driver-boundary` | pass (exit 0 — dep-graph/signature/allowlist) |
| `mise exec -- prek run --all-files` | pass (exit 0) |

補助証拠（canonical = mise）: `plugin-dock-console --list
--include-real`（`--features real-dock`）を
`plugin-dock-console-list-include-real.jsonl` に保存 — 29 行
（headless 8 + real 21）projection、6 新規 row 含む全件一致。

## `test-ui-plugin-dock-real` 実走 verdict（`plugin-dock-real-scenario.jsonl`）

interactive Windows session で全 21 case を実走。新規 6 row の
観測 step 詳細:

| case | verdict | 観測 |
| --- | --- | --- |
| `real-session-degraded` | **pass** | `open_or_degrade(Floating→Docked)` → `Degraded{Docked}` fallback attach。events `[Degraded { mode: Docked }, Closed]`（`Attached` 非発火）+ child HWND contract/enumeration/teardown 全 pass |
| `real-session-recovery` | **pass** | `open(Floating)` → `Failed{Unsupported}`（`failed_unsupported=true`・`editor_live=false`）→ `recover(Docked)` attach。events `[Failed { cause: Unsupported }, Recovered { mode: Docked }, Closed]` + child HWND contract 全 pass |
| `real-session-timeout` | **pass** | `SessionDeadlinePolicy{timeout: 0s}` → `begin_detach`（request=2・deadline armed `3.6µs`）→ `tick_all()==1` → `Failed{Timeout}`。events `[Attached { mode: Docked }, Failed { cause: Timeout }, Closed]` + child HWND teardown 全 pass |
| `real-editor-visibility` | **unsupported** | `set_visibility(false)` → `EditorOpError::Unsupported`（no hide/show verb on L0 surface）。session は `Attached{Docked}` 維持・events `[Attached { mode: Docked }, Closed]`（`Hidden`/`Shown` 0 件）を確認した上で refused boundary として記録 |
| `real-session-crash` | **unsupported** | in-process lane は fixture の process-killing crash fault を生存できず out-of-process plugin host は存在しない — fault を実行せず boundary row として記録 |
| `real-session-hang` | **unsupported** | 同上 — wedging/hang fault は lane を永久に塞ぐため実行しない。architectural boundary を明示 |

既存 15 row の verdict は前 slice から不変（`real-dcomp-pixel`
のみ既知 `fail` — #532 決裁待ちの GPUI DComp topmost occlusion、
baseline/ baseline2 / dcomp-disabled の 3 実走でも同一
`green_pixels=0` fail を確認済みで本 slice 起因ではない）。

## 設計との対応

- degrade/recovery/timeout は `attempt_real_attach` の `drive_open`
  seam 経由で case 固有の session op を実行 — open 成功時のみ
  observe が lease 生存中に走り、close → child teardown →
  event 収束の共有 tail を通る。
- `real_editor_ops` は `RealOsGuiTest::redeem_native_lease` 経由の
  実 ledger redeem のみ — fake/direct HWND path なし。
- `real_catalog.rs`（driver-neutral）と `real_dock.rs`
  `real_entries`（executable）は case id / scenario id /
  required capabilities で 1:1 同期 — `real_catalog_matches_entries`
  test が gate。
- crash/hang は `required: &[NativePluginDock]` で preflight を通り
  body が `Unsupported` を返す — `NotRun` 偽装・catalog 欠落なし。

---

# rf389 editor-verbs — verification record

Issue `scarecrow173/ruviform#389` の adapter editor verbs slice
（`InProcessEditorOps` の `set_size`/`set_visibility`/`reattach`
defined `Unsupported` refusal を除去し、plugin-core trait →
`HostCommand` → host dispatch → composite → VST3/CLAP adapter の
production chain へ実装。`rf631.plugin-dock-real` lane の
`real-editor-resize`/`real-editor-redock`/`real-editor-visibility`
を refusal/deferred から実実行 + effect oracle 検証へ flip）の
ローカル検証記録。canonical = `mise` lane。

- Date: 2026-11-18
- OS: Windows (x86_64) — interactive desktop session あり（real lane 実走）
- Toolchain: rustc 1.99.1 / cargo 1.99.1（mise 管理）
- Repo SHA: lane JSONL の `environment.repo_sha` は
  `b6d0cc6a2e14f47e5324061d8dcac3d905e4abd5`（branch HEAD、
  本 slice の変更は working tree 未コミット状態で実走）。

## Lane results

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | pass (exit 0) |
| `cargo test -p ruviform-plugin-host --test editor_session` | pass — 54/54（`l0_set_size_reaches_plugin_through_setguisize`・`l0_set_visibility_cycles_hidden_shown_through_host`・`l0_reattach_redeems_lease_and_reaches_set_gui_parent`・`l0_session_open_close_leaves_dsp_processing_untouched` 等を含む） |
| `cargo test -p ruviform-plugin-test-vst3` | pass — unit 91 + integration 16、fixture の `canResize == kResultTrue`・`checkSizeConstraint`/`onSize` 契約 test を含む |
| `cargo clippy -p ruviform-plugin-{core,host,vst3,clap,test-vst3} --all-targets` | pass — warning 0 |
| `cargo clippy -p ruviform-plugin-dock-test --all-targets --features real-dock` | pass — warning 0 |
| `cargo test -p ruviform-plugin-dock-test` | pass — 4/4 integration + unit 全 green |
| `cargo test -p ruviform-plugin-dock-test --features real-dock` | pass — `real_catalog_matches_entries`・`blocked_reports_fail_closed`・console 14/14 を含む全 green |
| `mise run test-ui-plugin-dock-real-headless` | pass — 22 case row 全て `not_run` + `observed.environment_blocked`、violation 0、exit 0 |
| `mise run test-ui-plugin-dock-real` | exit 1 — `real-dcomp-pixel` の既知 `fail`（GPUI DComp topmost occlusion・Issue #532 決裁待ち・本 slice 対象外）による設計通りの exit code |

## Real lane verdicts（`evidence/rf631/plugin-dock-real-scenario.jsonl`）

| case | verdict | 内容 |
| --- | --- | --- |
| `real-editor-resize` | **pass** | `session.set_size(800×600)` が REAL container lease 経由で accepted。child HWND の `GetWindowRect` が要求サイズと一致（fixture `onSize` が自 window を resize）、session `Attached` 維持、events `[Attached{Docked}, Closed]`、close 収束を実測 |
| `real-editor-redock` | **pass** | `session.reattach` が同一 live lease で accepted。pre-reattach child HWND 全て破棄（`removed()` → `DestroyWindow`）、新 attach generation（`rvf-test-editor-attach-{n}` title 前進）が container 配下に出現、events `[Attached{Docked}, Detached, Reattached{Docked}, Closed]` を実測 |
| `real-editor-visibility` | **pass** | `set_visibility(false)` → `Hidden{Docked}` で pre-hide child HWND 全破棄、`set_visibility(true)` → `Attached{Docked}` で child contract が新 generation で再成立、events `[Attached{Docked}, Hidden, Shown{Docked}, Closed]` を実測 |
| `real-window-ime` / `real-session-crash` / `real-session-hang` | unsupported | 既存の honest boundary row（本 slice 無変更） |
| `real-dcomp-pixel` | fail | 既知・対象外（#532） |

その他の real row は全て `pass`。

## Evidence

- `evidence/rf631/plugin-dock-real-scenario.jsonl`（canonical、
  本 slice 実走版）
- `evidence/rf631/plugin-dock-real-headless.jsonl`（blocked-probe
  fail-closed proof、本 slice で再実走）
- `evidence/rf389/plugin-dock-real-scenario.jsonl` /
  `plugin-dock-real-headless.jsonl`（本 slice snapshot コピー）

---

# 追記 — real-window-ime slice 着地確認と HEAD `ff7013dc` 再実走記録

Issue `scarecrow173/ruviform#389` の `real-window-ime` slice
（`ScenarioCapability::Ime` backend-presence claim + IMM32 OS-level
composition drive + caller-side subclass `WM_IME_*` 配達 oracle +
正直な `environment_blocked` fallback）は `scarecrow173/ruviform#875`
で main 着地済み（`scarecrow173/ruviform#879` で非 Windows
dead-code 修正が追従）。本記録は、着地後に main へ入った
`scarecrow173/ruviform#906`（io-structured・無関係）と
`scarecrow173/ruviform#907`（plugin-host L1EditorOps +
composite/host GUI routing — dock lane の session/attach 経路に
隣接）を含む HEAD での全 lane 再実走による回帰確認と、
`real_catalog.rs`/`real_dock.rs` に残存していた pre-#875
stale 説明文の修正に対する検証記録。canonical = `mise` lane。

- Date: 2026-11-23
- OS: Windows (x86_64) — interactive desktop session あり（real lane 実走）
- Toolchain: rustc 1.99.1 / cargo 1.99.1（mise 管理）
- Repo SHA: lane JSONL の `repo.sha`/`environment.repo_sha` は
  `ff7013dc7fab47e2277f872347973c662e06ea8e`（branch HEAD —
  stale comment 修正 commit。実走は commit 済み tree で実施）。
- Host IME: active layout `HKL 0x04110411`（Microsoft Japanese
  IME）— `ime_preflight` step が `ime_layout`/`ime_context`/
  `ime_ui` 全 check `effective:true` で `pass`。

## Lane results

| Command | Result |
| --- | --- |
| `mise run fmt-check` | pass (exit 0) |
| `mise run check ruviform-ui-gpui` | pass (exit 0) |
| `mise run lint ruviform-ui-gpui` | pass (exit 0, `-D warnings`) |
| `mise run test ruviform-ui-gpui` | pass (exit 0 — default feature では real-window 系 test は cfg で非対象・0 test) |
| `mise run check ruviform-ui-gpui --features real-scenario` | pass (exit 0) |
| `mise run lint ruviform-ui-gpui --features real-scenario` | pass (exit 0, `-D warnings`) |
| `mise run test ruviform-ui-gpui --features real-scenario` | **flaky 1 件** — `tests/real_window.rs::real_os_input_scenario` が初回 `NotRun — window w1 is not the focused window`（foreground-lock 環境 timing）で fail、単独再実行で **pass**。`os_input` 系の既存 flaky 性で本 diff（comment のみ）と無関係。`os_ime` unit test 8/8 を含む他 42 test は全 green |
| `mise run check ruviform-plugin-dock-test` | pass (exit 0) |
| `mise run lint ruviform-plugin-dock-test` | pass (exit 0, `-D warnings`) |
| `mise run test ruviform-plugin-dock-test` | pass (exit 0 — unit 2 + console 14 + plugin_dock 4) |
| `mise run check ruviform-plugin-dock-test --features real-dock` | pass (exit 0) |
| `mise run lint ruviform-plugin-dock-test --features real-dock` | pass (exit 0, `-D warnings`) |
| `mise run test ruviform-plugin-dock-test --features real-dock` | pass (exit 0 — unit 17 + console 14 + plugin_dock 5) |
| `mise run check ruviform-plugin-dock-test --features lab-gui` | pass (exit 0) |
| `mise run lint ruviform-plugin-dock-test --features lab-gui` | pass (exit 0, `-D warnings`) |
| `mise run test ruviform-plugin-dock-test --features lab-gui` | pass (exit 0 — lab bin unit 7 を含む全 green) |
| `mise run build ruviform-plugin-test-vst3` | pass (exit 0) |
| `mise run test ruviform-plugin-host --test editor_session --test editor_session_driver` | pass (exit 0 — 57/57 + 18/18、`l1_editor_ops_refuses_every_op_on_an_unadvertised_child` を含む) |
| `mise run test-ui-plugin-dock` | pass (exit 0 — `plugin-dock-scenario.jsonl` 再生成) |
| `mise run test-ui-plugin-dock-console` | pass (exit 0 — `plugin-dock-console.jsonl` 再生成) |
| `mise run test-ui-plugin-dock-real-headless` | pass (exit 0 — blocked-probe 32 row 全て `not_run` + `environment_blocked`、violation 0) |
| `mise run test-ui-plugin-dock-real` | exit 1 — 既知 class C `real-dcomp-bandescape`/`real-dcomp-childinv` の `fail` による設計通りの exit code（interactive 実走・全 32 case `real-window` で実行） |
| `mise run test-ui-scenario-contract` | pass (exit 0 — fake 17/17 + gpui 7/7) |
| `mise run check-ui-driver-boundary` | pass (exit 0 — dep-graph/signature/allowlist 全 pass) |
| `mise exec -- prek run --all-files` | pass (exit 0) |

## Real lane verdicts（`evidence/rf631/plugin-dock-real-scenario.jsonl` @ `ff7013dc`）

| case | verdict | 内容 |
| --- | --- | --- |
| `real-window-ime` | **pass** | `Ime` capability claim（capabilities 行に `ime` 出現）→ docked `test_editor` child HWND attach → `ime_preflight` pass（`ImmIsIME`/`ImmGetContext`/`ImmGetDefaultIMEWnd` 全 effective）→ `ime_drive` pass（composition `かな` + `CPS_COMPLETE` accepted — acceptance≠delivery）→ `ime_delivery` pass（caller-side subclass の `WM_IME_*` log が commit sequence を観測）→ session `Attached` 維持 → `[Attached{Docked}, Closed]` + close 収束を実測。`thread_affinity` step は documented D1 deviation の `note` のまま |
| `real-dcomp-bandescape` / `real-dcomp-childinv` | fail | 既知 class C（Issue #532 系・対象外）— `real-dcomp-pixel` は本 run では pass |
| `real-session-crash` / `real-session-hang` / `real-session-rt-boundary` | unsupported | honest boundary row（in-process lane の構造限界・本 slice 対象外） |

その他の real row 27 case は全て `pass` — `05f7450f` 時点の
分布（27 pass / 2 fail / 3 unsupported）と同一で、#907 着地後の
回帰なし。

## Evidence

- `evidence/rf631/plugin-dock-real-scenario.jsonl`（canonical —
  `ff7013dc` で interactive 再実走）
- `evidence/rf631/plugin-dock-real-headless.jsonl`（blocked-probe
  fail-closed proof — 再実走）
- `evidence/rf631/plugin-dock-scenario.jsonl` /
  `plugin-dock-console.jsonl` / `plugin-dock-console-list.jsonl`
  （headless lane 再生成。`plugin-dock-console-list.jsonl` の
  diff は JSON key 順のみ — 補助証拠、canonical = mise）
- `evidence/rf389/plugin-dock-real-scenario.jsonl` /
  `plugin-dock-real-headless.jsonl`（本 slice snapshot コピー —
  旧 `62d595b6`/`b6d0cc6a` 世代から `ff7013dc` へ刷新）
