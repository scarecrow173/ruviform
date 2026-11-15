# Issue #628 — scripted port-trait backend 検証記録（2nd slice）

- Issue: https://github.com/scarecrow173/ruviform/issues/628（2nd slice: scripted backend）
- 対象: `crates/ruviform-audio-test` `backend` module + tests
- 環境: Windows 11 x64 / rustc 1.99.1（mise pin）/ cargo workspace
- canonical 検証はすべて `mise run` 経由。cargo 直接実行は補助のみ。

## 実行した検証（canonical = mise）

| コマンド | 結果 | exit |
| --- | --- | --- |
| `mise run fmt-check` | pass（`cargo fmt --all -- --check` 差分なし） | 0 |
| `mise run check ruviform-audio-test` | pass（`cargo check --locked --all-targets`） | 0 |
| `mise run lint ruviform-audio-test` | pass（`cargo clippy … -D warnings`） | 0 |
| `mise run test ruviform-audio-test` | pass（backend 15 + backend_faults 9 + rt_contract 6 + 既存 suite、0 failed） | 0 |
| `mise run test-rt` | pass（workspace 全体 `rt_` filter） | 0 |

## `test-rt` の `rt_` 収集確認

`tests/rt_contract.rs` に追加した 2 test が `rt_` prefix で収集・
実行されることを `cargo test --test rt_contract` および
`mise run test-rt` の出力で確認（0 件一致の空成功ではない）:

- `rt_scripted_backend_output_pump_does_not_alloc` —
  `record_output: false` の output stream に対し steady-state pump
  8 回（prologue・fault scan・scratch fill・view 構築・timestamp・
  callback・counter 更新）を `assert_no_alloc` 窓内で実行、
  allocation なし。`#[global_allocator] AllocDisabler` は
  `rt_contract.rs` 既存の 1 つを共用。
- `rt_scripted_backend_input_pump_does_not_alloc` — scenario 全域
  カバー（gap 無し）で input pump 8 回を同様に計測、allocation なし。

## Case Schema v1 / environment_kind

- 本 slice の証拠は `environment_kind = deterministic-backend`、
  `level = OS-L1` のみ。deterministic backend の Pass は port 契約の
  実装可能性の証拠であり、実 OS API（WASAPI/WinMM）・GUI・plugin
  ABI の成功は一切主張しない。
- callback 発火は test driver thread の manual pump — 実 RT thread
  の scheduling/priority は本 slice の証明範囲外。

## NotRun / Unsupported（正直な記録）

- `*-msrv` / `*-full` lane: 本 slice の指定検証対象外（タスク固有表、
  MSRV は daily CI 管轄）。
- 3 OS（Windows/macOS/Linux）実走: 本 worktree は Windows のみ。
  pure-Rust deterministic 実装のため OS 依存はないが、3 OS 実測は
  後続統合 slice / CI lane で実施。
- production pipeline 配線（Input→Recording、Output→Playback、
  MIDI→Timestamp/Host adapter）・consumer oracle（AudioLens/
  Synvelis）・CI lane 追加: 後続 slice（本 slice 範囲外）。
