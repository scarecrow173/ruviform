---
id: "IMPL-034VFhJztohjkuJtNUpBHT"
type: "impl"
status: "completed"
title: "Issue #253 unsupported transparency slice"
created: "2026-11-11"
updated: "2026-11-11"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-253-unsupported-transparency.md"
      - type: "file"
        path: "docs/designs/issue-253-unsupported-transparency-design.md"
      - type: "file"
        path: "docs/plans/issue-253-unsupported-transparency-plan.md"
      - type: "file"
        path: "docs/test-specs/issue-253-unsupported-transparency.md"
      - type: "file"
        path: "docs/tasks/issue-253-task-unsupported-transparency.md"
      - type: "file"
        path: "docs/impl/ir/issue-253-unsupported-transparency-slice.md"
      - type: "file"
        path: "evidence/issue-253-real-midi-lane/README.md"
    modified:
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "crates/ruviform-audio-core/src/midi_input/mock.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/input.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/status.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/tests/midi_delivery.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/tests/midi_wiring.rs"
      - type: "file"
        path: "crates/ruviform-midi-device/README.md"
      - type: "file"
        path: "crates/ruviform-midi-device/tests/ports.rs"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-253-task-unsupported-transparency.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-253-unsupported-transparency-plan.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-253-unsupported-transparency.md"
    - "docs/designs/issue-253-unsupported-transparency-design.md"
    - "docs/plans/issue-253-unsupported-transparency-plan.md"
    - "docs/test-specs/issue-253-unsupported-transparency.md"
    - "docs/impl/ir/rf-253-l1-report-verb.md"
    - "docs/impl/ir/issue-253-os-midi-input-backend-adapter-windows-winmm-os-unsupported.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-253-unsupported-transparency-plan.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-253-unsupported-transparency.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #253 unsupported transparency slice

## Summary

Issue #253 の残存 slice — 2 つの先行 impl record が「別 slice」と
明記していた既知ギャップを閉じた:

1. **bridge `unsupported` 計数透過の欠落を解消** —
   `MidiReadStats.unsupported`（受信したが channel-voice 語彙へ
   翻訳不能な traffic の計数）が `InputTx::pump` /
   `InputStage::read` で読み捨てられていたのを、`PumpStats::
   backend_unsupported`（per-call）・`InputTx::unsupported_backend`
   累積 + getter・`InputStageCounters::unsupported_backend`・
   `NodeStatus` mirror まで `dropped` と同じ形状で透過させた。
   `dropped` とは別 counter のまま（unsupported は過負荷ではなく
   語彙設計上の除外 traffic）。
2. **opt-in 実機検証 lane の scaffold** — `ruviform-midi-device`
   の `#[ignore]` E2E を enumerate→open→repeated read→timestamp/
   `dropped`/`unsupported` 観測→close→`StreamInvalidated` の全経路
   へ拡張し、bridge 側に `SystemBackend`→`InputTx::pump`→
   `InputStage` の `#[ignore]` 実機 E2E を追加（dev-dep
   `ruviform-midi-device`）。暫定入口は
   `cargo test -p <crate> -- --ignored`、canonical mise lane 登録は
   `mise.toml` が別 Worker 占有のため follow-up として記録。
3. **mock の unsupported 注入手段** — `MockPort::inject_unsupported`
   を追加し、`MockInput::read` が
   `MidiReadStats::with_unsupported` で両 counter を
   since-last-read 意味論で返すようにした。

## Context

Issue #253。spec → design → plan → test-spec → task の文書チェーン
（`docs/{specs,designs,plans,test-specs,tasks}/issue-253-unsupported-transparency*.md`）
に従う。Issue #253 全体は open のまま — macOS/Linux backend・実
DAW 往復・capability `Declared`→`Verified` 昇格・V9-V13・video
verb・MIDI2/UMP（#317）・consumer 消費実装は後続 slice。

出典行（本 slice で閉じる既知ギャップ）:

- `docs/impl/ir/rf-253-l1-report-verb.md` L311-312:
  「`ruviform-audio-plugin-bridge` の `unsupported` 計数透過欠落 —
  別 slice（編集範囲外、PR 本文「未対応の作業」に記録）」、
  L335-336: 「bridge 側 `unsupported` 計数の透過欠落 —
  capability 行列の設計 slice と一緒に扱う」。
- `docs/impl/ir/issue-253-os-midi-input-backend-adapter-windows-winmm-os-unsupported.md`
  L269-270: 「host-graph consumer 配線（`InputTx::pump` 経路との
  結線確認は別 slice で実施）」。

## Implementation

### Changes

- `crates/ruviform-audio-core/src/midi_input/mock.rs`
  - `PortState.unsupported: u64` を追加。
  - `MockPort::inject_unsupported(n)` を追加 — 次回 `read` で
    `MidiReadStats.unsupported` に `n` を一度だけ surface。
  - `MockInput::read` が `MidiReadStats::with_unsupported(
    take(dropped), take(unsupported))` を返すように変更
    （`dropped` と同じ since-last-read 消費意味論）。
  - unit test `injected_unsupported_surfaces_once_independently_of_dropped`
    を追加（両 counter の独立 drain と reset 検証）。
- `crates/ruviform-audio-plugin-bridge/src/input.rs`
  - `InputStageCounters.unsupported_backend`（`#[non_exhaustive]`
    additive field）+ `InputStage::read` で `stats.unsupported`
    を累積。
  - `PumpStats.backend_unsupported`（`#[non_exhaustive]` additive
    field、per-call 観測）。
  - `InputTx::unsupported_backend` 累積 field + `pump` での加算 +
    `unsupported_backend()` getter（`dropped_backend` と同じ
    non-RT read-only 契約）。
- `crates/ruviform-audio-plugin-bridge/src/status.rs`
  - `NodeStatus::input_unsupported_backend: AtomicU64` を追加。
  - `set_input` が absolute store で mirror、`snapshot` が
    `input.unsupported_backend` として復元。
  - unit test `set_input_mirrors_whole_counters_including_unsupported`
    を追加（全 counter の whole-struct mirror と absolute store
    意味論を検証）。
- `crates/ruviform-audio-plugin-bridge/tests/midi_delivery.rs`
  - `ScriptedInput.unsupported` field 追加（全 literal 更新）、
    `read` が `with_unsupported` を返すよう変更。
  - `midi_delivery_unsupported_is_observable` を追加 —
    `unsupported_backend` 累積（read 跨ぎで 4→7）、`dropped_backend`
    との独立性、event 配送への非干渉を検証。
- `crates/ruviform-audio-plugin-bridge/tests/midi_wiring.rs`
  - `mock_backend_drives_input_stage_to_plugin` を拡張 —
    `inject_unsupported` → pump の `backend_unsupported` per-call・
    `InputTx::unsupported_backend()` 累積（4→7）・`dropped` との
    独立性を検証。channel 経路では `InputTx` が counter 所有者で
    stage mirror は 0 のままであることを明示的に assert（ownership
    分離の文書化 assert）。
  - `real_midi_device_drives_input_tx_to_stage`（`#[ignore]`）を
    追加 — `ruviform_midi_device::backend()` → enumerate →
    （ports なし/`Err` なら honest exit）→ open → declared
    `clock_domain` で `set_clock_anchor` → 10 回 pump（500ms 窓）
    → per-call 合計と `InputTx` 累積の一致 assert → process →
    snapshot 観測記録 → close → `StreamInvalidated` assert。
- `crates/ruviform-audio-plugin-bridge/Cargo.toml`
  - dev-dependency `ruviform-midi-device = { path =
    "../ruviform-midi-device" }` を追加（実機 E2E 用・公開 surface
    には出ない。依存方向 bridge→adapter で合法）。
- `crates/ruviform-midi-device/tests/ports.rs`
  - `real_device_open_read_close` を
    `real_device_open_read_observe_close` へ拡張・`#[cfg(windows)]`
    を外して全 platform で compile 可能に。
  - enumerate → （`Err`・ports なしなら honest skip）→
    capabilities 記録 → open → 10 回 read（500ms 窓）→
    `dropped`/`unsupported` 合計記録 → stamped port では declared
    domain・`Carried` provenance・non-decreasing ticks を assert →
    close → `StreamInvalidated` assert。
- `crates/ruviform-midi-device/README.md`
  - opt-in 実機検証節を追加 — `#[ignore]` E2E の経路・honest skip
    契約・暫定入口コマンド・canonical lane 登録が follow-up で
    あること。
- `evidence/issue-253-real-midi-lane/README.md`
  - 本環境での lane 実行記録（Windows 11・rustc 1.99.0・MIDI
    ポートなし → 両 lane honest skip、判定 NotRun）。

### Decisions During Implementation

- **channel 経路 counter の所有権**: `InputTx::pump` は
  `MidiInput::read` を直接呼ぶため、`dropped`/`unsupported` の
  backend 計数は `InputTx` が持ち `InputStageCounters` には載せ
  ない（`InputStage::read` 直接経路のみ stage counter に累積）。
  `dropped_backend` の既存 ownership 分離をそのまま踏襲し、
  wiring test に stage mirror = 0 の assert を置いて契約を
  文書化した。
- **`#[non_exhaustive]` additive のみ**: `InputStageCounters`/
  `PumpStats` とも `#[non_exhaustive]` struct のため field 追加は
  破壊的変更ではない。`MidiReadStats::new(dropped)` は
  `unsupported=0` のまま後方互換を維持。
- **実機 lane は観測のみ assert**: 物理 MIDI traffic は環境依存で
  生成不能なため、`received`/`dropped`/`unsupported` の非零は
  assert せず `eprintln` で記録。assert するのは契約不変条件のみ
  （declared domain・`Carried`・non-decreasing ticks・
  `StreamInvalidated`・per-call と累積の一致）。
- **`ports.rs` import の cfg 分離**: `MidiInputPort` は
  `#[cfg(windows)]` test 内でしか使われないため import を
  `#[cfg(windows)]` gate — 非 Windows で `unused_imports` warning
  を避ける（review で検出・`x86_64-unknown-linux-gnu` target の
  `cargo check --all-targets` で確認済み）。
- **暫定入口の文書化**: canonical lane 登録（`mise.toml`）は別
  Worker 占有のため不可 — `cargo test -p <crate> -- --ignored` を
  `#[ignore]` reason・README・evidence に記録し、follow-up 行と
  して残した。

## Related Experiments

### Adopted

なし（既存 `dropped` 経路の形状を踏襲する additive 変更のみ）。

### Rejected

なし。

## Validation

canonical = `mise run` 経由。Windows 11 x64・rustc 1.99.0
(b940084d7 2026-09-28) で実施済み:

- `mise run fmt-check`: Pass（`cargo fmt --all -- --check`）。
- `mise run check`: Pass（`cargo check --workspace --all-targets`）。
- `mise run lint`: Pass（`cargo clippy --workspace --all-targets
  -- -D warnings`）。
- `mise run test`: Pass（`cargo test --workspace`、exit 0）。
  初回実行で `ruviform-plugin-host` の
  `hang_converges_on_command_timeout` が `SpawnFailed { cause:
  Timeout }` で失敗したが、単独再実行で Pass・全 workspace 再実行
  でも Pass — spawn timeout の infra flaky で本変更と無関係
  （plugin-host は scope 外・diff なし）。
- `mise run test-rt`: Pass（`cargo test --workspace rt_`、exit 0）。
- `cargo check -p ruviform-midi-device --target
  x86_64-unknown-linux-gnu --all-targets`: Pass（非 Windows cfg
  経路の compile 確認 — 補助証拠）。
- `cargo check -p ruviform-audio-plugin-bridge --target
  x86_64-unknown-linux-gnu --tests`: Pass（同・補助証拠）。
- `cargo test -p ruviform-midi-device -- --ignored`: Pass（lane
  実行・"no MIDI input ports — nothing to verify" で honest
  skip — 補助証拠、暫定入口）。
- `cargo test -p ruviform-audio-plugin-bridge -- --ignored`: Pass
  （同 — 補助証拠、暫定入口）。
- `cargo run -p ruviform-midi-device --example list_midi_ports`:
  `no MIDI input ports`（環境確認 — 補助証拠）。
- review-protocol self-review 1 回実施。blocking finding
  （`MidiInputPort` の非 Windows unused import）を修正済み、
  残 blocker なし。

NotRun / Unsupported（正直な記録）:

- 実 MIDI デバイスを接続した実機検証: NotRun（本環境に MIDI input
  ポートなし — 両 `#[ignore]` lane が honest skip。証拠:
  `evidence/issue-253-real-midi-lane/`）。
- 非対応 OS（macOS/Linux）での `DeviceError::Unsupported` 経路
  実行: NotRun（Windows のみで実施。compile は Linux target で
  確認済み）。
- canonical mise lane 登録: 本 slice 対象外（`mise.toml` 別 Worker
  占有 — follow-up 行）。
- `mise run test-msrv`: 該当 task なし（`test-*-msrv` は per-crate
  で本変更 crate は対象外）。
- 実 DAW 往復・capability `Verified` 昇格・V9-V13・video verb・
  MIDI2/UMP・consumer 消費実装: 本 slice 対象外。

## Risks

- 実機経路（open→read/pump→close・stamp 契約・counter 実測）は
  本環境では未観測 — 実デバイス保有環境での `#[ignore]` 実行が
  必要。lane の honest skip 機構自体は動作確認済み。
- `InputTx` 累積計数は status mirror に出ない（`dropped_backend`
  同様 getter のみ）— consumer が snapshot 経由で pump 側計数を
  必要とする場合は別途 mirror 追加が要る（spec Deferred Design
  Concerns に記録）。

## Follow-ups

- canonical lane（`test-midi-real-input` 相当）の `mise.toml`
  登録 — Issue #251 `test-audio-real-input` パターン準拠
  （`mise.toml` 占有解放後、別 slice）。
- 実デバイス保有環境での `#[ignore]` lane 実行と evidence 更新。
- macOS CoreMIDI / Linux ALSA・JACK・PipeWire backend（別 slice）。
- `InputTx` 累積計数の `NodeStatus` 統合 mirror（consumer 需要が
  出た時点）。
- Issue #253 は open のまま（本 slice で Close しない）。
