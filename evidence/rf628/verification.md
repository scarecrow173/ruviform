# Issue #628 — deterministic test runtime core 検証記録

- Issue: https://github.com/scarecrow173/ruviform/issues/628（first slice: deterministic core）
- 対象: test-only crate `crates/ruviform-audio-test`（新設）
- 環境: Windows 11 x64 / rustc 1.99.1（mise pin）/ cargo workspace
- 実施日: 2026-11-14
- canonical 検証はすべて `mise run` 経由。cargo 直接実行は補助のみ。

## 実行した検証（canonical = mise）

| コマンド | 結果 | exit |
| --- | --- | --- |
| `mise run fmt-check` | pass（`cargo fmt --all -- --check` 差分なし） | 0 |
| `mise run check ruviform-audio-test` | pass（`cargo check --locked --all-targets`） | 0 |
| `mise run lint ruviform-audio-test` | pass（`cargo clippy … -D warnings`） | 0 |
| `mise run test ruviform-audio-test` | pass（37 tests + 3 doctests、0 failed） | 0 |
| `mise run test-rt` | pass（`cargo test --workspace rt_`、全 workspace） | 0 |

補助: `mise exec -- cargo check -p ruviform-audio-test`（lockfile 更新のため
`--locked` なしで 1 回実行、以後は `--locked` で pass）。

## `test-rt` の `rt_` 収集確認

`mise run test-rt` は `cargo test --workspace rt_`（name filter）。
出力で `tests\rt_contract.rs` が `running 4 tests` を報告し、以下が
個別に `ok` であることを確認（0 件一致の空成功ではない）:

- `rt_virtual_clock_reads_do_not_alloc` — `VirtualClock::tick` /
  `monotonic_nanos` / `timebase` / `epoch` / config `*_at` が
  `assert_no_alloc` 窓内で allocation-free。
- `rt_pcm_fill_does_not_alloc` — `PcmFixture::fill` / `sample_at` /
  `fill_channel` が allocation-free。
- `rt_scheduler_callback_and_trace_sink_do_not_alloc` — production
  `TransportScheduler::run_callback` × 32 block + `TraceSink` が
  allocation-free（sink capacity 事前確保、entries=32）。
- `rt_trace_sink_span_direct_does_not_alloc` — `SpanSink::span` 直接
  呼出しが allocation-free。

いずれも `#[global_allocator] static ALLOC: AllocDisabler` を持つ
`tests/rt_contract.rs`（先例: `crates/ruviform-audio-device/tests/
mock_input_backend.rs`）で計測。debug build のため allocator が実際に
介在する。

## Case Schema v1 / environment_kind

- 本 slice の証拠は `environment_kind = deterministic-backend`、
  `level = OS-L1` のみ。real OS API（WASAPI/WinMM）・GUI・plugin ABI
  の成功は一切主張しない（POLICIES.md 2026-11-15 仮想優先、
  `scarecrow173/ruviform#627` 契約）。
- `RunMetadata` は `seed` + `fixture_version` + `clock_config` で
  再現可能な typed record（JSON serialization は後続 slice）。

## NotRun / Unsupported（正直な記録）

- `*-msrv` / `*-full` lane: 本 slice の指定検証対象外（タスク固有表）。
- 3 OS（Windows/macOS/Linux）での実行: 本 worktree は Windows のみ。
  pure-Rust deterministic core のため OS 依存はないが、3 OS 実測は
  後続統合 slice / CI lane で実施。
- scripted backend・fault injection・production pipeline 配線・
  consumer oracle（AudioLens/Synvelis）・Windows EXT-L3: 後続 slice。
- `test-rt` の workspace 全体 run で他クレートの `rt_` test も実行
  されたが、本 slice の新規分は上記 4 件のみ。

## 生成物

- `Cargo.lock`: 新規 workspace member `ruviform-audio-test` と
  dev-dependency `assert_no_alloc`（既存 lock 内 version を再利用）
  のため再生成。
- docs index（`docs/{specs,designs,plans,tasks,test-specs}/README.md`、
  `docs/impl/{ir,exp}/README.md`）: doc-maintenance / impl-doc skill
  の index 適用で再生成。
