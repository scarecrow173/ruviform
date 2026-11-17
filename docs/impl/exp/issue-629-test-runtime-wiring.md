# Issue #629 test runtime wiring — 実装実験記録（2026-11-17）

`docs/tasks/issue-629-test-runtime-wiring.md` の AC3 Test Runtime
完全結線 slice（依存方向裁定 + verdict→RunMetadata 対応付け +
deterministic signal/Event 供給の headless 範囲）実装で試行・観測
した事項の記録。実装結果の正本は
[`docs/impl/ir/issue-629-test-runtime-wiring.md`](../ir/issue-629-test-runtime-wiring.md)
（`IMPL-034Zb3GiG71rPTDG6eZpFv`）、イベントログは
[`issue-629-test-runtime-wiring-exp.jsonl`](issue-629-test-runtime-wiring-exp.jsonl)
を参照。

## 試行 1: 依存方向の裁定

- **目的**: device wiring test の置き場と依存方向を決める
  （実装・`Cargo.lock` 変更より先の第一工程）。
- **観測**: `ruviform-audio-device`→`ruviform-audio-test`
  dev-dep は前回不承認 — audio-test が引く
  `ruviform-audio`/`plugin-bridge`/`graph` 系が device crate の
  default `cargo test` compile graph に載るため。逆方向
  （`ruviform-audio-test` dev-deps → 両 device crate）は crate
  cycle を生じない（`ruviform-audio` は両 device crate に依存
  しない）ことを依存グラフで確認。先例として
  `ruviform-audio-plugin-bridge`→`ruviform-midi-device` dev-dep、
  `ruviform-plugin-test-vst3`→両 device crate dev-dep が存在。
- **結論**: 逆方向 dev-dep を採用（design §1 に ADR 相当の裁定
  として記録）。dev-dep に留めるため `ruviform-audio-test` の
  lib compile graph・AL/SV consumer の dependency resolution を
  汚染しない。verifier 側で RunMetadata 準拠 JSON を emit する
  代替案（schema 重複）は不採用 — typed 結線の方が schema 単一
  正本を保てる。

## 試行 2: `RunMetadata::for_environment` の fail-closed 設計

- **目的**: verdict→RunMetadata 対応付けの typed API を additive
  に追加する。
- **観測**: 既存 `deterministic` constructor は OsL1+
  DeterministicBackend 固定で endpoint case（ext-l3 +
  virtual-os-endpoint）を表現できない。`for_environment` は
  level/environment_kind を明示引数化し、`result` を
  `CaseResult::NotRun` で初期化 — 構築だけで Pass にならない
  ことを型で保証。builder method（`with_result` 等）は従来どおり
  self 消費で、endpoint case では実行後にのみ Pass/Fail を立てる。
- **結論**: `for_environment` 追加 + `deterministic` は shorthand
  として維持。既存 serde 語彙・variant 不変（additive のみ）。
  `#[expect(clippy::too_many_arguments)]` は 8 引数のため
  workspace 方針どおり `expect`（`allow` 不使用）で対応。

## 試行 3: wiring test の expected/observed 分離

- **目的**: deterministic signal/Event を本番 device API 入口へ
  供給し、自己参照 Oracle を避ける。
- **観測**: MIDI leg は send list を `MidiScenario`→
  `run_midi_trace` で生成（手組みリストを被試験 adapter に
  食わせない）。expected 側は独立 literal 4-event channel-voice
  oracle を ordered subsequence で照合。Audio leg は
  `virtual_endpoint.rs` の name-stem pairing を再利用しつつ、
  supply は `PcmFixture`/`PcmPattern`、expected は独立
  marker/RMS tolerance oracle — 被試験 adapter は期待値を
  生成しない。
- **結論**: Test Runtime が供給するのは expected 側 Fixture/
  metadata のみ。observed 側 oracle は全て独立実装で、
  「供給側と照合側が同じコードを共有しない」契約を維持。

## 試行 4: 未 provisioning での fail-closed self-run

- **目的**: 本ホスト（仮想 endpoint 未 provisioning）で wiring
  leg の NotRun 経路を実測する。
- **観測**: `cargo test -p ruviform-audio-test --test
  os_endpoint -- --ignored --nocapture` で audio leg が
  `observed:"no input devices on this host"`（host の input
  device 列挙が空 — device verifier lane でも観測済みの
  NotRun 系）、MIDI leg が `observed:"NOT_PROVISIONED — no
  bidirectional name match"` の `run_metadata` JSON
  （`result:"not-run"`, `level:"ext-l3"`,
  `environment_kind:"virtual-os-endpoint"`）+ `verdict: NotRun`
  を emit して exit 0。deterministic legs は 3 passed。
  `test-audio-virtual-endpoint`/`test-midi-virtual-endpoint`/
  `test-midi-real-device` の canonical lane も全て exit 0 で
  既存 verifier の挙動不変を確認。
- **結論**: 未 provisioning → NotRun（fail しない・Pass に
  しない）が設計どおり動作。evidence は
  `evidence/rf629/test_runtime_wiring_slice_selfrun-windows-
  local.txt` と `os_endpoint_wiring_selfrun-windows-local.txt`
  に記録。Pass・`virtual-os-endpoint` 実走 Pass は主張しない。

## 試行 5: stale 参照の同期範囲

- **目的**: AC3 typed 結線の Deferred 記述を持つ 3 design と
  survey Evidence を同期する。
- **観測**: `issue-629-wasapi-endpoint.md`・
  `issue-629-virtual-endpoint-lane.md`・
  `issue-629-provisioning-runbook.md` の Deferred Design
  Concerns に「Coordinator 承認待ち」「逆方向も未裁定」の
  記述が残っていた。survey Evidence 節は append-only 規約の
  ため既存 entry を編集せず新規 dated entry を追記。
- **結論**: 3 design の該当節を「解消済み（AC3 wiring slice）
  + 逆方向 dev-dep 採用・device→audio-test 不承認不変・
  provisioned 実走は外部 gate に残る」へ更新。`loopback.rs`・
  `virtual_endpoint.rs` の module doc は結線状況を直接記述
  していないため変更不要と判断（doc のみの stale は無し）。
