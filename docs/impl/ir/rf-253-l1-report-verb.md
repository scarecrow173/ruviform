---
id: "IMPL-034VnQ9uBnTxr8BNjvWsMt"
type: "impl"
status: "completed"
title: "Issue #253 L1 wire report verb"
created: "2026-11-10"
updated: "2026-11-10"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/rf-253-l1-report-verb.md"
      - type: "file"
        path: "docs/specs/rf-midi-l1-report-verb.md"
      - type: "file"
        path: "docs/designs/rf-midi-l1-report-verb-design.md"
      - type: "file"
        path: "docs/plans/rf-midi-l1-report-verb-plan.md"
      - type: "file"
        path: "docs/tasks/rf-253-task1-l1-report-verb.md"
      - type: "file"
        path: "docs/test-specs/rf-midi-l1-report-verb.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-core/src/wire.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/src/bin/ruviform-plugin-l1-runner.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/tests/l1_runner_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/bin/l1-test-worker.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/mod.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/composite.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/plugin_graph_e2e.rs"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/rf-253-task1-l1-report-verb.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/rf-midi-l1-report-verb.md"
    - "docs/designs/rf-midi-l1-report-verb-design.md"
    - "docs/plans/rf-midi-l1-report-verb-plan.md"
    - "docs/test-specs/rf-midi-l1-report-verb.md"
    - "docs/impl/ir/issue-253-host-graph-report-consumer-wiring.md"
    - "docs/impl/ir/issue-253-vst3-midi-canonical-path.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/rf-midi-l1-report-verb.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/253"
    - "https://github.com/scarecrow173/ruviform/pull/563"
    - "https://github.com/scarecrow173/ruviform/pull/555"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #253 L1 wire report verb

## Summary

Issue #253 の残件「L1 wire verb」を実装した。wire protocol を
`WIRE_VERSION` 3→4 に bump し、`Command::DrainOutputReports`
（instance-scoped、tag 23）と `Reply::Reports { events }`
（tag 8）を追加 — report event は既存 canonical
`Event`/`EventKind` codec を再利用し、decode は
`WIRE_MAX_EVENTS`（1024）で allocation 前に bound する。runner
（`ruviform-plugin-export`）は `InstanceEntry.report_backlog` を
所有し、plugin の `drain_output_reports` を backlog へ転記して
先頭 `WIRE_MAX_EVENTS` 件を oldest-first に返す（残りは次回
drain）。host は `L1Instance::drain_output_reports` が short
reply まで verb を loop して chunk を集約し、composite の L1
audio arm が `ReportDrain::Drained` を返す。video slot は report
verb を持たないため `Unavailable` を維持。`l1-test-worker` に
`--reports-per-block` fixture、reference runner の `GainPlugin`
に入力 event echo + deterministic MIDI marker を追加し、codec
unit / runner E2E / proxy E2E / graph E2E で実 plugin・実
device なしに全経路を検証した。

## Context

- 実施タスク: [task](../../tasks/rf-253-task1-l1-report-verb.md)
- 上流: [spec](../../specs/rf-midi-l1-report-verb.md)、
  [design](../../designs/rf-midi-l1-report-verb-design.md)、
  [plan](../../plans/rf-midi-l1-report-verb-plan.md)
- 契約保証の正本: [test-spec](../../test-specs/rf-midi-l1-report-verb.md)
- 前提: PR #563（Issue #253 host-graph consumer 配線）の
  `PluginHost::drain_output_reports`/`ReportDrain` — L1 slot は
  `Unavailable` だった。[IR](issue-253-host-graph-report-consumer-wiring.md)
  が「L1 wire への report verb 追加は別 slice」と記録。
  PR #555（`641f49dd`）の canonical `Event` codec と
  `PluginProcessor::drain_output_reports` 中立契約 —
  [IR](issue-253-vst3-midi-canonical-path.md)。

## Implementation

### Changes

- `crates/ruviform-plugin-core/src/wire.rs`:
  - `WIRE_VERSION` を 3→4 に bump。module doc・`Hello`・shm
    layout（`EVENT_WIRE_SIZE` = 33 B、slot sizing）の version
    記述を v4 前提に更新。
  - `Command::DrainOutputReports`（wire tag 23、instance-scoped）
    と `Reply::Reports { events: Vec<Event> }`（reply tag 8）を
    語彙に追加。encode は `u8 tag + u32 count + encode_event ×
    count`、decode は `read_count(WIRE_MAX_EVENTS)` で
    allocation 前に bound。
  - tag 8 は旧来「未知 reply tag」として test されていた値 —
    `video_reply_tags_and_presence_are_strict` の unknown-tag
    probe を 9 へ移し、v4 語彙の comment を更新。
  - codec test 追加: verb/reply roundtrip（両 `EventKind`・
    producer order）、`WIRE_INSTANCE_NONE` scope reject、
    `LimitExceeded`/`Truncated`/`Malformed` 境界、version assert
    = 4。
- `crates/ruviform-plugin-export/src/lib.rs`:
  - `InstanceEntry` に `report_backlog: Vec<Event>` を追加し
    `CreateInstance` で初期化。`DestroyInstance` で entry ごと
    破棄。
  - `WireCommand::DrainOutputReports` dispatch arm →
    `InstanceEntry::drain_output_reports`: `guarded` 経由で
    `ErasedPlugin::drain_output_reports(&mut report_backlog)`
    を呼び（panic は `HostError::PluginFailed`）、backlog 先頭
    から `min(len, WIRE_MAX_EVENTS)` を oldest-first に
    `Reply::Reports` へ。残りは backlog に残る。
- `crates/ruviform-plugin-export/src/bin/
  ruviform-plugin-l1-runner.rs`:
  - `GainPlugin` に `pending_reports`/`blocks` を追加。`process`
    は入力 `EventList` の各 event を到着順に echo し、block 毎に
    `gain_marker(blocks)`（deterministic MIDI `NoteOn`）を 1 件
    append。`drain_output_reports` は pending を caller の vec
    へ append。既存の gain/lifecycle/save-restore/panic/
    underwrite fixture 挙動は不変。
- `crates/ruviform-plugin-export/tests/l1_runner_e2e.rs`:
  - `e2e_drain_output_reports` — 実 runner child で空 drain・
    `InvalidHandle`・`ParamValue`+`Midi` echo+marker の producer
    order・repeat drain 空・`StopProcessing` 後 drain・
    deactivate/destroy/shutdown を検証。
- `crates/ruviform-plugin-host/src/bin/l1-test-worker.rs`:
  - `Flags.reports_per_block`（`--reports-per-block=N`、default
    0）、`WorkerEntry.pending_reports`/`next_report_index` を追加。
  - `report_events(count, base)`: even index `ParamValue`（id
    `index%8`、value `(index%100)/100`）、odd `Midi` `NoteOn`
    （key `60+index%12`、channel 0、velocity `index`）、
    `frame_offset` は run 内 index。
  - `process_block` 成功時のみ queue（失敗/reject block は
    queue しない）。`DrainOutputReports` dispatch は backlog
    先頭から `WIRE_MAX_EVENTS` まで drain して `Reply::Reports`。
- `crates/ruviform-plugin-host/src/l1/mod.rs`:
  - `L1Instance::drain_output_reports(&mut self, out)` —
    `execute(Command::DrainOutputReports)` を loop し、
    `Reply::Reports` chunk を `out` に append。chunk が
    `WIRE_MAX_EVENTS` ちょうどなら再要求、short reply で終了。
    他の reply payload は `Err(Ipc)`（protocol anomaly）。
    lifecycle-ungated、non-RT contract を doc 明記。
- `crates/ruviform-plugin-host/src/composite.rs`:
  - `PluginHost::drain_output_reports` の `Slot::OutOfProcess`
    （audio）arm を `instance.drain_output_reports(out)?` →
    `Ok(Drained)` に変更。video arm は `Unavailable` 維持。
  - unit test `drain_output_reports_routes_by_slot_kind` を更新
    — L1 `test_fixture` スロットが childless fixture 経路で
    `Drained` を返すことを検証（fixture は `Reply::Reports`
    空を返す stub）。
- `crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs`:
  - `worker_reports`（worker `report_events` の mirror）+ drain
    E2E 3 件: `drain_output_reports_pulls_worker_reports`
    （両 kind・per-block run の frame_offset restart・repeat 空・
    destroy 後 `Err(Ipc)`）、`drain_output_reports_chunks_over_
    wire_max_events`（1100 件 backlog の集約回収）、
    `drain_output_reports_failed_instance_is_failed`（`Failed`
    収束で `Err(Failed)`）。
- `crates/ruviform-plugin-host/tests/plugin_graph_e2e.rs`:
  - `l1_processing_with_args`（`OutOfProcessSpec.args` 経由で
    worker argv を渡す）+ `l1_worker_reports`（同 mirror）。
  - `l1_reports_drain_through_host_while_endpoint_attached` —
    旧 `l1_reports_drain_answers_unavailable` を置換。実
    `l1-test-worker` child + endpoint attach 中の graph 駆動
    block（各 block 後に 2×`SaveState` pipeline barrier で
    submit channel（depth = lookahead+1 = 3）の drop を防ぎ
    deterministic に）で `Drained` + canonical `Event` を検証。
- `docs/`: spec → design → plan → test-spec → task → IR の
  文書チェーンを作成（scaffold scripts + 手動 relations 記述）。
  `docs/*/README.md` index は scaffold が再生成。

### Decisions During Implementation

- **report は control-plane verb、data plane ではない**: report
  drain は plugin の非 RT 状態を読む操作であり、shm slot +
  doorbell の block 単位 RT 契約に乗らない。`SaveState`/`GetParam`
  と同じ「instance-scoped command → bounded reply」の形が責務
  分割と一致する（design Context）。
- **runner backlog は chunking のためだけに存在する**: plugin
  の `drain_output_reports` は since-last-drain を caller vec に
  append する契約であり、runner はそれを `report_backlog` に
  転記して先頭 `WIRE_MAX_EVENTS` のみを reply に載せる。残りは
  次回 drain が拾う — reorder/複製/無制限 buffer を持たない。
- **host loop の終了条件は「chunk < WIRE_MAX_EVENTS」**: child
  は serial で backlog を単調減少させるため full chunk の連続は
  有限。`Reports` 以外の reply は protocol anomaly として
  `Err(Ipc)` で fail-closed。
- **lifecycle-ungated**: drain verb は状態遷移を要求しない —
  report buffer は plugin 状態であり、L0 drain・`buses` と同じ
  「滞留分を引き出せる」意味論。`post_command_state` wildcard
  が mirror を触らないため追加配線は不要だった。
- **video は `Unavailable` 維持**: video wire 語彙（command
  `12..=22`）に report が無く、consumer 需要も未定 — YAGNI。
  `ReportDrain` は `#[non_exhaustive]` で将来拡張に耐える。
- **capability は `Declared` 維持**: L1 `Drained` を fixture で
  証明したが、`param_reporting` cap が指す実 plugin/実 DAW 往復・
  実デバイス lane・consumer 消費は未検証。`cap.rs` は編集範囲外
  でもある。
- **tag 8 の再利用**: reply tag 8 は旧 test の「未知 tag」probe
  だったが、reply 語彙は `0..=7` が使用中で 8 が最小の空き —
  `Reports` を 8 に割当、unknown-tag probe は 9 へ移動。

## Related Experiments

### Adopted

<!-- なし — 実験的分岐なし（既知解の機械的適用 + 設計判断は design に記録）。 -->

### Rejected

<!-- なし。 -->

## Validation

実施済み（補助証拠 — canonical = mise）:

- `cargo test -p ruviform-plugin-core wire::` — 51 tests PASS
  （codec roundtrip/bounds/scope/malformed/version assert）。
- `cargo test -p ruviform-plugin-export --test l1_runner_e2e
  e2e_drain_output_reports` — PASS（実 runner child の全
  semantics）。
- `cargo test -p ruviform-plugin-host --lib
  drain_output_reports` — composite unit PASS。
- `cargo test -p ruviform-plugin-host --test l1_proxy_e2e
  drain_output_reports` — 3 tests PASS（pull/chunk/failed）。
- `cargo test -p ruviform-plugin-host --test plugin_graph_e2e
  l1_reports_drain` — PASS（endpoint attach 中の L1 drain）。

canonical 検証（`git merge origin/main` = Already up to date 後に
実施・Windows 11 x64 / Rust 1.99.0 / exit 0）:

- `mise run fmt-check` — PASS
- `mise run check` — PASS
- `mise run lint` — PASS（初回 `assert_is_empty`・
  `manual_is_multiple_of`・`unnested_or_patterns`・
  `semicolon_if_nothing_returned`・`too_many_lines` を修正して
  再検証）
- `mise run test` — PASS（初回に `l1_proxy_e2e::
  command_timeout_converges_failed_with_timeout_cause` と
  `plugin_graph_e2e::l1_endpoint_node_drives_worker_gain_
  through_session` が spawn/pipeline timeout で各 1 件 flake —
  単独再実行・全量再実行とも PASS、変更起因でないことを確認）
- `mise run test-rt` — PASS
- `mise run test-plugin-core` — PASS
- `mise run lint-plugin-core` — PASS
- `mise run lint-l1-real-plugin` — PASS
- `mise run test-l1-real-plugin` — PASS（CLAP/VST3 fixture +
  `l1_real_adapter_e2e` 3 tests × 2 crate）
- `mise run verify-ci` — PASS（fmt-check → lint-ci → test-ci）
- review-protocol — reviewer A/B・validator・judge を separated
  passes で実施。CONFIRMED P0/P1 なし、test-spec RN-03 の
  coverage 記述を実測に合わせて修正（P2 相当）。verdict:
  ACCEPT WITH NON-BLOCKING NOTES

NotRun / Unsupported:

- 実 OS MIDI backend・opt-in 実デバイス lane・実 DAW VST3 往復・
  macOS/Linux CoreMIDI/ALSA — Issue #253 の残 gate（本 slice
  対象外、実機必須）。
- Synvelis/AudioLens 側の consumer 消費実装 — 別 Issue。
- `param_reporting` の `Verified` 昇格根拠となる実 plugin/実機
  検証 — 未実施のため `Declared` 維持。
- video report verb — video wire 語彙に report が無く、consumer
  需要も未定（design の deferred concern）。
- `ruviform-audio-plugin-bridge` の `unsupported` 計数透過欠落 —
  別 slice（編集範囲外、PR 本文「未対応の作業」に記録）。

## Risks

- host loop が `WIRE_MAX_EVENTS` ちょうどの chunk を連続して
  受け取る経路 — child は serial で backlog を単調減少させるため
  有限。異常 reply は `Err(Ipc)` で fail-closed。
- 巨大 backlog の drain が `command_timeout` を超える — chunk
  は `WIRE_MAX_EVENTS` 固定で 1 verb の待ちは bounded reply と
  同程度。timeout は `execute` 既存機構が `Err` に写像。
- `Destroy`/child 終了で backlog は破棄される — drain しなかっ
  た report の欠損は defined（L0 と同一契約）。
- fixture の deterministic report 式は test 側に mirror —
  fixture 変更時に `worker_reports`/`l1_worker_reports` の同期が
  必要（各所に mirror comment 記載済み）。

## Follow-ups

- Issue #253 は open のまま: 実 OS MIDI backend（macOS/Linux
  含む）、opt-in 実デバイス lane、実 DAW 往復、consumer 消費
  実装、`param_reporting` 昇格が残る。
- video report verb — video wire 語彙拡張 + consumer 需要が
  出た時点で別 slice。
- bridge 側 `unsupported` 計数の透過欠落 — capability 行列の
  設計 slice と一緒に扱う。
