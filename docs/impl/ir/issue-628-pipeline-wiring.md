---
id: "IMPL-034XW6UqxoYIAHfogFzvj0"
type: "impl"
status: "completed"
title: "Issue #628 pipeline wiring (3rd slice)"
created: "2026-11-15"
updated: "2026-11-15"
owners:
  - "devin-worker"
relations:
  source: []
  changes:
    added:
      - type: "directory"
        path: "crates/ruviform-audio-test/src/pipeline/"
      - type: "file"
        path: "crates/ruviform-audio-test/src/pipeline.rs"
      - type: "file"
        path: "crates/ruviform-audio-test/src/pipeline/common.rs"
      - type: "file"
        path: "crates/ruviform-audio-test/src/pipeline/output.rs"
      - type: "file"
        path: "crates/ruviform-audio-test/src/pipeline/input.rs"
      - type: "file"
        path: "crates/ruviform-audio-test/src/pipeline/midi.rs"
      - type: "file"
        path: "crates/ruviform-audio-test/tests/pipeline_output.rs"
      - type: "file"
        path: "crates/ruviform-audio-test/tests/pipeline_input.rs"
      - type: "file"
        path: "crates/ruviform-audio-test/tests/pipeline_midi.rs"
      - type: "file"
        path: "docs/specs/issue-628-pipeline-wiring.md"
      - type: "file"
        path: "docs/designs/issue-628-pipeline-wiring.md"
      - type: "file"
        path: "docs/plans/issue-628-pipeline-wiring.md"
      - type: "file"
        path: "docs/tasks/issue-628-pipeline-wiring.md"
      - type: "file"
        path: "docs/test-specs/issue-628-pipeline-wiring.md"
      - type: "file"
        path: "docs/impl/exp/issue-628-pipeline-wiring.md"
      - type: "file"
        path: "docs/impl/exp/issue-628-pipeline-wiring.jsonl"
      - type: "file"
        path: "evidence/rf628/pipeline-wiring-verification.md"
    modified:
      - type: "file"
        path: "crates/ruviform-audio-test/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-audio-test/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-audio-test/README.md"
      - type: "file"
        path: "crates/ruviform-audio-test/tests/rt_contract.rs"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-628-pipeline-wiring.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-628-scripted-backend.md"
    - "docs/impl/ir/issue-628-deterministic-test-runtime.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/test-specs/issue-628-pipeline-wiring.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-628-pipeline-wiring.jsonl"
    rejected: []
---

# Issue #628 pipeline wiring (3rd slice)

## Summary

`crates/ruviform-audio-test` に `pipeline` module を追加し、2nd slice の
scripted backend を production 消費者口へ投入する 3 脚の harness を実装した。
製品 crate の `src` は一切変更していない（製品コード変更ゼロが本 slice の
受入そのもの）。

- `pipeline::output` — `PlaybackPipeline`: `ScriptedDeviceHandle` を
  production `ruviform_audio::Session::open(&dyn Device,
  OutputStreamConfig, GraphPlan)` へ渡し、本番 `OutputDataCallback` 内の
  `GraphPlan::drive` を実走。`pump` は
  `ScriptedDeviceHandle::pump_output_next` を呼び、post-callback PCM を
  `captured`、per-pump `PumpReport` を `reports` で公開。
- `pipeline::input` — `InputPipeline`: production
  `Device::open_input_stream` で scripted input stream を開き、
  Source → Gain → Sink の `ruviform-graph` グラフを production
  `Compiler` で compile、`GraphRuntime`/`RtRuntime` で公開。callback 側は
  preallocated `InputCapture`（bounded SPSC pool）へ PCM +
  `InputCallbackInfo` を verbatim capture し、driver 側が各 block を
  `input_writer` → `RtRuntime::process` → `output_reader` に
  exactly-once で流す。観測 PCM/metadata は `ObservedBlock` として保持。
- `pipeline::midi` — `MidiLeg`: `MockBackend`/`MockPort` + `MidiFeed` を
  production `MidiInputEnumerator` → `MidiInputPort::open` →
  `InputStage` 契約で駆動。`InputTx::pump` は `pub(crate)` のため
  test-owned `InputPump` が同アルゴリズムを mirror し、public
  `InputStage::collect_block`/`ClockAnchor`/`map_stamp_to_frame` を使用。
- `pipeline::common` — Expected/Observed 比較 helper
  （`expected_block`/`expected_fixture_block`/`scale_expected`/
  `compare_planar`/`compare_pcm_block`）。Expected は
  `pcm_reference_block` による独立生成で、本番 consumer の出力を
  正解値として共用しない。

## Context

`docs/tasks/issue-628-pipeline-wiring.md`（Issue `scarecrow173/ruviform#628`
3rd slice）。spec/design/plan/test-spec は同 slug の
`docs/{specs,designs,plans,test-specs}/issue-628-pipeline-wiring.md`。
前提: 1st slice deterministic core（PR `scarecrow173/ruviform#638`）、
2nd slice scripted backend（PR `scarecrow173/ruviform#659`、merge
`98fd4306`）。

## Implementation

### Changes

- `src/pipeline.rs` + `src/pipeline/{common,output,input,midi}.rs`:
  上記 3 脚 + 比較 helper。`lib.rs` に `pub mod pipeline` と
  root re-export を追加。
- `Cargo.toml`: pipeline 接続に必要な本番 crate への path 依存を追加
  （`ruviform-audio`、`ruviform-audio-plugin-bridge`、`ruviform-graph`、
  `ruviform-plugin-core`、および推移的に必要な
  `ruviform-audio-core`/`ruviform-media-core`/`ruviform-rt`）。
  dev-dependency に `assert_no_alloc`。いずれも理由コメント付き。
  `Cargo.lock` は path 依存追加に伴う再生成物として取り込み。
- `tests/pipeline_{output,input,midi}.rs`: 35 integration tests。
  `tests/rt_contract.rs`: `rt_pipeline_*` 3 test を追加。
- `README.md`: pipeline module の責務・honest limits・後続 slice 範囲を
  明文化。

### Decisions During Implementation

- **`InputTx` は `pub(crate)`**: production MIDI feed の pump アルゴリズムは
  crate 外から駆動不能。製品を変更せず、test-owned `InputPump` が同一
  アルゴリズムを mirror しつつ public `InputStage` API を駆動する構成を採用
  （製品コード変更ゼロの受入を優先）。
- **Input leg の capture**: 本番 Recording コンポーネントは意図的に未存在
  （`scarecrow173/ruviform#187` 後送り）のため、typed runtime 境界
  `RtRuntime::input_writer`/`process`/`output_reader` を優先経路とし、
  callback PCM+`InputCallbackInfo` は bounded SPSC `InputCapture` pool で
  driver 側へ handoff（`ruviform-audio-device/tests/mock_input_backend.rs`
  の先例に倣う）。capture `None` は合成せず `None` のまま保持、
  frame-position fault は verbatim 伝播。
- **Output leg の error 観測限界**: `Session::open` は no-op error
  callback を install するため、production callback 内の `DeviceError`
  値自体は session surface から観測不能。`PumpReport` の
  `errors_fired`/`invalidated` count のみが生存する — finding として
  記録し、製品側は変更しない。
- **RT 計測境界**: output の `captured` は post-callback に alloc するため
  RT 窓では `record_output` を切り、callback 駆動部のみを計測。
  input は capture pool を warm-up で事前確保し、writer/process/reader
  経路を alloc-free で計測。midi は `collect_block`（scratch-seeded）
  を計測。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-628-pipeline-wiring.jsonl` — InputTx privacy、
  InputCapture handoff、Session error-callback 限界の観察と採用判断。

### Rejected

- なし（代替案は実装せず却下記録のみ: `InputTx` mirror を避けて
  `InputStage` を bypass する案は production 契約の実走にならないため
  不採用）。

## Validation

canonical = mise（cargo 直接実行は補助）:

- `mise run fmt-check` — pass
- `mise run check ruviform-audio-test` — pass
- `mise run lint ruviform-audio-test` — pass（`clippy -- -D warnings`）
- `mise run test ruviform-audio-test` — pass（pipeline 35 + rt_contract 9
  + 既存 suite + doctest 3、0 failed）
- `mise run test-rt` — pass（workspace 全体。`rt_pipeline_output_session_
  pump_does_not_alloc`/`rt_pipeline_input_capture_and_process_does_not_
  alloc`/`rt_pipeline_midi_collect_does_not_alloc` の収集・実行を出力で
  確認）
- `mise exec -- prek run --all-files` — pass

詳細は `evidence/rf628/pipeline-wiring-verification.md`。

## Risks

- `Session::open` の no-op error callback により、production callback 内
  fault の `DeviceError` 値は観測不能（count/invalidated のみ）。
  session 側に error observer を公開するかは製品側の将来判断。
- `InputPump` は production `InputTx::pump` の mirror であり、production
  側アルゴリズムが将来変わった場合に drift し得る。現行 source との
  一致は test で assertion 済みだが、契約 pin の強化は後続 slice 余地。
- deterministic-backend の Pass は OS-L1 証拠のみ。実 OS API・実
  デバイス・実 thread timing・duplex・latency は範囲外（EXT-L3 /
  `scarecrow173/ruviform#629` 領域）。

## Follow-ups

- consumer oracle（AudioLens/Synvelis）、CI lane 統合、MIDI
  `attach()`/実 endpoint 配線（`scarecrow173/ruviform#630` slice）、
  3 OS 実走は後続 slice へ Deferred。
