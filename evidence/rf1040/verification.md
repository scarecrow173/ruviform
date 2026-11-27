# rf#1040 musical transport handoff — verification evidence

## 対象と環境

- Issue: `scarecrow173/ruviform#1040`
- base SHA: `6e24925d54dabcf824f644042a0102dfb8163494`（最終検証前に `origin/main` を fast-forward 取り込み済み。検証はこの HEAD に本 slice の worktree diff を乗せた状態）
- OS/Arch: Windows 11 / x86_64（MINGW64 shell）
- toolchain: `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, host `x86_64-pc-windows-msvc`
- canonical verification: `mise run ...`。反復中の `cargo test/check` 直接実行は補助証拠であり、下表では使わない。

## 受入条件ごとの結果

| AC | 結果 | 証拠 |
| --- | --- | --- |
| production graph node 経路で musical transport が plugin へ届く | Pass | typed graph `musical_transport_reaches_plugin_with_plan_ppqn_and_tempo`: 150 BPM / host PPQN 480 が 40-frame call position 0/40/80 に対して tick 0/1/2 で観測された。VST3/CLAP real ABI `attach_transport_clicks_follow_production_plan_musical_handoff` も両 format で Pass |
| missing tempo map / missing PPQN / stopped / paused / zero-invalid tempo で捏造しない | Pass | typed graph `musical_transport_negative_controls_do_not_invent_musical_time`: 4状態すべてで `musical=None`。zero と `1_000_000_000` milli-BPM は `TempoMapBuilder::build()` が reject。real ABI でも missing halves と stopped/paused の matched block は silence |
| 120 BPM 以外の click frame が独立計算と一致 | Pass | 150 BPM / 48 kHz / fixture-visible PPQN 960。position tick 954 → expected/observed frame 120、position tick 958 → expected/observed frame 40。amplitude `0.5`、click 以外 IEEE zero、stereo identical。VST3/CLAP 両方で Pass |
| loop/seek/position jump で前 block state を引きずらない | Pass | typed graph loop test: positions `[0,40,0,40]`、ticks `[0,1,0,1]`。real ABI seek phase: tick 958 → frame 40。loop phase: projected tick 930/954 を往復し、tick 954 block のみ frame 120 |

## real ABI oracle の詳細

- route: out-of-process L1 `test_instrument` → `attach()` → `PluginNode` → typed `GraphRuntime` → sink PCM。caller supplied `TransportInfo` は使用していない。
- host plan: 150 BPM、PPQN 480。typed graph Event oracleが host internal non-960 handoff を直接証明する。
- real ABI expectation: 現行 VST3/CLAP adapter contract が plugin-side PPQN 960 へ再投影するため、click law は PPQN 960 で独立計算した。これは host internal PPQN の固定化ではない。
- 480-frame block は 150 BPM/PPQN 480 で exactly 12 host tick（projected PPQN 960 で 24 tick）。scripted position は adapter rounding を検証対象にしない exact position。
- pass-through/substituted block は fresh evidence から除外した。matched block は `call - L1_LOOKAHEAD` の source block に帰属させ、click phase ごとに少なくとも 2 fresh sampleを要求した。
- 両 formatとも lifecycle は `Processing` のままだった。
- `mise run test-l1-real-plugin` では VST3/CLAP それぞれの `l1_real_test_plugin_e2e` が 12 tests 実行され、両方に `attach_integration::attach_transport_clicks_follow_production_plan_musical_handoff ... ok` が出力された。ignore/unbuilt artifact のままの zero-match pass ではない。

## RT evidence

`mise run test-rt` は exit 0。`ruviform-media-core` の `transport_schedule::tests::rt_run_callback_no_alloc` が実行され、musical source 付き plan installed state で allocation failure がなかった。`ruviform-audio-test` の `rt_contract` も 9/9 pass で、`rt_trace_sink_span_direct_does_not_alloc` / `rt_scheduler_callback_and_trace_sink_do_not_alloc` を含む。`rt_midi_wiring_no_alloc` も pass。

## 実行した正規検証

| Command | Exit | Result |
| --- | ---: | --- |
| `mise run fmt-check` | 0 | Pass |
| `mise run check ruviform-media-core` | 0 | Pass |
| `mise run lint ruviform-media-core` | 0 | Pass |
| `mise run test ruviform-media-core` | 0 | Pass（109 unit + 35 doctests） |
| `mise run check ruviform-plugin-core` | 0 | Pass |
| `mise run lint ruviform-plugin-core` | 0 | Pass |
| `mise run test ruviform-plugin-core` | 0 | Pass |
| `mise run check ruviform-audio` | 0 | Pass |
| `mise run lint ruviform-audio` | 0 | Pass |
| `mise run test ruviform-audio` | 0 | Pass |
| `mise run check ruviform-audio-test` | 0 | Pass |
| `mise run lint ruviform-audio-test` | 0 | Pass |
| `mise run test ruviform-audio-test` | 0 | Pass |
| `mise run check ruviform-audio-plugin-bridge` | 0 | Pass |
| `mise run lint ruviform-audio-plugin-bridge` | 0 | Pass |
| `mise run test ruviform-audio-plugin-bridge` | 0 | Pass（typed graph 8/8 を含む） |
| `mise run check ruviform-plugin-test-vst3` | 0 | Pass |
| `mise run lint ruviform-plugin-test-vst3` | 0 | Pass |
| `mise run test ruviform-plugin-test-vst3` | 0 | Pass |
| `mise run check ruviform-plugin-test-clap` | 0 | Pass |
| `mise run lint ruviform-plugin-test-clap` | 0 | Pass |
| `mise run test ruviform-plugin-test-clap` | 0 | Pass |
| `mise run check-l1-real-plugin` | 0 | Pass |
| `mise run lint-l1-real-plugin` | 0 | Pass |
| `mise run test-l1-real-plugin` | 0 | Pass（VST3 12/12, CLAP 12/12 + fault/state lanes） |
| `mise run test-rt` | 0 | Pass（rt_filter 非 zero-match を確認） |
| `mise run check-full` | 0 | Pass（`SpanSink` compile sweep） |
| `mise exec -- prek run --all-files` | 0 | Pass |

初回 lint sweep で `ref_as_ptr`、test code の underscore binding/redundant closure、real ABI test helper の clippy 違反が検出された。いずれも lint を弱めず修正し、該当 lane を再実行して exit 0 を確認した。

## NotRun / NotApplicable

- Workspace full test/build、Feature Matrix、MSRV、3OS 横断、GUI-L2/EXT-L3、手動 CI dispatch: NotRun。本 slice の task spec で Required に指定されず、通常 slice は branch CI 非ブロッキング/Daily CI 集約のユーザー指定に従う。
- macOS/Linux real ABI lane: Unsupported/NotRun。本 slice は新設しない。Windows local laneのみ。
- consumer Synvelis production end-to-end: NotRun/OutOfScope。consumer repo は変更していない。Synvelis production plan builder が tempo map/PPQN を供給する follow-up が必要。

## 境界確認

- `node.rs` に `call_transport_info(..., None, None)` は残っていない。
- fixed 960 fallback は host side に存在しない。960 は real ABI adapter projection の expectation にだけ使う。
- plan-less playback は `musical=None` の defined contract。
- fixture click law、ABI adapter、`mise.toml`、docs index、`Cargo.lock`、rf#1034 lease、consumer repo は変更していない。
- `crates/ruviform-audio-test/tests/rt_contract.rs` の 1 行更新は、`SpanSink` signature compile sweep に必要な lease 外 direct caller として Coordinator 承認を得た最小変更である。musical を検査しない direct call には `MusicalTimeSource::default()` を渡した。
