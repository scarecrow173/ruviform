# Issue #253 virtual MIDI e2e verifier — 実装実験記録（2026-11-16）

`docs/tasks/issue-253-virtual-midi-e2e.md` の slice（実 WinMM
backend → loopback-style pair → `InputTx::pump` → `InputStage` →
`attach` → `PluginNode` → typed `GraphRuntime` → out-of-process
VST3 `test_instrument` の `#[ignore]` fail-closed verifier）実装で
試行・観測した事項の記録。実装結果の正本は
[`docs/impl/ir/issue-253-virtual-midi-e2e.md`](../ir/issue-253-virtual-midi-e2e.md)、
イベントログは
[`issue-253-virtual-midi-e2e-exp.jsonl`](issue-253-virtual-midi-e2e-exp.jsonl)
を参照。

## 試行 1: API 面の実コード確認と初版の全面改訂

- **目的**: `attach`/`InputTx::pump`/`InputStageCounters`/
  `set_clock_anchor`/typed graph/L1 fixture の実シグネチャを確定する。
- **観測**: 初版 draft は推測 API（`attach` の戻り値 shape、
  counter field 名、anchor publish 経路、graph compile 手順）を
  含み、実コードと不一致だった。`attach.rs`（`AttachConfig::input`・
  `Attached{input_tx, control, node, ports}`）・`input.rs`
  （`InputTx::pump(&mut dyn MidiInput) -> DeviceResult<PumpStats>`、
  `PumpStats.backend_dropped`/`backend_unsupported`・累積
  `dropped_backend()`/`unsupported_backend()`）・`control.rs`
  （`PluginNodeControl::set_clock_anchor`/`snapshot().input`）・
  `l1_real_test_plugin_e2e.rs::attach_integration`（48 kHz・
  stereo・512 frame・`GraphBuilder`+`ConnDef`+`Validator`+
  `Compiler`+`GraphRuntime::publish`+`RtRuntime`）・`loopback.rs`
  （name-match pair・open 順序・close semantics）を精読して全面
  書き直した。
- **結論**: 実 API に揃えた版が `cargo check --features
  l1-runner --tests` を通過。以後の lint/設計判断はこの版を正本とする。

## 試行 2: stamped-offset oracle の anchor 較正方式

- **目的**: OS backend が stamp した実時刻を engine frame に写し、
  PCM 上の intra-block offset を検証する方式を決める。
- **観測**: scripted 先例（`l1_real_test_plugin_e2e.rs`）は anchor
  を先に固定して event をその位置に置くが、本 verifier は逆方向
  — `WinMM` が `midiInStart` からの ms tick（`base=
  TimeBase::new(1, 1_000)`）で stamp した `NoteOn` を観測し、
  `ClockAnchor{domain, stamp_ticks: observed, frame:
  EngineFrame(0, ANCHOR_FRAME=100), calibration: Estimated}` を
  publish する。anchor 未 publish の pending event は待機する
  既存契約（`midi_delivery_anchor_none_pending_waits_then_maps`
  系）があるため、pump を `rt.process` より先に完了させられる。
- **結論**: `ObservingInput`（production `MidiInput` への passive
  tap — `read`/`close` を delegate し全 `TimedMidiInput` を記録）
  で実 stamp を採取し、anchor を観測値から較正。`×48`
  （48 000/1 000）の frame 換算は `stamp.base==1/1000` を
  `stamp_contract_holds` で契約検査した場合のみ使用 — foreign
  base は envelope 外の正直な `NotRun`。

## 試行 3: counter oracle の独立期待値

- **目的**: `InputStage`/`InputTx` counter を被試験経路の値では
  なく独立期待値で照合する。
- **観測**: 全 event を `rt.process` 前に pump する構成では、
  anchor stamp より前の stamp を持つ観測 event は `BeforeAnchor`
  として `rejected` に計数される — 期待値は
  `observed.iter().filter(|e| e.stamp.ticks < on_stamp).count()`
  で厳密に計算可能。`InputStage::read` 経路は使わないため
  stage 側 `dropped_backend`/`unsupported_backend` は 0 が期待値、
  実 transparency は `InputTx` の累積 accessor が pump 観測累積と
  一致することで検証する。`mapped==delivered` は全 mapped event
  が処理済み block 範囲に写像される前提 — stamped span を
  `MAX_BLOCKS=48` で事前制限して edge を fail-closed にする。
- **結論**: `assert_counter_oracle` に独立期待値を実装。
  `late`/`anchor_rejected`/`dropped_capacity`=0、`delivered>=4`、
  `deferred()=0` を併せて固定。

## 試行 4: lint 対応（`#[allow]` 不使用の分解）

- **目的**: workspace lint（`-D warnings`）を `#[allow]` なしで
  通す。
- **観測**: `doc_markdown`（`NotRun`/`WinMM`/`NoteOn`/`NoteOff`
  等の backtick 必須）・`too_many_lines`/複雑度系・
  `cast_possible_truncation`（`u64`→`usize` は
  `usize::try_from` へ）・`cast_precision_loss`（attack 値の
  `u32 as f32`）が検出された。
- **結論**: doc comment を全面改訂して backtick を付与、test 本体
  を `open_loopback_pair`/`pump_until_sequence`/`process_blocks`/
  `assert_pcm_oracle`/`assert_counter_oracle`/
  `assert_close_semantics` に分解。cast は checked 変換に置換し、
  fixture と bit-exact を要する `u32 as f32 / 4_294_967_295.0`
  のみ repo 既存 pattern の `#[expect(clippy::cast_precision_loss)]`
  + 正当化 comment（`#[allow]` は不使用）。

## 試行 5: dev-dep の最小化判断

- **目的**: 承認済み dev-dep リスト（`ruviform-midi-device`・
  `ruviform-audio-device` の path dep）から必要最小限を選ぶ。
- **観測**: verifier は `ruviform_midi_device::backend()` の
  `MidiInputEnumerator`/`MidiOutputEnumerator` port API のみを
  使い、`ruviform-audio-device` の面（audio device backend）は
  直接参照しない。
- **結論**: `ruviform-midi-device` のみ追加。`Cargo.lock` 差分は
  当該 dep 1 行のみであることを確認。
