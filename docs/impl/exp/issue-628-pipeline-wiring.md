# Issue #628 pipeline wiring — 実装実験記録（2026-11-15）

`docs/tasks/issue-628-pipeline-wiring.md` の 3rd slice（pipeline wiring）
実装で試行・観察した事項の記録。実装結果の正本は
[`docs/impl/ir/issue-628-pipeline-wiring.md`](../ir/issue-628-pipeline-wiring.md)、
イベントログは
[`issue-628-pipeline-wiring.jsonl`](issue-628-pipeline-wiring.jsonl)
を参照。

## 観察事項

- **`InputTx` privacy**: `ruviform-audio-plugin-bridge` の
  `InputTx::pump` は `pub(crate)` で、production の MIDI feed pump
  アルゴリズムを crate 外から駆動できない。公開面は
  `MidiInputEnumerator` → `MidiInputPort::open` →
  `InputStage::{read,pump,collect_block}` のみ。製品を変更しない
  受入条件の下で、test-owned `InputPump` が同一アルゴリズムを mirror
  する構成を採用した。
- **Input leg の capture 境界**: 本番 Recording コンポーネントは
  意図的に未存在（`scarecrow173/ruviform#187` 後送り）。typed runtime
  境界 `RtRuntime::input_writer`/`process`/`output_reader` を優先経路
  とし、`Device::open_input_stream` の `InputDataCallback` が受け取る
  PCM + `InputCallbackInfo` は bounded SPSC `InputCapture` pool
  （`pool_blocks × channels × capacity_frames` 事前確保）で driver 側へ
  handoff する構成を採用。`mock_input_backend.rs` の先例に倣った。
  capture `None` は合成せず `None` のまま保持し、frame-position fault
  は verbatim 伝播する。
- **Session error callback の観測限界**: `Session::open` は no-op
  error callback を install するため、production `OutputDataCallback`
  内で発火した `DeviceError` の値自体は session surface から観測不能。
  `PumpReport` の `errors_fired`/`invalidated` と stream invalidated
  状態のみが生存する。製品側を変更せず finding として記録した。
- **`#[non_exhaustive]` 対応**: production の `PumpStats` は外部構築
  不能のため、test-owned `InputPumpStats` を公開面に採用。
  `expect_err` が要求する `T: Debug` のため `PlaybackPipeline`/
  `InputPipeline`/`MidiLeg` に手動 `Debug` impl を追加。
- **RT 計測境界の切り分け**: output leg の `captured` は post-callback
  capture が意図的に alloc するため RT 窓では `record_output=false`
  で callback 駆動部のみを計測。input leg は capture pool を
  warm-up で事前確保し writer/process/reader 経路を alloc-free 計測。
  midi leg は `collect_block`（scratch-seeded）を計測対象に分離。
- **clippy `-D warnings` 対応**: `similar_names`（`compiled`→`plan`）、
  `manual_let_else`/`single_match_else`（capture block 取得を
  `let ... else` 化）、`double_must_use`（`InputCapture::pair` の
  `#[must_use]` 除去）、`assert_is_empty`（test の
  `assert!(x.is_empty())` → `assert_eq!(x.len(), 0)`）、
  `needless_range_loop`（`enumerate` 化）、doc_markdown（counter 名に
  backtick 付与）。

## 結果

- `mise run fmt-check` / `check` / `lint` / `test
  ruviform-audio-test` / `test-rt` / `prek run --all-files` すべて
  pass（Windows 11 x64、rustc 1.99.1 mise pin）。
- 新規 `rt_pipeline_*` 3 test が `mise run test-rt`（workspace 全体
  `rt_` filter）で収集・実行されることを出力で確認。
