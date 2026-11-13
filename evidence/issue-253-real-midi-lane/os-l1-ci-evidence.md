# Issue #253 — OS-L1 既存CI証拠照合・Backend能力/未実装の正直な記録

- Issue: https://github.com/scarecrow173/ruviform/issues/253
- 対象 SHA（タスク指定）: `b9eaf4880681734b2d65dbf200ca8c49c6e4c81e`（main HEAD・PR #602 merge 時点）
- 記録時点の main HEAD: `05c7fc3bff54fbf73264408dc5a33dd4bff8f9d8`（PR #603 merge・Issue #306 evidence slice）
- 収集日: 2026-11-13
- 作業 branch: `docs/253-midi-os-l1-evidence`
- 本 slice の範囲: Issue #253 の OS-L1 受入条件のうち「既存 CI の Windows/macOS/Linux での MIDI 関連コード Build/Link・Mock/Headless の Backend 変換・Timing/Error/RT 回帰の対象 SHA 照合」「未実装/未提供能力の正確な記録」「実測済みと NotRun/Unsupported の OS×Backend×Device×Case 一覧の確定」に限定。実 MIDI device・実 OS 権限・実 Plugin/DAW 往復（L3 相当）と macOS/Linux 実 backend 実装は対象外で Issue は open 継続。
- 文書チェーン: 証拠記録のみの slice のため spec/design/plan/task の完全チェーンは生成せず、正本は `docs/impl/ir/issue-253-midi-os-l1-evidence.md` に集約。

## 検証スコープと Oracle（受入条件「能力別判定」対応）

| 項目 | 内容 |
| --- | --- |
| OS | `windows-latest` / `ubuntu-latest` / `macos-latest`（GitHub hosted runner） |
| 採用 Feature | workspace default（`lint-full`/`test-full` は `--all-features` なし）+ `host`（`l1-real-verify` 系の fixture cdylib + L1 runner bin + `--lib --bins` test）。MIDI 関連コードに cargo feature gate はなく、`ruviform-midi-device` / `ruviform-audio-plugin-bridge` / `ruviform-media-core` / `ruviform-audio-core` の MIDI 経路は default feature で compile・test される。`ruviform-plugin-vst3` / `ruviform-plugin-clap` の `host` feature は host 側 bridge（`src/host.rs`、IMidiMapping snapshot・routing を含む）を有効化し `l1-real-verify` 3 lane で実行される |
| Device/Format | MIDI は repo 内 fixture・MockBackend・WinMM API のみ。実 MIDI input ポート・実商用 Plugin/DAW は対象外（Windows で利用する機能の L3 は別途 Required だが本 slice 対象外） |
| Run URL / Oracle | 下記 2 run。Oracle = 各 job/step の success、cargo test の `test result` 集計行・個別 test 行（`... ok` / `... ignored`）、`checkout-sha-*` artifact の実測 SHA、`ci-summary` の fail-closed verdict |
| 新規 CI Workflow | 作成していない（既存 `ci.yml` の dispatch run のみを証拠とする） |

## 証拠 run と SHA 照合

### 主証拠: Full CI run [43428172820](https://github.com/scarecrow173/ruviform/actions/runs/43428172820)（SHA `e93c438a`）

- `ci.yml` workflow_dispatch、2026-11-13T12:47:36Z 開始、conclusion=`success`、全 20 job（19 lane + `ci-summary` verdict=PASS）success。run の `headSha` = `e93c438adc4af5b498e10926fe859ac1338ac390`。
- MIDI 関連 7 leg（`verify`×3 OS・`rt-verify`・`l1-real-verify`×3）の `checkout-sha-*-attempt-1` artifact を本 slice で再取得し、実測 `git rev-parse HEAD` が全て `e93c438a…` で一致することを確認（全 19 leg の照合は `evidence/rf388-os-l1-ci/report.md` でも実施済み）。
- **対象 SHA との差分検証（Coordinator 裁定に基づく採用根拠）**: タスク指定 SHA `b9eaf488` に対応する completed CI run は存在しない（`gh run list --commit b9eaf488` = 0 件、in_progress/queued もなし）。Coordinator 判断（orchestration `msg_482da7ea856e`）により、本 run を MIDI capability matrix の OS-L1 証拠として採用し、以下を実測で確認した:
  - `git diff --name-only e93c438a..b9eaf488` → 11 files（`.github/workflows/ci.yml` の actionlint/pre-commit 配線、`.pre-commit-config.yaml`、`crates/ruviform-ui-test` の stall/timeout test、`docs/`・`evidence/` の #313/#388 記録）— **MIDI・plugin・audio 系 path への変更 0 件**。
  - `git diff --name-only b9eaf488..05c7fc3b` → 15 files（`crates/ruviform-io-e2e` test/README、`docs/` の #306 文書チェーン、`evidence/rf306/`）— **MIDI・plugin・audio 系 path への変更 0 件**。
  - 合算 `git diff --name-only e93c438a..05c7fc3b` → 25 files、MIDI 影響 path は 0 件（`midi|plugin|vst|clap|audio-core|media-core|audio-plugin|ruviform-audio|ruviform-graph|ruviform-rt` に一致する path なし）。
  - したがって MIDI 関連 crate の Build/Link・test 結果は `e93c438a` run の実測を `b9eaf488`・`05c7fc3b` へ読み替える根拠が成立する。ただしこれは「同一 SHA の CI 証拠」ではなく「差分検証済みの直前 SHA 証拠」であり、判定上はその旨を明記する。

### 補足証拠: Full CI run [43436082797](https://github.com/scarecrow173/ruviform/actions/runs/43436082797)（SHA `05c7fc3b`、main HEAD）

- `ci.yml` workflow_dispatch、2026-11-13T13:43:04Z 開始、**conclusion=`failure`（completed）**。job 別結果:
  - MIDI 関連 lane は全て success — `verify (windows-latest)`・`verify-macos`（log で MIDI test 行 `ok`/`ignored` を確認）・`rt-verify`（strict `rt_` = **166**、`rt_midi_delivery_collect_and_merge_no_alloc` / `rt_midi_wiring_no_alloc` `ok`）・`l1-real-verify`×3。
  - `verify (ubuntu-latest)`: **failure** — `prek run --all-files` の `actionlint` hook が `.github/workflows/consumer-verify.yml`・`ci.yml` の shellcheck style 指摘（SC2129/SC2155、PR #602 で追加された hook）で exit 1。**cargo 系 step（fmt/lint/test）未到達のため、05c7fc3b での Linux `test-full`（MIDI 含む workspace test）は NotRun**。MIDI コードとは無関係の workflow 設定衛生失敗であり、本 slice の編集範囲外。
  - `ci-summary`: failure（fail-closed verdict が ubuntu verify failure を反映）。その他 16 job（`ui-*`/`ui-gpui-*` 含む）は全て success。
- この run の結果は主証拠 `43428172820` を置き換えない。`verify (ubuntu-latest)` の failure は `.github/`・`.pre-commit-config.yaml` の修正が必要であり本 slice 範囲外。Linux `test-full` の 05c7fc3b 同一 SHA 証拠は未取得のまま（`e93c438a` 実測 + 差分検証で補完）。

## MIDI 関連 lane × OS カバレッジ（証拠 run `43428172820` @ `e93c438a`）

| lane（job） | 実行内容 | OS | MIDI 関連の観測 |
| --- | --- | --- | --- |
| `verify`（job [131894691153](https://github.com/scarecrow173/ruviform/actions/runs/43428172820/job/131894691153) ubuntu / [131894691209](https://github.com/scarecrow173/ruviform/actions/runs/43428172820/job/131894691209) windows / [131894691243](https://github.com/scarecrow173/ruviform/actions/runs/43428172820/job/131894691243) macos） | `prek --all-files` → `fmt-check` → `lint-full`（`cargo clippy --workspace --all-targets -D warnings`）→ `test-full`（`cargo test --workspace`、default feature） | 3 OS | 全 step success。下記「crate 別 test 集計」参照 |
| `rt-verify`（job [131894691112](https://github.com/scarecrow173/ruviform/actions/runs/43428172820/job/131894691112) ubuntu） | `cargo test --workspace --locked rt_ -- --list` で strict `rt_` = **166** を計測後 `mise run test-rt` | ubuntu | success。`rt_midi_delivery_collect_and_merge_no_alloc ... ok`、`rt_midi_wiring_no_alloc ... ok`（log 確認） |
| `l1-real-verify`（job [131894691202](https://github.com/scarecrow173/ruviform/actions/runs/43428172820/job/131894691202) ubuntu / [131894691699](https://github.com/scarecrow173/ruviform/actions/runs/43428172820/job/131894691699) windows / [131894690956](https://github.com/scarecrow173/ruviform/actions/runs/43428172820/job/131894690956) macos） | `cargo build -p ruviform-plugin-clap -p ruviform-plugin-vst3 --features host` → `cargo test --features host --lib --bins` → `cargo test --features host --test l1_real_adapter_e2e -- --ignored` + `lint-l1-real-plugin` | 3 OS | 全 step success。`--features host` で clap lib **65** + runner bin **2**、vst3 lib **74** + runner bin **2** passed。`host::tests::imidi_mapping_routes_volume_cc_to_gain_param` / `unmapped_cc_passes_through_without_param_routing` / `midi_events_reach_process_as_input_events` と `midi::tests::*` 全件 `ok`（3 OS log 確認）。ignored E2E は各 crate **3 passed** — audio processing のみで **MIDI coverage 0**（`l1_real_adapter_e2e.rs` に midi 参照なし・grep 確認） |
| `msrv-verify`（job [131894691257](https://github.com/scarecrow173/ruviform/actions/runs/43428172820/job/131894691257) ubuntu, rust@1.89.0） | `test-ofx-msrv`（plugin-core/ofx）・`test-ui-test-msrv`・`test-observability-msrv`・`test-io-msrv` | ubuntu | success だが **MIDI 関連 crate（midi-device / audio-plugin-bridge / plugin-vst3 / plugin-clap / audio-core / media-core）は MSRV lane の対象外 → MSRV での MIDI crate 検証は NotRun**（lane 設計上の範囲。default/stable toolchain での compile・lint・test は verify lane が担保） |
| `consumer-verify.yml` | Consumer tuple gate（workflow_dispatch 専用） | — | 本 SHA 系の run なし（最新は `43347745016` @ `3692c6fa`）→ **NotRun**（本 slice 必須範囲外・記録のみ） |
| `test-midi-real-device`（mise task、CI 非搭載の manual lane） | `cargo test -p ruviform-midi-device -p ruviform-audio-plugin-bridge -- --ignored --test-threads=1` | Windows のみ実施記録 | 本 dir `README.md` の既存記録どおり: evidence 環境に MIDI input ポートなし → 3 ignored test が `no MIDI input ports — nothing to verify` で honest return・exit 0。**実デバイス観測は NotRun**（Pass に偽装していない） |

### crate 別 test 集計（`test-full` log 実測・`e93c438a`）

`cargo test --workspace` は全 target（lib unittests・tests/・bins・Doc-tests）を build・link して実行する。以下は MIDI 関連 crate の `test result` 行（log の interleave により binary 帰属が一意でない箇所は test 名で裏付けた範囲で記載）。

| crate | Windows | Ubuntu | macOS | 内容 |
| --- | --- | --- | --- | --- |
| `ruviform-midi-device` lib | 17 passed, 1 ignored | 1 passed | 1 passed | Windows のみ `midi::tests::decodes_*`（channel-voice decoder 7 件）・`os::windows::tests::*`（enumeration honesty・stamp provenance・port key 解決）・`queue::tests::*`（bounded queue・dropped/unsupported 計数・StreamInvalidated）を compile。非 Windows は `os::unsupported::tests::enumeration_is_explicitly_unsupported` のみ。ignored = `real_device_open_read_close_smoke`（Windows のみ存在） |
| `ruviform-midi-device` tests/ports.rs | 2 passed, 1 ignored | 1 passed, 1 ignored | 1 passed, 1 ignored | Windows: `enumerator_produces_ports_with_honest_declarations`（`supports_ump=false`/`provides_timestamps=true`/`origin=Unknown`/per-port domain の正直な宣言）+ `read_contract_shape_holds_for_trait_objects`。非 Windows: `unsupported_platforms_return_explicit_unsupported`（`Err(DeviceError::Unsupported)` を assert）。ignored = `real_device_open_read_observe_close`（全 OS） |
| `ruviform-audio-plugin-bridge` lib | 4 passed | 4 passed | 4 passed | bridge 内部 unit test |
| `ruviform-audio-plugin-bridge` tests/midi_delivery.rs | 6 passed | 6 passed | 6 passed | `midi_delivery_note_cc_bend_with_frame_offsets`・`block_boundary_and_release_all`・`rejections_are_observable`・`unsupported_is_observable`・`zero_frame_block`・`rt_midi_delivery_collect_and_merge_no_alloc`（frame offset・順序・loss/unsupported の observable error・RT no-alloc） |
| `ruviform-audio-plugin-bridge` tests/midi_wiring.rs | 15 passed, 1 ignored | 15 passed, 1 ignored | 15 passed, 1 ignored | `plan_midi_replays_at_scheduled_offsets`・`loop_refires_midi_events_per_lap`・`rt_midi_wiring_no_alloc` 他。ignored = `real_midi_device_drives_input_tx_to_stage`（`SystemBackend`→`InputTx::pump`→`InputStage` の実機経路） |
| `ruviform-plugin-vst3` lib（default feature） | 41 passed | 41 passed | 41 passed | `midi::tests::*`: `incoming_*`（VST3 event→neutral 変換）・`outgoing_*`・`fill_event_list_*`（offset 保存・IMidiMapping suppress）・`ppq_position`・`control_assignment`/`routed_assignment`/`assignment_table_capture`（IMidiMapping table）。**plugin 側 MIDI 翻訳は default feature で 3 OS 実走** |
| `ruviform-plugin-vst3` lib（`--features host`、l1-real-verify） | 74 passed | 74 passed | 74 passed | 上記に加え `host::tests::*`（host bridge）。`imidi_mapping_routes_volume_cc_to_gain_param` 等の IMidiMapping CC→Parameter routing を含む。runner bin 各 2 passed |
| `ruviform-plugin-clap` lib（default feature） | 48 passed | 48 passed | 48 passed | `adapt::tests::translate_events_translates_midi_channel_voice` 他 CLAP event↔neutral 変換（`adapt.rs` の MIDI 1.0 channel-voice decode） |
| `ruviform-plugin-clap` lib（`--features host`、l1-real-verify） | 65 passed | 65 passed | 65 passed | 上記に加え `host::tests::*`。runner bin 各 2 passed |
| `ruviform-audio-core` lib | 56 passed | （集計は log 参照） | （集計は log 参照） | `midi_input::mock::tests::*`（MockBackend）・`midi_input::tests::*`（port contract・bounded read・dropped 計数）・`wire::tests::midi_event_wire_roundtrip_and_rejections` |
| `ruviform-media-core` lib | （集計は log 参照） | 同左 | 同左 | `midi::tests::*`: `ledger_*`（note state ledger・eviction・release_all）・`rt_ledger_no_alloc`・`midi_value_scale_to_max_roundtrips`・`validated_newtypes_reject_out_of_range`・`effective_note_state_resolves_velocity_zero` — typed timestamp/order/loss の model 層 |

- Doc-tests も全 crate で `0 failed`（`ruviform-midi-device` doctest は `backend()` の enumerate 例で non-Windows では `unwrap_or_default` 経路、Windows では実 enumerate）。
- `test-full` は ignored test を実行しない — `real_device_open_read_observe_close` / `real_device_open_read_close_smoke` / `real_midi_device_drives_input_tx_to_stage` は全 OS で `ignored, requires ... manual opt-in check` として列挙されるのみ（**CI での実機 Pass ではない**）。

## OS × Backend × Device × Case 判定一覧

判定語彙は `POLICIES.md` §3。`Unsupported` は公開契約どおりの明示的非対応（`DeviceError::Unsupported`）、`NotRun` は未実行（環境・lane 不在）で Pass/Unsupported のいずれでもない。

| OS | Backend | Device/Case | 判定 | 証拠 |
| --- | --- | --- | --- | --- |
| Windows | WinMM（`ruviform-midi-device`、唯一の実装 backend） | Build/Link + unit/integration test（decoder・queue loss・stamp provenance・honest capability 宣言・Unsupported 境界の compile 側） | **Pass** | run 43428172820 `verify (windows-latest)` job success + 上記 test 集計。05c7fc3b `verify (windows-latest)` でも同 test 行 `ok`/`ignored` を確認 |
| Windows | WinMM | 実 MIDI input ポートでの enumerate→open→read/pump→close・`dropped`/`unsupported` 実測・`StreamInvalidated`（L3 相当） | **NotRun**（Deferred: Issue の Windows L2/L3 条件付き必須。実ポートを持つ環境での `mise run test-midi-real-device` 実測が必要） | evidence 環境はポートなしで honest skip（`README.md`）。CI の 3 ignored test は `ignored` 表示のみで実行されない |
| Windows | WinMM | port 非存在環境での enumerate→empty/honest 動作 | **Pass**（CI windows runner で `input_ports()` が `Ok` を返すことを `enumerator_produces_ports_with_honest_declarations` が実走確認） | 同上 job log |
| macOS | backend 未実装 → `DeviceError::Unsupported` 明示境界 | `SystemBackend::input_ports()` が `Err(Unsupported)`・consumer が compile して正直に失敗する契約 | **Pass**（Unsupported 契約の実装+test として）/ 実 port 能力としては **Unsupported** | run 43428172820 `verify-macos` で `os::unsupported::tests::enumeration_is_explicitly_unsupported`・`unsupported_platforms_return_explicit_unsupported` `ok`。05c7fc3b `verify-macos` でも同 `ok` |
| macOS | CoreMIDI 等の実 backend | 実装なし（`src/os/unsupported.rs` が全 `cfg(not(windows))` を覆う。CoreMIDI/ALSA/JACK/PipeWire は deferred・`docs/designs/issue-253-os-midi-backend.md`） | **Unsupported**（技術的非対応として明示。実装は将来 slice） | 同上 + コード `#[cfg(not(windows))]` 構造 |
| Linux | backend 未実装 → `DeviceError::Unsupported` 明示境界 | 同上契約 | **Pass**（契約 test）/ 実 port 能力は **Unsupported** | run 43428172820 `verify (ubuntu-latest)` で同 test `ok`。**05c7fc3b では `verify (ubuntu-latest)` が prek/actionlint で cargo 未到達 failure のため Linux test-full は NotRun**（e93c438a 証拠 + 差分検証で補完） |
| Linux | ALSA/JACK/PipeWire 等の実 backend | 実装なし | **Unsupported**（同上） | 同上 |
| 全 OS | VST3 MIDI 翻訳（incoming/outgoing/fill_event_list/IMidiMapping capture）+ CLAP `adapt` MIDI 変換 + bridge delivery/wiring | Mock/Headless（fixture・MockBackend・in-process） | **Pass** | verify×3 OS（default feature）+ l1-real-verify×3 OS（`host` feature で `host::tests::imidi_mapping_*` 含む） |
| 全 OS | VST3/CLAP host bridge の MIDI 経路 | `host` feature opt-in | **Pass**（feature 実行範囲: `l1-real-verify` 3 lane の `--features host` build + `--lib --bins` test。`lint-l1-real-plugin` の clippy `-D warnings` も success） | run 43428172820 l1-real-verify×3 job log（clap 65+2 / vst3 74+2 passed）。05c7fc3b でも l1-real-verify×3 success |
| 全 OS | 実 Plugin/DAW での MIDI mapping・output reports 往復（L0/L1 実 adapter） | `l1_real_adapter_e2e` は audio processing のみで MIDI coverage 0。実 Plugin 受入は Synvelis #126/#127 が Owner | **NotRun / Deferred** | `l1_real_adapter_e2e.rs` に midi 参照なし（grep 確認）。Issue #253 残件どおり |
| 全 OS | MIDI 2.0 / UMP / MPE | `supports_ump=false` を capability として正直に宣言。MIDI2/UMP/MPE は future Issue #317 | **NotApplicable（現行契約）/ Deferred（#317）** | `MidiPortCapabilities::supports_ump`・`ports.rs` の honest declaration assert |
| 全 OS | 実 MIDI device → native timestamp → Host/Plugin・loss/disconnect/RT 実測（L3） | opt-in lane `test-midi-real-device` は manual・CI 非搭載。実ポート環境での実測が必要 | **Deferred / NotRun** | `mise.toml` task 定義（manual lane, never CI）+ `README.md` 実施記録（ポートなし honest skip） |
| Windows/macOS/Linux | MSRV（rust@1.89.0）での MIDI crate 検証 | `msrv-verify` の対象 crate set に MIDI 関連 crate なし | **NotRun**（lane 範囲外。stable toolchain の verify lane で compile/lint/test は担保） | `msrv-verify` job log の task 一覧 |

## Feature opt-in 実行範囲の明示

- **default feature（`cargo test --workspace` / `clippy --workspace --all-targets`）**: `ruviform-midi-device`・`ruviform-audio-plugin-bridge`・`ruviform-media-core`・`ruviform-audio-core` の全 MIDI 経路、`ruviform-plugin-vst3`/`ruviform-plugin-clap` の plugin 側 MIDI 翻訳（`mod midi`/`mod adapt` は feature gate なし）を 3 OS で compile・lint・test。
- **`host` feature（opt-in）**: `ruviform-plugin-vst3`/`ruviform-plugin-clap` の `src/host.rs`（host 側 bridge・`IMidiMapping` snapshot/routing の `host::tests::*`）は `#[cfg(feature = "host")]`。実行範囲は `l1-real-verify`（ubuntu/windows）+ `l1-real-verify-macos` の `cargo build --features host`・`cargo test --features host --lib --bins`・`lint-l1-real-plugin`（clippy `-D warnings`）のみ。default `cargo test --workspace` では compile されない。
- **`test-midi-real-device`（mise task、opt-in manual lane・CI 非搭載）**: `#[ignore]` E2E 3 件（`real_device_open_read_observe_close`・`real_device_open_read_close_smoke`・`real_midi_device_drives_input_tx_to_stage`）。実 MIDI input ポートが必要で、ポートなし環境では `no MIDI input ports — nothing to verify` / `OS MIDI backend unavailable` を記録して honest return。証拠環境（Windows 11・ポートなし）での実行記録は `README.md`。
- **`real-device` feature**: `ruviform-audio-device` 専用（`test-audio-real-input` lane）であり MIDI とは無関係 — 混同しない。

## run 43436082797 の最終状態（SHA `05c7fc3b`、PR 作成時点で再確認済み）

- `gh run view 43436082797` → status=`completed` / conclusion=`failure`。job 別: success 18 件、failure 2 件（`verify (ubuntu-latest)` = prek `actionlint` hook の shellcheck 指摘で cargo 未到達、`ci-summary` = fail-closed verdict）。MIDI 関連 lane（`verify (windows-latest)`/`verify-macos`/`rt-verify`/`l1-real-verify`×3）は全て success で、log 上も MIDI test 行 `ok`/`ignored` を確認。
- `verify (ubuntu-latest)` の failure は PR #602 で追加された pre-commit `actionlint` hook が `consumer-verify.yml`/`ci.yml` の shellcheck style（SC2129/SC2155）を検出したもので、MIDI コードとは無関係。修正には `.github/`・`.pre-commit-config.yaml` の変更が必要であり本 slice の編集範囲外。
- この failure は main の CI 破損を意味する。05c7fc3b 以降の merge SHA での統合検証は、`verify (ubuntu-latest)` が修復されるまで Linux `test-full`（MIDI 含む）が NotRun のままになる可能性がある。e93c438a run + MIDI 影響 path 0 件の差分検証が現時点の Linux 証拠の補完である。

## 判定まとめ

- **OS-L1（本 slice の必須範囲）: 証拠は揃ったが SHA 厳密性に注意。** 対象 SHA `b9eaf488` の completed CI run は存在せず、Coordinator 裁定で `e93c438a` run `43428172820`（全 20 job success）を採用。`e93c438a..05c7fc3b` の差分に MIDI 影響 path が 0 件であることを `git diff` で検証済み。補足として `05c7fc3b` run `43436082797` の完了済み job（Windows/macOS verify・rt-verify・l1-real×3）でも同一の MIDI test 結果を確認。Linux `test-full` は 05c7fc3b で prek 失敗により NotRun、e93c438a 実測 + 差分検証で補完。
- **能力判定**: Windows WinMM backend の Build/Link・Mock/Headless 変換・Timing/Error/RT は 3 OS の既存 CI で Pass。macOS/Linux の実 backend は `DeviceError::Unsupported` の明示境界として Pass（契約 test）/ 実 port 能力は Unsupported。実 MIDI device・実 Plugin/DAW 往復・MSRV での MIDI crate は NotRun/Deferred であり Pass に繰り上げていない。
- **Issue #253 は open 継続。** 残る阻害要因は実 MIDI ポートでの L3 実測（Windows 利用機能として Required・ポート保有環境での `test-midi-real-device`）、macOS/Linux 実 backend 実装の要否判断、実 Plugin/DAW 受入（Synvelis #126/#127）であり、OS-L1 証拠の欠落ではない。
