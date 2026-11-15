# Issue #628 close-out slice — 検証記録

- Issue: https://github.com/scarecrow173/ruviform/issues/628（close-out slice）
- 対象: test-only crate `crates/ruviform-audio-test`（RunMetadata JSON・
  `StreamFault::Cancellation`・docs 整合・AC 棚卸し）
- ブランチ: `feat/628-close-out`（worktree `G:/orca/workspaces/ruviform/rf628-close-out`）
- 環境: Windows 11 x64 / rustc 1.99.1（mise pin）/ cargo 1.99.1 /
  node v26.12.1
- 実施日: 2026-11-15
- canonical 検証はすべて `mise run`/`mise exec` 経由。cargo 直接実行は
  補助のみ（`cargo check -p ruviform-audio-test --all-targets` を
  反復中に使用）。

## 実行した検証（canonical = mise）

| コマンド | 結果 | exit |
| --- | --- | --- |
| `mise run fmt-check` | pass（`cargo fmt --all -- --check` 差分なし。初回は新規コードの整形差分を検出 → `cargo fmt` 適用後 pass） | 0 |
| `mise run check ruviform-audio-test` | pass（`cargo check --locked --all-targets`） | 0 |
| `mise run lint ruviform-audio-test` | pass（`cargo clippy --locked --all-targets -p ruviform-audio-test -- -D warnings`。初回 `clippy::unnecessary_wraps` 2 件を `cancelled_pump_*` の戻り値修正で解消後 pass） | 0 |
| `mise run test ruviform-audio-test` | pass（117 tests + 3 doctests、0 failed） | 0 |
| `mise run test-rt` | pass（`cargo test --workspace rt_` 全 workspace。`tests/rt_contract.rs` の audio-test 9 件を個別確認: `rt_virtual_clock_reads_do_not_alloc`・`rt_pcm_fill_does_not_alloc`・`rt_trace_sink_span_direct_does_not_alloc`・`rt_scheduler_callback_and_trace_sink_do_not_alloc`・`rt_scripted_backend_{output,input}_pump_does_not_alloc`・`rt_pipeline_{output_session,input_capture_and_process,midi_collect}_does_not_alloc` — 既存 `rt_` テスト非回帰） | 0 |
| `mise exec -- prek run --all-files` | pass（全 hook Passed/Skipped） | 0 |
| docs index 再生成 | `doc_maintenance.js index plan` → `index apply --expect <planHash>` で `docs/{specs,designs,plans,tasks,test-specs,impl/ir,impl/exp}/README.md` を再生成。再 plan で全 dir `in-sync` を確認 | 0 |
| `doc_status.js lint` | repo 全体で既存 finding 815 件（大半は先行 slice 由来の id-prefix/reciprocal 警告）。本 slice の新規 doc が出す `inconsistent-reciprocal-relation` warning 3 件は先行 slice（pipeline-wiring 等）と同型の既存 convention で非 blocking | — |

## test 内訳（`mise run test ruviform-audio-test`、exit 0）

| suite | tests | 新規 |
| --- | --- | --- |
| `tests/backend.rs` | 15 | — |
| `tests/backend_faults.rs` | 15 | cancellation 6 件（output/input scripted・mid-stream inject×2・`Backend("cancelled")` 単独負例・Invalidate 併用 error 順序） |
| `tests/clock.rs` | 9 | — |
| `tests/metadata.rs` | 10 | JSON 系 7 件（string/file round-trip・wire 語彙 pin・enum spellings・`EnvironmentKind` serde==`as_str`・reject 系・missing file） |
| `tests/midi.rs` | 7 | — |
| `tests/pcm.rs` | 14 | — |
| `tests/pipeline_input.rs` | 13 | — |
| `tests/pipeline_midi.rs` | 11 | — |
| `tests/pipeline_output.rs` | 11 | — |
| `tests/rt_contract.rs` | 9 | — |
| doctests | 3 | — |

## 補助証拠（canonical ではない）

- `mise exec -- cargo check -p ruviform-audio-test --all-targets` — exit 0
  （lockfile 更新のため `--locked` なしで実行。以後は mise task の
  `--locked` 付き check/lint が pass = lockfile 整合）。
- `git diff Cargo.lock` — `ruviform-audio-test` の deps に
  `serde`/`serde_json` が追加されるのみ（両 crate は workspace 既存
  lockfile entry）。手 merge なし・再生成のみ。

## `rt_` 非回帰の確認方法

`mise run test-rt` は `cargo test --workspace rt_`（name filter）。
出力で `tests\rt_contract.rs`（audio-test）が `running 9 tests` を
報告し、上記 9 件が個別に `ok` であることを確認（0 件一致の空成功
ではない）。本 slice の新規テストに `rt_` prefix は付けていない
（JSON save/load は alloc/IO を伴う非 RT API のため）。

## 実施していない検証（正直な記録）

- 3OS 横断 Native Build/Link/Headless — merge 後対象 SHA の CI 証拠
  （daily/手動 Full CI・3OS matrix）で判定。ローカルは Windows x64 のみ。
- `*-msrv`・`*-full` 系 — daily/手動 Full CI 管轄（task 指定により
  実行しない）。
- 実 OS/実デバイス/実 thread timing・EXT-L3（Windows 仮想 endpoint、
  real plugin ABI）— `scarecrow173/ruviform#629` 領域、NotRun。
- MIDI→Host adapter `attach()` 実 endpoint — `scarecrow173/ruviform#630`
  領域、Deferred。
- consumer oracle（AudioLens/Synvelis）の resolved SHA 別集計 —
  consumer repo 側の検証成果物、本 repo からは主張しない。
