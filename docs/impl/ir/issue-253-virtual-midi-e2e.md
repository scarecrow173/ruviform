---
id: "IMPL-034Yo5FWaisliHM3bvwqgc"
type: "impl"
status: "completed"
title: "Issue #253 virtual MIDI e2e"
created: "2026-11-16"
updated: "2026-11-16"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/tests/os_endpoint_midi_e2e.rs"
      - type: "file"
        path: "docs/specs/issue-253-virtual-midi-e2e.md"
      - type: "file"
        path: "docs/designs/issue-253-virtual-midi-e2e.md"
      - type: "file"
        path: "docs/plans/issue-253-virtual-midi-e2e.md"
      - type: "file"
        path: "docs/tasks/issue-253-virtual-midi-e2e.md"
      - type: "file"
        path: "docs/test-specs/issue-253-virtual-midi-e2e.md"
      - type: "file"
        path: "docs/impl/ir/issue-253-virtual-midi-e2e.md"
      - type: "file"
        path: "docs/impl/exp/issue-253-virtual-midi-e2e.md"
      - type: "file"
        path: "docs/impl/exp/issue-253-virtual-midi-e2e-exp.jsonl"
      - type: "file"
        path: "evidence/rf253/os-endpoint-midi-e2e-verification-windows-local.txt"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-253-virtual-midi-e2e.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-253-virtual-midi-e2e.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-253-virtual-midi-e2e.md"
    - "docs/designs/issue-253-virtual-midi-e2e.md"
    - "docs/plans/issue-253-virtual-midi-e2e.md"
    - "docs/test-specs/issue-253-virtual-midi-e2e.md"
    - "docs/impl/exp/issue-253-virtual-midi-e2e.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/issue-253-virtual-midi-e2e.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "evidence/rf253/os-endpoint-midi-e2e-verification-windows-local.txt"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-253-virtual-midi-e2e-exp.jsonl"
    rejected: []
---

# Issue #253 virtual MIDI e2e

## Summary

Issue #253 AC1/AC6 の合成経路 verifier を
`crates/ruviform-plugin-test-vst3/tests/os_endpoint_midi_e2e.rs`
に新設した。実 WinMM backend（`ruviform_midi_device::backend()`）
で列挙・open した name-match loopback-style port pair から、
`InputTx::pump` → node `InputStage` → `attach()` の input leg →
`PluginNode` → typed `GraphRuntime`（Source→Processor→Sink）を
経て、out-of-process real-ABI VST3 `test_instrument` fixture の
PCM 出力と stage/`InputTx` counter を独立 oracle で照合する
`#[ignore]`-gated 手動 verifier。provisioned pair が無い環境・
非 Windows・timestamp 非対応 port では assert せず正直な
`NotRun` で終了し、fixture artifact 欠落と pair 以降の失敗は
`Fail` とする fail-closed 構造。`environment_kind=
virtual-os-endpoint` は name match のみを根拠に主張しない。

## Context

`docs/tasks/issue-253-virtual-midi-e2e.md`（plan:
`docs/plans/issue-253-virtual-midi-e2e.md`、spec:
`docs/specs/issue-253-virtual-midi-e2e.md`、design:
`docs/designs/issue-253-virtual-midi-e2e.md`、test-spec:
`docs/test-specs/issue-253-virtual-midi-e2e.md`）の通り、
Issue #253 残存受入（AC1 Virtual-first 移行 row・AC6 Windows
GUI-L2/EXT-L3 row）の verifier 実装 slice。構成部品
（`SystemBackend`・loopback pair verifier・`InputTx::pump`・
`attach()` input leg・L1 runner/fixture）は全て merge 済みで、
production コード変更はゼロ。

## Implementation

### Changes

- `crates/ruviform-plugin-test-vst3/tests/os_endpoint_midi_e2e.rs`
  （新設・1008 行）— module doc に fail-closed 契約と実鎖を記述。
  - fixture plumbing: sibling `l1_real_test_plugin_e2e.rs` の
    instrument-only mirror（`fixture_lib_name`/`instrument_cid`/
    `instrument_descriptor`/`runner_program`/`fixture_path`/
    `assert_fixture_present`/`config`/`instantiate`/
    `drive_to_processing`/`instrument_host`）。
  - attach rig: `attach_integration` と同型 — `AttachConfig`
    （stereo bus・48 kHz・512 frame・`DeliveryPolicy::Exact`・
    `InputConfig::new(64)`）→ `attach` → `GraphBuilder` +
    `ConnDef` → `Validator` → `Compiler` → `GraphRuntime::
    publish` → `RtRuntime`。
  - OS endpoint: `loopback_pair_candidates`（`info().name`
    双方向一致、port id は方向で体系が異なるため name が唯一の
    正直な鍵）/`select_loopback_pair`/`open_loopback_pair`
    （timestamp capability gate・input 優先 open）。
  - stamp/anchor: `ObservingInput`（production `MidiInput` への
    passive tap — `read`/`close` を delegate し全
    `TimedMidiInput` を記録）・`stamp_contract_holds`（domain/
    `Carried`/非減少 ticks/`base=1/1000`）・`match_expected`
    （ordered subsequence）・`pump_until_sequence`（5 s
    deadline）。
  - oracle: `assert_pcm_oracle`（`first_sound ==
    DELIVERED_LOOKAHEAD=2`・attack 前完全無音・`ANCHOR_FRAME=100`
    の bit-exact attack 値・dual-mono・release tail が stamped
    `NoteOff` + `RELEASE_FRAMES=2400` + 1 block 内・末尾 3 block
    無音）・`assert_counter_oracle`（`late`/`anchor_rejected`/
    `dropped_capacity`=0、`mapped==delivered`、`delivered >= 4`、
    `rejected` = anchor stamp より前の観測 event 数、`InputTx`
    backend counter が pump 観測累積と一致）・
    `assert_close_semantics`（両方向 `StreamInvalidated`）。
  - entry: `os_endpoint_midi_loopback_drives_test_instrument`
    （`#[test]` + `#[ignore]`）。
- `crates/ruviform-plugin-test-vst3/Cargo.toml` —
  `[dev-dependencies]` に `ruviform-midi-device` path dep を
  追加（Coordinator 承認済み・理由コメント付き。
  `ruviform-audio-device` は backend 面を直接使わないため不要
  と判断し未追加）。
- `Cargo.lock` — 当該 dep 1 行の再生成差分のみ。
- docs chain 7 file + exp jsonl + `evidence/rf253/
  os-endpoint-midi-e2e-verification-windows-local.txt`。

### Decisions During Implementation

- **初版の全面改訂**: 最初の draft は `attach` 戻り値・
  `InputStageCounters` field 名・`set_clock_anchor` 等を推測で
  記述しており実 API と不一致 — `attach.rs`/`control.rs`/
  `input.rs`/`l1_real_test_plugin_e2e.rs`/`loopback.rs`/
  `PluginNode::process`/media-core を実コードで確認して全面
  書き直した（compile で検証）。
- **anchor は観測 stamp から較正（`Estimated`）**: scripted 先例
  は anchor を先に決めて event をその位置に置くが、本 verifier
  は OS が stamp した実 tick を `ClockAnchor{frame:
  EngineFrame(0, ANCHOR_FRAME)}` に写す — stamp→frame の実写像を
  PCM oracle で検証するのが目的のため。anchor 未 publish の
  pending event は待機する既存契約（V9/V11/V12 系）により
  pump→process の順序付けが成立する。
- **`f_off` は `NoteOff` stamp から導出**: stamp は非減少が
  契約検査済みのため `note_off_stamp >= on_stamp` が保証され、
  u64 減算の underflow は起きない。release tail の bound は
  `f_off + RELEASE_FRAMES + BLOCK`（約 1 block の許容）。
- **`stamp base` 非対応は `NotRun`、他の stamp 違反は `Fail`**:
  `×48` frame 換算は `base=1/1000` 前提のため foreign base は
  verifier の envelope 外（環境差）— domain/provenance/単調性は
  契約違反であり実 Fail。
- **counter oracle の `rejected` 独立期待値**: anchor より前の
  stamp を持つ観測 event は `BeforeAnchor` で reject される —
  全 event を block 0 処理前に pump する構成のため期待値は
  `observed.iter().filter(stamp < on_stamp).count()` で厳密に
  計算できる。
- **`#[expect(clippy::cast_precision_loss)]`**: attack 値の
  `u32 as f32 / 4_294_967_295.0` は fixture の正規化式と同じ
  bit-exact 計算 — repo 既存 pattern（80+ 箇所）に倣い
  `#[allow]` ではなく `#[expect]` + 正当化 comment。
- **dev-dep は `ruviform-midi-device` のみ**: 承認リストには
  `ruviform-audio-device` もあったが、verifier は
  `ruviform_midi_device::backend()` の port API のみを使い
  audio-device の面を必要としない — 必要最小限の原則で未追加。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-253-virtual-midi-e2e.md` +
  `issue-253-virtual-midi-e2e-exp.jsonl` — API 調査・初版
  rewrite・lint 分解・anchor 較正方式の試行記録。

### Rejected

- なし（外部 provisioning・lane 登録は scope 外の deferred で
  あり experiment ではない）。

## Validation

- `cargo check -p ruviform-plugin-test-vst3 --features
  l1-runner --tests` — Pass（補助証拠、exit 0）。
- `cargo clippy -p ruviform-plugin-test-vst3 --features
  l1-runner --tests -- -D warnings` — Pass（補助証拠、exit 0）。
- `cargo check -p ruviform-plugin-test-vst3`（feature なし）—
  Pass（`#![cfg]` gate で file が空、補助証拠、exit 0）。
- `cargo fmt -p ruviform-plugin-test-vst3 -- --check` — Pass
  （補助証拠）。
- canonical mise lane（`fmt-check`/`check`/`lint`/`test`
  `ruviform-plugin-test-vst3`、`check-l1-real-plugin`/
  `lint-l1-real-plugin`/`test-l1-real-plugin`、`prek`）—
  `evidence/rf253/os-endpoint-midi-e2e-verification-windows-
  local.txt` に記録。
- 補助実走 `cargo test -p ruviform-plugin-test-vst3 --features
  l1-runner --test os_endpoint_midi_e2e -- --ignored` — 同上
  evidence に verdict を記録（pair 未 provision なら
  `NotRun`）。

## Risks

- pair 存在時の Pass 経路は本 slice では未実走 — oracle は
  scripted 先例と fixture 定数から導出したが、実 WinMM stamp の
  粒度・loopback 転送遅延でのフレーク可能性は provisioning 環境
  での初回実走まで残る（Fail は正直な失敗として観測値を残す）。
- `mapped == delivered` は「全 mapped event が処理済み block 範囲
  に写像される」前提 — ambient traffic が anchor より大幅に後の
  stamp を持ち込むと mapped>delivered で Fail し得る（silent
  pass ではなく正直な Fail、観測値は verdict に残る）。
- name match は virtual endpoint の証明ではない — verdict/docs/
  evidence で `virtual-os-endpoint` 非主張を固定済みだが、
  provisioning 確立後の evidence 昇格は人間の確認が前提。

## Follow-ups

- loopMIDI/Windows MIDI Services の実 provisioning と
  `virtual-os-endpoint` evidence — 後続 slice（repo 外）。
- `mise.toml` への本 verifier 収集 lane 登録 — 別 Worker 占有、
  Coordinator 統合時（PR 本文「未対応の作業」に記録）。
- 仮想 endpoint 経由の reconnect/loss/permission/xrun・
  Test Runtime（`scarecrow173/ruviform#628`）結線・consumer
  oracle 照合 — #629/#630 側受入・別 Issue。
- macOS/Linux MIDI backend 実装・実 DAW 往復 — 別 slice。
