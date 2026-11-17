---
id: "TSPEC-A0lPTvgFQReroORyTmsciT"
type: "test-spec"
status: "approved"
title: "Issue #629 AC3 Test Runtime wiring test spec"
created: "2026-11-17"
updated: "2026-11-17"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/specs/issue-629-test-runtime-wiring.md"
    - "docs/designs/issue-629-test-runtime-wiring.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-629-test-runtime-wiring.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-629-test-runtime-wiring.md"
    - "docs/designs/issue-629-test-runtime-wiring.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-629-test-runtime-wiring.md"
    - "docs/tasks/issue-629-test-runtime-wiring.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-629-test-runtime-wiring.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #629 AC3 Test Runtime wiring test spec

`crates/ruviform-audio-test/tests/os_endpoint.rs`（新規）が
保証すべき振る舞いを Feature/Rule/Example で記録する。

## Feature: deterministic case → `RunMetadata`（常時実行）

### Rule: PCM fixture block は独立 oracle と一致し `os-l1`/
`deterministic-backend` の `pass` record を emit する

- `PcmFixture` で生成した block を `oracle::pcm_reference_block`
  と照合し、一致時に `result=pass`・`level=os-l1`・
  `environment_kind=deterministic-backend` の JSON を
  `CARGO_TARGET_TMPDIR` へ save し stderr にも出力する。
- Example: `pcm_fixture_block_records_metadata` — canonical
  `mise run test ruviform-audio-test` で実行・Pass。

### Rule: MIDI scenario trace は独立 oracle と一致し `os-l1`/
`deterministic-backend` の `pass` record を emit する

- `MidiScenario`→`run_midi_trace` の trace を
  `oracle::midi_expected_playthrough` と照合し、同上の record
  を emit する。
- Example: `midi_scenario_trace_records_metadata`。

### Rule: `RunMetadata::for_environment` は宣言した
level/environment で record を構築し JSON round-trip する

- `not-run` 開始・`with_result`/`with_expected`/`with_platform`/
  `with_evidence` で確定。serde 語彙は kebab-case。
- Example: `for_environment_builds_endpoint_case_record` —
  `virtual-os-endpoint`/`ext-l3` の構築と JSON 往復を検証。

## Feature: virtual OS endpoint case（`#[ignore]` opt-in）

### Rule: 未 provisioning は `not-run` を fail-closed に記録する

- endpoint 列挙 0 件・pair 不在・host 非対応のいずれでも
  `result=not-run` の record を emit して正常終了する。
  `pass` を記録しない。
- Example: `midi_loopback_os_endpoint`・
  `audio_render_capture_os_endpoint` — 本ホスト（未
  provisioning）で `verdict: NotRun` を確認。

### Rule: stream 成功時のみ `virtual-os-endpoint`/`ext-l3` の
`pass`/`fail` を記録する

- audio leg は `PcmFixture::fill_view` で本番 output callback
  へ供給し、capture channel-0 を独立 oracle（marker/peak/
  monotonic/domain/non-finite/callback counts）で照合する。
- MIDI leg は trace 由来 event を output port へ送信し、input
  drain を ordered subsequence + stamp contract で照合する。
- 失敗時は `result=fail` の record を emit してから panic。
- `environment_kind=virtual-os-endpoint` は実際に stream した
  run のみが主張する（name-match のみの Pass 昇格なし）。

## Feature: 既存資産の非回帰

### Rule: device verifier・probe・lane の挙動を変えない

- `test-audio-virtual-endpoint`・`test-midi-virtual-endpoint`・
  `test-midi-real-device`・`lint-audio-real-input` の exit code・
  verdict 出力・`#[ignore]` gate が不変。
- `ruviform-audio-test` の既存 test が全て Pass のまま
  （additive dev-dep による compile graph 拡大のみ）。
