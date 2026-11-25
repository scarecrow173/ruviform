---
id: "SPEC-034Yo3qBLOF8ei9OW6Nx7M"
type: "spec"
status: "approved"
title: "Issue #253 virtual-first OS-endpoint MIDI e2e verifier"
created: "2026-11-16"
updated: "2026-11-16"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/253"
    - "POLICIES.md"
    - "docs/specs/issue-629-midi-loopback.md"
    - "docs/specs/issue-629-midi-output.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-253-virtual-midi-e2e.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/designs/issue-253-virtual-midi-e2e.md"
    - "docs/plans/issue-253-virtual-midi-e2e.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-253-virtual-midi-e2e.md"
    - "docs/plans/issue-253-virtual-midi-e2e.md"
    - "docs/test-specs/issue-253-virtual-midi-e2e.md"
    - "docs/tasks/issue-253-virtual-midi-e2e.md"
    - "docs/specs/issue-629-midi-loopback.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-629-midi-loopback.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-253-virtual-midi-e2e.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #253 virtual-first OS-endpoint MIDI e2e verifier

## What Should Be Built

Issue #253 の残存受入（AC1 Virtual-first 移行 row と AC6 Windows
GUI-L2/EXT-L3 row）の合成経路 verifier を 1 本実装する。
`crates/ruviform-plugin-test-vst3/tests/os_endpoint_midi_e2e.rs`
（新設・`#[ignore]`-gated・手動 opt-in）として、以下の実鎖を
headless に駆動する `#[ignore]` fail-closed verifier:

```text
real WinMM backend（`ruviform_midi_device::backend()` —
  Windows では `SystemBackend`、非 Windows では
  `DeviceError::Unsupported`）
  → bidirectional name-match で特定した loopback-style port pair
    （candidate であり virtual endpoint の証明ではない）
  → `InputTx::pump(&mut dyn MidiInput)` → node `InputStage`
  → `attach()`/`AttachConfig::input` → `PluginNode`
  → typed `GraphRuntime`（Source→Processor→Sink）
  → out-of-process real-ABI VST3 `test_instrument` fixture
    （`l1-runner` feature の sibling runner binary が dlopen）
  → drained sink PCM + `InputStage`/`InputTx` counters
    （独立 oracle で照合）
```

構成は `crates/ruviform-midi-device/tests/loopback.rs`（列挙 →
pair 特定 → open → 固定イベント列送信 → drain → 独立期待値照合 →
verdict）と `tests/l1_real_test_plugin_e2e.rs`（fixture spawn →
`attach` → typed graph → L1 pacing → PCM oracle）の合成。

## Why Now

- Issue #253 の AC1/AC6 は「実 OS endpoint → attach → 実
  `test_instrument`」の合成 e2e を要求するが、構成部品
  （`SystemBackend` input/output・loopback pair verifier・
  `InputTx::pump`・`attach()` の input leg・L1 runner/fixture）は
  全て merge 済みで、両者を繋ぐ verifier だけが未実装だった。
- 仮想優先 POLICIES（2026-11-15 改訂）では EXT-L3 は仮想 OS
  endpoint と real-ABI test plugin で検証する。本 slice はその
  verifier を provisioning 確立前に用意し、pair 未 provision 環境
  では正直な `NotRun`/`NOT_PROVISIONED` で fail-closed に終わる
  形にする。
- 旧物理 MIDI port 3 ignored case は歴史的 `NotRun` であり、
  仮想 port の新 Case を未実走のまま Pass 化してはならない。

## Users and Value

- **後続 slice（provisioning 確立後）**: loopMIDI/Windows MIDI
  Services の仮想 loopback pair を provision した環境で本 test を
  `--ignored` 実行するだけで、実 WinMM backend から attach・
  plugin・L1 oracle までの合成鎖の証拠を取得できる。
- **Ruviform 自身**: `InputTx::pump` → `InputStage` → `PluginNode`
  → `GraphRuntime` → out-of-process VST3 という本番 input 経路を、
  mock ではなく実 OS backend と実 ABI fixture で dogfooding する
  最初の e2e。
- **レビュー/受入**: fail-closed verdict（`NotRun`/`Fail`/`Pass`）
  と独立 oracle により、環境未整備を Pass と誤認しない境界が
  コードとして固定される。

## Scope

### In Scope

- `crates/ruviform-plugin-test-vst3/tests/os_endpoint_midi_e2e.rs`
  の新設（`#![cfg(feature = "l1-runner")]` + `#[ignore]`）。
- `crates/ruviform-plugin-test-vst3/Cargo.toml` への workspace 内
  dev-dep 追加（`ruviform-midi-device` path dep — Coordinator
  承認済み。`ruviform-audio-device` は本実装では不要と判断）。
- `Cargo.lock` の当該 dev-dep 追加に伴う再生成差分。
- docs chain（spec/design/plan/task/test-spec/impl ir・exp）と
  `evidence/rf253/` の検証記録。

### Out of Scope

- loopMIDI/Windows MIDI Services の実 provisioning と
  `virtual-os-endpoint` evidence の取得（repo 外・後続 slice）。
- 仮想 endpoint 経由の reconnect/loss/permission/xrun 再現
  （#629/#630 側受入・`scarecrow173/ruviform#628` scripted 側が
  決定的 event oracle を既担）。
- Test Runtime（#628）との結線・consumer oracle 照合
  （`scarecrow173/synvelis#126`/`#127`）。
- `mise.toml` への新 verifier lane 登録（別 Worker 占有 — 本 slice
  では行わず、canonical 検証は `check-l1-real-plugin`/
  `lint-l1-real-plugin` の `--all-targets` compile/lint まで）。
- macOS/Linux MIDI backend 実装・実 DAW 往復・
  `param_reporting` の Verified 昇格。
- 物理機種固有特性の保証（未保証と記録する）。
- `crates/ruviform-plugin-test-vst3/src/`（別 Worker が
  `events.rs` を修正中 — zero-diff）、製品 crate の src、
  `l1_real_test_plugin_e2e.rs` / `midi_wiring.rs`（共有 helper の
  拡張が不要だったため未変更）。

## Requirements

- verifier は `ruviform_midi_device::backend()` の production
  port API のみを使い、Win32 FFI・mock backend を追加しない。
- input/output 両方向を列挙し、`info().name` の双方向一致で
  loopback-style pair を特定する（port id は方向で体系が異なる
  ため name が唯一の正直な鍵 — `loopback.rs` と同じ規則）。
- backend 不在・`Unsupported`・列挙空・pair 0 件・timestamp 非
  対応 port では `verdict: NotRun` を出力して early return し、
  assert しない。Skip=Pass にしない。
- fixture artifact（本 crate cdylib + `l1-runner` runner binary）
  の欠落は `assert!` 失敗 — 正直な `NotRun` と区別する。
- pair が open した後の送信・配送・plugin・runtime・graph・PCM/
  oracle のいずれかの失敗は `verdict: Fail`（panic）とする。
- 期待列は独立した固定リテラル（channel 3 の NoteOn/CC/
  PitchBend/NoteOff、値は `from_u7`/`bend_from_u14` で wire 幅から
  構築）であり、被試験経路が期待値を生成しない。
- stamped-offset oracle: 観測した実 `TimedMidiInput` の `NoteOn`
  stamp に `ClockAnchor`（`Estimated` provenance）を較正し、
  fixture の決定的 triangle attack が `ANCHOR_FRAME` の
  intra-block offset に着地すること、release が stamped `NoteOff`
  + release 長の範囲で完了すること、dual-mono 出力であることを
  照合する。
- counter oracle: `InputStage` snapshot（`late`/`anchor_rejected`/
  `dropped_capacity`/`mapped`/`delivered`/`rejected`/
  `dropped_backend`/`unsupported_backend`）と `InputTx` の
  backend 透過 counter を、pump 観測値と独立期待値に照合する。
- close 後 `StreamInvalidated` を input/output 両方向で確認する
  （`loopback.rs` と同じ契約）。
- `MidiPortOrigin::Unknown` や name match のみを根拠に
  `environment_kind=virtual-os-endpoint` を主張しない — verdict
  出力にその旨を明記する。
- 非 Windows では backend が `DeviceError::Unsupported` を返す
  ため `NotRun` で fail-closed に振る舞い、3OS lane の green を
  維持する（`#[cfg]` 分岐ではなく runtime 判定）。
- 実行していない検証を Pass と書かない。evidence は Case schema
  語彙で正直に記録する。

## Acceptance Criteria

- [ ] `tests/os_endpoint_midi_e2e.rs` が `l1-runner` feature で
  compile し、`cargo test --features l1-runner --test
  os_endpoint_midi_e2e -- --ignored` で収集される。
- [ ] provisioned loopback-style pair が無い環境で
  `verdict: NotRun — NOT_PROVISIONED`（または同等の正直な
  `NotRun`）を記録して終了し、Pass を主張しない。
- [ ] pair 存在時に NoteOn/CC/PitchBend/NoteOff が実 WinMM backend
  → `InputTx::pump` → `InputStage` → `PluginNode` → `GraphRuntime`
  → out-of-process `test_instrument` を通過し、stamped-offset
  attack・release tail・dual-mono・counter oracle を全て満たす。
- [ ] fixture artifact 欠落時は `assert!` 失敗（skip ではない）。
- [ ] close 後 `StreamInvalidated` が両方向で確認される。
- [ ] `environment_kind=virtual-os-endpoint` の Pass を name match
  のみを根拠に主張する経路が存在しない。
- [ ] 本 slice が対象とするのは AC1/AC6 の verifier 実装まで —
  Issue #253 の受入チェックボックスは close しない（Issue は
  open のまま、PR は `Refs #253`）。

## Deferred Design Concerns

- loopMIDI/Windows MIDI Services の実 provisioning と
  `virtual-os-endpoint` Pass evidence — 後続 slice（repo 外
  provisioning が前提）。再開条件: provisioned pair の確立。
- 仮想 endpoint 経由の reconnect/loss/permission/xrun 再現・
  Test Runtime（#628）結線・consumer oracle 照合 — #629/#630 側
  受入・別 Issue。
- `mise.toml` への本 verifier 収集 lane 登録 — 別 Worker 占有の
  ため本 slice 対象外。再開条件: Coordinator による lane 統合。
- 物理機種固有特性 — 未保証（POLICIES の virtual-first 方針どおり
  保証対象外）。

## Sources and Relations

- `scarecrow173/ruviform#253`（本 Issue、open のまま — PR は
  `Refs`）、`scarecrow173/ruviform#629`（仮想 endpoint survey・
  loopback verifier・output port）、`scarecrow173/ruviform#628`
  （決定적 event oracle / Test Runtime）、`scarecrow173/
  ruviform#627`（Closed — Case Schema・`environment_kind` 語彙）。
- `docs/specs/issue-629-midi-loopback.md`（fail-closed verifier・
  name-match pair 特定・timestamp 観測の先例）、
  `crates/ruviform-midi-device/tests/loopback.rs`（pair 選択・
  close semantics の正本実装）。
- `crates/ruviform-plugin-test-vst3/tests/l1_real_test_plugin_e2e.rs`
  （fixture descriptor・runner 解決・`attach` rig・typed graph・
  L1 pacing・PCM oracle の先例）。
- `POLICIES.md` §2.1（仮想優先の受入・証拠要件）・§5（ローカル
  検証と Daily CI の責務分離）。

## 追記（2026-11-23）— canonical lane `test-os-endpoint-midi-e2e` 登録済み

上記 Out of Scope の「`mise.toml` への新 verifier lane 登録（別
Worker 占有 — 本 slice では行わず、canonical 検証は
`check-l1-real-plugin`/`lint-l1-real-plugin` の `--all-targets`
compile/lint まで）」および Deferred Design Concerns の
「`mise.toml` への本 verifier 収集 lane 登録 — 別 Worker 占有の
ため本 slice 対象外。再開条件: Coordinator による lane 統合」は、
lane 統合 slice で解消された。`mise.toml` に
`[tasks.test-os-endpoint-midi-e2e]` を登録し、本 verifier の
canonical 起動式は `mise run test-os-endpoint-midi-e2e`（fixture
cdylib + `l1-runner` runner bin の build → `cargo test -p
ruviform-plugin-test-vst3 --features l1-runner --test
os_endpoint_midi_e2e -- --ignored --test-threads=1`）。本 lane は
`#![cfg]` + `#[ignore]` + `required-features` で既定 `cargo test`
から分離された当該 test の唯一の canonical 実行入口であり、
provisioned pair の無いホストでは `verdict: NotRun —
NOT_PROVISIONED` で fail-closed に終わる（Pass 偽装なし）。
`check-l1-real-plugin`/`lint-l1-real-plugin` の `--all-targets`
compile/lint 収集は変わらず有効で、専用 lint lane は設けない。
provisioned pair での実走 Pass は rf#762 の self-hosted runner 調達ではなく、
ruviform 独自 test-signed driver を install する GitHub-hosted `windows-2025`
branch-ref `virtual-audio-driver-probe.yml` が gate する。rf#762 は GUI-L2 等の
別用途の残余価値はあるが、本 MIDI e2e の必須前提ではない。実行記録は
`evidence/rf253/`。

## 追記（2026-11-25）— RF#628 supply・RunMetadata・provisioned RvfMidi stage

本 vertical slice では、上記 verifier の stimulus を RF#628 の
`MidiScenario`/`run_midi_trace` 供給へ接続した。本 case は Play-at-0・
commands 空の playthrough のみを使い、`midi_trace_frames_monotonic` と
`midi_expected_playthrough`/`midi_trace_eq` で供給 trace を独立 oracle と
照合してから output port へ送る。trace frame 間隔は wall-clock pacing の
名目入力に使うが、PCM/counter oracle は引き続き観測された OS timestamp
だけを根拠にし、deterministic clock を実 OS 時刻の証明として扱わない。

Case Schema v1 `RunMetadata` を terminal verdict ごとに emit する。case id は
`rf253.midi.virtual-endpoint.host-plugin-e2e`、level は `ext-l3`、
environment kind は `virtual-os-endpoint` である。ただし name match は
loopback candidate の観測にすぎず、acceptance-grade の Pass 解釈には
`virtual-audio-driver-probe.yml` の RvfMidi provisioning provenance
（`midi-endpoint-probe` の candidates・`midi-loopback` Pass・stage detail）を
必要とする。manual/unverified run は evidence にその weaker provenance を
記録する。

`virtual-audio-driver-probe.yml` に `os-endpoint-midi-e2e` stage を追加し、
fixture build → `--nocapture` 補助実行 → verdict/RunMetadata harvest →
`os-endpoint-midi-e2e-run-metadata-<os>/` artifact copy を行う。stage `ok`
は harness の honest 完走を意味し、AC 判定は RunMetadata JSON と provenance
evidence を読んで行う。Issue #253 は close しない。

### order/loss/overflow/stop/seek/RT/disconnect/timestamp-clock oracle ownership mapping

分類語彙: `direct` は本 OS endpoint case が実測で観測するもの、`delegated` は既存の決定的 unit/integration oracle が意味論を所有し本 case が再実装しないもの、`residual` は本 slice では最終受入を閉じない残課題である。run id は Rust test 関数名または本 case の verifier 名を示す。

| concern | run id / oracle owner | 分類 | 本 case での扱い |
| --- | --- | --- | --- |
| playthrough supply | `os_endpoint_midi_loopback_drives_test_instrument` / `crates/ruviform-plugin-test-vst3/tests/os_endpoint_midi_e2e.rs` | direct | commands 空の RF#628 trace を `midi_expected_playthrough` + `midi_trace_eq` + monotonic invariant に照合する |
| order | `same_frame_events_keep_source_order` / `crates/ruviform-media-core/src/transport_schedule.rs`; `out_of_order_cue_list_still_delivers_in_stamp_order`・`channel_capacity_backpressure_defers_then_delivers_in_order` / `crates/ruviform-audio-test/tests/pipeline_midi.rs`; `os_endpoint_midi_loopback_drives_test_instrument` | delegated + direct | scheduler/pipeline の順序意味論は既存 oracle が所有し、本 case は OS 受信列が独立 literal の ordered subsequence であることだけを観測する |
| loss | `loss_counters_do_not_terminate_the_subscription` / `crates/ruviform-audio-test/tests/midi_faults.rs`; `backend_drops_and_unsupported_are_observable_per_pump_and_cumulative` / `crates/ruviform-audio-test/tests/pipeline_midi.rs`; `os_endpoint_midi_loopback_drives_test_instrument` | delegated + direct | fault 時の loss 意味論は既存 oracle が所有し、本 case は `InputTx`/`InputStage` counters を記録し、通常経路で 0 であることを観測する |
| overflow | `pending_budget_rejects_overflow` / `crates/ruviform-media-core/src/transport_schedule.rs`; `overflow_reports_are_observable`・`ledger_eviction_delivers_synthetic_off` / `crates/ruviform-audio-plugin-bridge/tests/midi_wiring.rs` | delegated | 本 case は overflow を人為注入しない。scheduler budget、ledger/chase overflow、eviction synthetic-off の決定的再現は既存 oracle が所有する |
| stop | `stop_releases_sounding_notes`・`stop_releases_live_notes_too` / `crates/ruviform-audio-plugin-bridge/tests/midi_wiring.rs`; `pending_budget_rejects_overflow` / `crates/ruviform-media-core/src/transport_schedule.rs` | delegated | OS endpoint stimulus には使わない。stop command 適用・budget 意味論は scheduler、sounding/live note release は plugin-bridge が所有する |
| seek/loop/pause | `seek_skips_intermediate_positions`・`loop_region_repeats_in_loop_events`・`pause_stops_event_delivery` / `crates/ruviform-audio-test/tests/midi.rs`; `seek_releases_and_chases_sounding_notes` / `crates/ruviform-audio-plugin-bridge/tests/midi_wiring.rs` | delegated | RF#628 deterministic trace のコマンド意味論と bridge の release/chase は既存 oracle が所有し、本 case の stimulus は Play-at-0 のみである |
| RT | `rt_run_callback_no_alloc`・`rt_device_restart_no_alloc` / `crates/ruviform-media-core/src/transport_schedule.rs`; `rt_midi_wiring_no_alloc` / `crates/ruviform-audio-plugin-bridge/tests/midi_wiring.rs` | delegated | allocation-free / RT 境界は既存 no-alloc oracle が所有する。本 OS endpoint case は wall-clock pacing を使う EXT-L3 であり、新しい RT safe 主張を追加しない |
| disconnect / close / loss of endpoint | `device_restart_vs_seek_distinct_epoch_generation` / `crates/ruviform-media-core/src/transport_schedule.rs`; `invalidate_fault_kills_stream_and_allows_reopen`・`vanish_mid_input_stream_invalidates_once` / `crates/ruviform-audio-test/tests/backend_faults.rs`; `close_makes_next_pump_fail_with_stream_invalidated` / `crates/ruviform-audio-test/tests/pipeline_midi.rs`; `assert_close_semantics` in `os_endpoint_midi_loopback_drives_test_instrument` | delegated + direct + residual | close 後の `StreamInvalidated` は本 case が直接観測する。device restart epoch と backend invalidate/vanish 意味論は既存 oracle が所有し、provisioned endpoint 消失・復帰の最終受入は #629/#1002 系 fault evidence 側の residual とする |
| timestamp-clock | `cues_arrive_at_independent_expected_block_offsets`・`late_stamp_is_clamped_to_offset_zero_and_counted`・`unmapped_domain_stamp_is_rejected_and_counted`・`feed_stamp_domain_matches_leg_anchor_domain` / `crates/ruviform-audio-test/tests/pipeline_midi.rs`; `live_input_waits_for_clock_anchor`・`set_clock_jump_triggers_release_and_chase` / `crates/ruviform-audio-plugin-bridge/tests/midi_wiring.rs`; `os_endpoint_midi_loopback_drives_test_instrument` | delegated + direct | timestamp domain/late/unmapped/clock jump の意味論は既存 oracle が所有し、本 case は観測 OS timestamp・clock domain・stamp base・monotonicity・PCM offset を direct に記録する。deterministic clock を実 OS 時刻の証明として扱わない |

したがって旧来の「Test Runtime（#628）との結線」は no-command playthrough
supply について本 slice で解消されたが、stop/seek/loop/pause を OS endpoint
経由で再現する case、overflow fault injection、provisioned endpoint 消失/復帰の
最終受入は本 slice の非目標または residual のままである。
