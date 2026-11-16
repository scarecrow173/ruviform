# Issue #707 — AC 棚卸し

`evidence/rf628/ac-inventory.md` の Deferred 表（`:38`「MIDI 側
fault への cancellation 拡張 → `scarecrow173/ruviform#707`」）を
本 slice で回収した分の受入条件棚卸し。形式は rf628 に倣う。

## AC 棚卸し（Issue #707 転記）

| # | 受入条件（Issue 転記） | verdict | 理由 | 根拠 |
| --- | --- | --- | --- | --- |
| 1 | MIDI input path で cancellation fault が expected/observed 一致する正負 test が `crates/ruviform-audio-test` に追加されている（deterministic OS-L1 範囲） | **Pass** | `MockPort::inject_cancelled`/`fail_next_read` が `MockInput::read` で FIFO 消費され、`MidiLeg`→`InputPump`→`PipelineError::Device` 経路で `Backend("cancelled")` → `StreamInvalidated` の observable が Expected/Observed 一致。正例 3 件（pre-armed・mid-stream・`pump_inputs` 直撃）+ 負例 3 件（`Backend("cancelled")` 単体非終端・FIFO ordered・loss counter 非終端） | `crates/ruviform-audio-test/tests/midi_faults.rs`（6 tests・`mise run test ruviform-audio-test` で実走 pass）。fixture 側は `crates/ruviform-audio-core/src/midi_input/mock.rs` unit test 4 件 |
| 2 | `mise run test ruviform-audio-test` / `check` / `lint` で収集・実行される | **Pass** | 新規 `tests/midi_faults.rs` は cargo が test target を自動収集 — `mise.toml` の enumerated lane 変更不要を実測確認（`mise run test ruviform-audio-test` exit 0 で 6 tests 実行）。check/lint も両 crate で pass | `evidence/rf707/verification.md` 実行記録。`mise.toml:68-71` の package raw_args 構造（task 前提の確認どおり） |
| 3 | 対応する docs chain（spec/design/plan/tasks/test-specs/impl）が追加されている | **Pass** | `docs/{specs,designs,plans,tasks,test-specs}/issue-707-midi-cancellation.md` + `docs/impl/{ir,exp}/issue-707-midi-cancellation.{md,jsonl}` を追加。docs index（`docs/**/README.md`）は Worker 範囲外のため未再生成 — Coordinator が統合時に再生成 | 本ディレクトリの docs chain ファイル群 |
| 4 | 実 MIDI デバイス上の fault 実 timing は EXT-L3 として本 Issue では NotRun 扱い（#629/#630 領域） | **Pass**（簿記） | test-specs `Non-goals`・本表 Deferred 節・`docs/specs` の Verification 節に NotRun/EXT-L3 を明記。実デバイス実走は実施していない | `docs/test-specs/issue-707-midi-cancellation.md` Non-goals、下記 Deferred 表 |

## 本 slice で正直に Deferred / NotRun のまま残る項目

| 項目 | 状態 | 理由・所有 |
| --- | --- | --- |
| 実 MIDI デバイス・実 endpoint 上の fault 実 timing | NotRun | EXT-L3。`scarecrow173/ruviform#629`（virtual MIDI endpoint）/`#630`（real ABI/plugin/runtime）領域。本 slice は deterministic in-process のみ |
| MIDI→Host adapter `attach()` 実 endpoint 接続 | NotRun | rf628 Deferred 表どおり `scarecrow173/ruviform#629` 領域（attach() 配線自体は PR `scarecrow173/ruviform#676` で回収済み） |
| cue 粒度の cancel（subscription-level より細かい中断点） | Deferred | audio 側 `Cancellation` の semantics は callback 単位の中断+終了であり port/subscription レベルが直接対応。cue 粒度が要求される場合は別設計 — spec `Deferred Design Concerns` に記録 |
| `Backend("cancelled")` + stats 併存観測（audio 側 error callback + data callback 相当） | Deferred | `MidiInput::read` は `Err` と `MidiReadStats` を同時に返せない契約。失敗 read（`fail_next_read`）と stats 付き成功 read（`inject_dropped`/`inject_unsupported`）に観測が分解される — test-spec `Risk` に記録 |
| `*-msrv`・`*-full` 系検証・3OS 横断 | NotRun | daily/手動 Full CI 管轄。merge 後 SHA の CI が統合検証を供給 |
| docs index 再生成 | Worker 範囲外 | Coordinator が統合時に `doc-maintenance` で再生成 |
