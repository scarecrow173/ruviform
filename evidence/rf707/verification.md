# Issue #707 — MIDI input cancellation fault slice 検証記録

- Issue: https://github.com/scarecrow173/ruviform/issues/707
- 対象: `crates/ruviform-audio-core/src/midi_input/mock.rs`
  （`MockPort`/`MockInput` read-level fault injector）+
  `crates/ruviform-audio-test/tests/midi_faults.rs`（新規）
- ブランチ: `feat/707-midi-cancellation`（worktree
  `G:/orca/workspaces/ruviform/rf707-midi-cancellation`）
- 環境: Windows 11 x64 / rustc 1.99.1（mise pin）/ cargo 1.99.1 /
  node v26.12.1
- 実施日: 2026-11-17
- canonical 検証はすべて `mise run`/`mise exec` 経由。cargo 直接実行は
  補助のみ（`cargo check -p ruviform-audio-core -p ruviform-audio-test
  --tests`・`cargo test -p ruviform-audio-core midi_input::mock`・
  `cargo test -p ruviform-audio-test --test midi_faults` を反復中に使用）。
- `git fetch origin && git merge origin/main` は "Already up to date"
  （merge 不要を確認済み）。

## 設計判断（実装前に確定）

audio 側 `StreamFault::Cancellation`（#628・merge `0b510cf8`）の MIDI
写像先を 2 案から選定:

- **採用 (a)**: `MockPort`/`MockInput` に read-level fault injector
  （`fail_next_read`/`inject_cancelled`/`is_terminated`）。
  cancelled observable は `MidiInput` subscription 契約の性質であり、
  fixture 層（`fail_next_open`/`inject_dropped`/`inject_unsupported`
  と同じ層）が backend 観測可能性を所有するのが自然。production
  trait `MidiInput`/`MidiInputPort`/`DeviceError` は不変。
- **不採用 (b)**: `MidiLeg`/`MidiFeed` に block/pump 単位の fault
  script。subscription lifecycle 状態を leg 側に複製し、`port()` の
  observable と pipeline observable が乖離し得るため。

詳細は `docs/designs/issue-707-midi-cancellation.md`。

## 実装内容

- `PortState` に `terminated: bool` と `read_faults:
  VecDeque<ReadFault>` を追加。`ReadFault` は private enum
  （`Fail(DeviceError)` / `Cancel`）。
- `MockPort::fail_next_read(error)` — 非終端 1-shot read error。
  FIFO キューに積み、`read` が drain 前に `Err(error)` を返す。
  `pending`/`dropped`/`unsupported`/`out` は一切消費しない。
- `MockPort::inject_cancelled()` — 終端 cancellation。`read` が
  `Err(DeviceError::Backend("cancelled"))` を返し `terminated=true`
  にする。以後の全 `read` は `Err(DeviceError::StreamInvalidated)`。
  `closed` とは独立（`is_closed()` は false のまま）。
- `MockInput::read` — `closed || terminated` を先に検査
  （dead subscription は armed fault を消費しない）→ FIFO で
  `read_faults` を 1 件 pop → `Fail` は error 返却、`Cancel` は
  terminated 化 + `Backend("cancelled")` 返却 → fault なしなら
  従来の drain 処理。
- `tests/midi_faults.rs`（新規 6 test）— `MidiLeg` 経由の
  pipeline 観測: pre-armed cancel・mid-stream cancel・
  `Backend("cancelled")` 単体の非終端負例・FIFO ordered
  error/cancelled/invalidated・loss counter 非終端負例。
- `mock.rs` unit test 4 件追加（非終端性・cancel→invalidate・
  FIFO・dead subscription の不発火）。

production 契約不変の確認: `git status` の差分は
`crates/ruviform-audio-core/src/midi_input/mock.rs`・
`crates/ruviform-audio-test/tests/midi_faults.rs`・
`docs/**/issue-707-*`・`docs/impl/{ir,exp}/issue-707-*`・
`evidence/rf707/` のみ。`src/backend/midi.rs`・
`src/pipeline/midi.rs`・`src/backend.rs`・`src/backend/stream.rs`・
`tests/backend_faults.rs`・`tests/pipeline_midi.rs`・
`mise.toml`・`Cargo.toml`/`Cargo.lock` に差分なし。

## 実行した検証（canonical = mise）

| コマンド | 結果 | exit |
| --- | --- | --- |
| `mise run fmt-check` | pass（`cargo fmt --all -- --check` 差分なし。途中 `cargo fmt -p` で整形適用後に再確認 pass） | 0 |
| `mise run check ruviform-audio-core` | pass（`cargo check --locked --all-targets`） | 0 |
| `mise run check ruviform-audio-test` | pass | 0 |
| `mise run lint ruviform-audio-core` | pass（初回 `clippy::assert_is_empty` 3 件を `assert_eq!(out.len(), 0)` に修正後 pass） | 0 |
| `mise run lint ruviform-audio-test` | pass（初回 `clippy::cast_lossless` 1 件を `u64::from(BLOCK)` に修正後 pass） | 0 |
| `mise run test ruviform-audio-core` | pass（68 unit tests + 37+2 doctests、0 failed。新規 mock test 4 件を含む） | 0 |
| `mise run test ruviform-audio-test` | pass（全 test target 0 failed。新規 `tests/midi_faults.rs` 6 件を cargo が自動収集・実行 — enumerated lane 変更不要を実測確認） | 0 |
| `mise run test-rt` | pass（`cargo test --workspace rt_` 全 workspace。`tests/rt_contract.rs` の audio-test 9 件を含む既存 `rt_` 非回帰） | 0 |
| `mise exec -- prek run --all-files` | pass（全 hook Passed/Skipped） | 0 |
| docs index 再生成 | **未実施（意図的）** — Worker は `docs/**/README.md` を編集・再生成しない。Coordinator が統合時に `doc-maintenance` で再生成する | — |

## test 内訳（本 slice の新規分）

| suite | tests | 新規 |
| --- | --- | --- |
| `ruviform-audio-core` `midi_input::mock::tests` | 9 | `fail_next_read_is_non_terminal_and_does_not_drain`・`inject_cancelled_reports_then_invalidates`・`read_faults_are_consumed_fifo`・`dead_subscription_never_consumes_armed_faults` |
| `ruviform-audio-test` `tests/midi_faults.rs` | 6 | `pre_armed_cancel_reports_backend_error_then_stream_invalidates`・`cancelled_leg_reports_stream_invalidated_on_every_pump`・`mid_stream_cancel_aborts_in_flight_block_then_terminates`・`backend_cancelled_error_value_alone_does_not_terminate`・`read_faults_report_in_arm_order_then_invalidate`・`loss_counters_do_not_terminate_the_subscription` |

## 補助証拠（canonical ではない）

- `mise exec -- cargo check -p ruviform-audio-core -p
  ruviform-audio-test --tests` — exit 0（実装反復中の早期確認）
- `mise exec -- cargo test -p ruviform-audio-core
  midi_input::mock` — 9 tests pass
- `mise exec -- cargo test -p ruviform-audio-test --test
  midi_faults` — 6 tests pass
- `mise exec -- cargo fmt -p ruviform-audio-core -p
  ruviform-audio-test` — 整形適用（fmt-check pass の前段）

## 実施していない検証（正直な記録）

- **実 MIDI デバイス・実 endpoint 上の fault 実 timing — NotRun**
  （EXT-L3。`scarecrow173/ruviform#629`/`#630` 領域。deterministic
  in-process のみで OS-L1 証拠を供給）。
- `*-msrv`・`*-full` 系 lane — NotRun（daily/手動 Full CI 管轄、
  task 指定どおり未実行）。
- 3OS 横断 Native Build/Link — 本 slice はローカル Windows のみ。
  merge 後 SHA の CI が統合検証を供給する（本 PR 本文に明記）。
- docs index（`docs/**/README.md`）再生成 — Worker 範囲外。
  Coordinator が統合時に再生成。
