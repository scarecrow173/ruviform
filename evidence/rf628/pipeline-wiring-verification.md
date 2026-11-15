# Issue #628 — pipeline wiring 検証記録（3rd slice）

- Issue: https://github.com/scarecrow173/ruviform/issues/628（3rd slice: pipeline wiring）
- 対象: `crates/ruviform-audio-test` `pipeline` module + tests
- 環境: Windows 11 x64 / rustc 1.99.1（mise pin）/ cargo workspace
- canonical 検証はすべて `mise run` 経由。cargo 直接実行は補助のみ。

## 実行した検証（canonical = mise）

| コマンド | 結果 | exit |
| --- | --- | --- |
| `mise run fmt-check` | pass（`cargo fmt --all -- --check` 差分なし） | 0 |
| `mise run check ruviform-audio-test` | pass（`cargo check --locked --all-targets`） | 0 |
| `mise run lint ruviform-audio-test` | pass（`cargo clippy … -D warnings`） | 0 |
| `mise run test ruviform-audio-test` | pass（pipeline_output 11 + pipeline_input 13 + pipeline_midi 11 + rt_contract 9 + 既存 suite + doctest 3、0 failed） | 0 |
| `mise run test-rt` | pass（`cargo test --workspace rt_`、全 workspace） | 0 |
| `mise exec -- prek run --all-files` | pass | 0 |

## `test-rt` の `rt_` 収集確認

`tests/rt_contract.rs` に追加した 3 test が `rt_` prefix で
`mise run test-rt`（workspace 全体 `rt_` filter）の出力に個別 `ok`
として現れることを確認（0 件一致の空成功ではない）:

- `rt_pipeline_output_session_pump_does_not_alloc` — production
  `Session` 内 `OutputDataCallback` → `GraphPlan::drive` の callback
  駆動部を `record_output=false` で計測（post-callback capture は
  意図的に alloc するため RT 窓から除外）。
- `rt_pipeline_input_capture_and_process_does_not_alloc` —
  `InputCapture` callback の pool 再利用 + `input_writer` →
  `RtRuntime::process` → `output_reader` 経路を warm-up 後に計測。
- `rt_pipeline_midi_collect_does_not_alloc` — `InputStage::collect_block`
  （scratch-seeded）を計測。

いずれも `tests/rt_contract.rs` 既存の `#[global_allocator]
AllocDisabler` を共用。

## pipeline 経路の検証内容（test 名で確認可能）

- Output: `session_open_pumps_fixture_pcm_matching_independent_expected`
  （`Session::open` → 本番 callback → `GraphPlan::drive` → scripted
  backend capture を独立 Expected と照合）、`xrun_fault_fires_through_
  production_callback_path`、`invalidate_fault_stops_delivery_and_pump_
  fails`、`vanished_device_invalidates_open_stream`、
  `device_spec_without_output_reports_unsupported`、
  `empty_graph_renders_silence`、`negotiated_format_is_reported_by_
  session`、`persistent_open_error_blocks_every_session_open`、
  `capture_disabled_records_nothing_but_pumps_continue`、
  `pcm_fixture_non_sine_patterns_compare_exact`。
- Input: scenario replay（sine/impulse/sweep）→ typed runtime 境界
  process → `compare_planar`/`compare_pcm_block` で独立 Expected と照合、
  scenario gap → silence + `DeviceError::Xrun` 観測、open fault、
  invalidate fault、frames fault（frame-position verbatim 伝播）、
  capture-None fault、`capture_pool_exhaustion_counts_starved_loss`
  （queue-full → `starved` 計数・損失は silent でない）、
  starved/rejected_shape/overflow counter。
- MIDI: `ClockAnchor` mapping、arrival ordering、late event clamp、
  unmapped domain reject + counter、unsupported/dropped traffic、
  `channel_capacity_backpressure_defers_then_delivers_in_order`
  （channel capacity 超過 → `InputPumpStats.deferred` 計数・後続 pump で
  順序保存配送・超過分は production late policy で offset 0 clamp +
  `late` 計数）、`collect_block` の `Event`/`BlockInputStats` 観測。

## Case Schema v1 / environment_kind

- 本 slice の証拠は `environment_kind = deterministic-backend`、
  `level = OS-L1` のみ。deterministic backend の Pass は production
  契約口への scripted data 投入と Expected/Observed 照合が成立する
  ことの証拠であり、実 OS API・GUI・plugin ABI・実デバイスの成功は
  一切主張しない。
- callback 発火は test driver thread の manual pump — 実 RT thread の
  scheduling/priority・実 xrun は本 slice の証明範囲外。

## Finding（製品側 gap、変更せず記録）

- `Session::open` は no-op error callback を install するため、
  production `OutputDataCallback` 内で発火した `DeviceError` の値は
  session surface から観測不能。`PumpReport.errors_fired`/
  `invalidated` count のみが生存する。session への error observer
  公開は製品側の将来判断として `docs/impl/ir/` に記録。
- `ruviform-audio-plugin-bridge::input::InputTx::pump` は
  `pub(crate)` — pipeline は test-owned `InputPump` で同一アルゴリズムを
  mirror し public `InputStage` API を駆動。production 側アルゴリズムの
  将来変更に対する drift リスクは残る（現行 source との一致は
  実装時に照合済み）。

## NotRun / Unsupported（正直な記録）

- `*-msrv` / `*-full` lane: 本 slice の指定検証対象外（MSRV は daily
  CI 管轄）。
- 3 OS（Windows/macOS/Linux）実走: 本 worktree は Windows のみ。
- 実 OS API（WASAPI/WinMM/CoreAudio）・実デバイス・duplex・latency・
  実 thread timing: EXT-L3 領域（`scarecrow173/ruviform#629`）。
- consumer oracle（AudioLens f64/長尺/解析非回帰、Synvelis
  PCM/Project/RT/Recording/MIDI）: 本 slice 範囲外・後続。
- CI lane 統合（新規 lane・`.github/**`・`mise.toml` 変更）: 範囲外。
  新規テストは既存 `test`/`test-rt` lane に自動収集される。
- MIDI `attach()`/実 endpoint 配線の runtime integration:
  `scarecrow173/ruviform#630` slice と棲み分け、本 slice は
  `InputStage` 直駆動に留める。
