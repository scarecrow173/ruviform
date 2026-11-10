---
id: "SPEC-034VnJfOmabJjA6HmkX45O"
type: "spec"
status: "approved"
title: "Issue #253 L1 wire report verb（drain_output_reports の L1 実装）slice"
created: "2026-11-10"
updated: "2026-11-10"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-253-hostgraph-reports.md"
    - "docs/designs/issue-253-hostgraph-reports.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-253-vst3-midi-canonical-path.md"
    - "docs/specs/issue-253-os-midi-backend.md"
    - "docs/designs/plugin-isolation.md"
    - "docs/designs/engine-plugin-host-integration.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-253-hostgraph-reports.md"
    - "docs/designs/issue-253-hostgraph-reports.md"
  derived-by:
    - "docs/designs/rf-midi-l1-report-verb-design.md"
  verifies: []
  verified-by:
    - "docs/test-specs/rf-midi-l1-report-verb.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/253"
    - "https://github.com/scarecrow173/ruviform/pull/563"
  defers: []
  deferred-by: []
metadata:
  issue: 253
---

# Issue #253 L1 wire report verb（drain_output_reports の L1 実装）slice

## 背景とスコープ

PR #563（Issue #253 host-graph consumer 配線 slice）で
`PluginHost::drain_output_reports` / `InProcessHost::
drain_output_reports` と `ReportDrain`（`Drained`/`Unavailable`）が
merge 済みである。しかし L1（out-of-process）audio/video スロットは
wire protocol に report verb を持たないため `Unavailable` を返す —
これが Issue #253 に残された本 slice の対象行「L1 wire verb」で
ある。本 slice はその 1 行だけを実装する: L1 wire 経路に report
drain verb を追加し、`PluginHost::drain_output_reports` の L1 audio
スロットが `Unavailable` ではなく `Drained` + canonical `Event` を
返すようにする。

対象:

- `ruviform-plugin-core/src/wire.rs`: `WIRE_VERSION` 3→4 bump、
  `Command::DrainOutputReports`（instance-scoped、verb tag 23）、
  `Reply::Reports { events: Vec<Event> }`（reply tag 8）の追加と
  既存 `Event`/`EventList` codec の再利用。decode は
  `WIRE_MAX_EVENTS` で allocation 前に bound し、truncated /
  malformed / reserved 非零の strict rejection を維持する。
- `ruviform-plugin-export`（runner）: `InstanceEntry` に per-instance
  report backlog を追加し、`DrainOutputReports` で
  `ErasedPlugin::drain_output_reports` を呼び、最大
  `WIRE_MAX_EVENTS` 件を oldest-first で返す。残りは次回 drain
  まで queue に残す。plugin panic は既存 `guarded` 機構経由で
  `HostError::PluginFailed` に写像する。
- `ruviform-plugin-host`: `L1Instance::drain_output_reports` が
  `DrainOutputReports` を short/empty reply まで loop し、返って
  きた chunk を caller の `Vec` に append する。`composite.rs` の
  L1 audio arm をこの instance drain に配線し、video スロットは
  report verb を持たないため `Unavailable` のまま維持する。
- fixture: `l1-test-worker` に `--reports-per-block=N`（処理成功
  `BlockIn` ごとに deterministic な `ParamValue`/`Midi` 交互
  report を queue）、reference runner の `GainPlugin` に入力
  event echo + block 毎の deterministic MIDI marker。
- headless fixture による E2E: codec 往復・runner E2E・
  `l1_proxy_e2e`・`plugin_graph_e2e` で全経路を実 plugin・実
  device なしに検証する。

対象外（Issue #253 に残る・本 slice では Pass に繰り上げない）:

- video protocol への report verb 追加 — video スロットの
  `Unavailable` は維持する
- `ruviform-audio-plugin-bridge` 側の `unsupported` 計数の透過
  欠落 — 別 slice（PR 本文の「未対応の作業」に記録）
- 実 OS MIDI backend、opt-in 実デバイス lane、実 DAW VST3 往復、
  macOS/Linux CoreMIDI/ALSA、consumer 消費実装
  （Synvelis/AudioLens 側 drain 消費）
- `param_reporting` capability の `Declared`→`Verified` 昇格 —
  実 plugin/実 DAW 往復が未検証のため本 slice でも `Declared`
  のまま（判断根拠は design/IR に記録）

## 要件

### R1 wire protocol（`ruviform-plugin-core/src/wire.rs`）

- `WIRE_VERSION` を 3→4 に bump し、version gate・doc・shm layout
  記述を整合させる。runner と host は workspace 内で同時再 build
  されるため互換ブロックにはならないが、protocol 変更として文書
  チェーンに記録する。
- `Command::DrainOutputReports`（verb tag 23、instance-scoped —
  `WIRE_INSTANCE_NONE` は reject）と `Reply::Reports { events }`
  （reply tag 8）を語彙に追加する。既存の audio command tag
  （`12..=22` は video domain）と reply tag の予約則を壊さない。
- report event は既存 canonical `Event`/`EventKind` codec
  （`encode_event`/`decode_event`）を再利用し、`ParamValue` と
  `Midi` の両 kind を運ぶ。decode は `WIRE_MAX_EVENTS` を allocation
  前に検査して bound し、truncated・malformed・reserved 非零を
  従来どおり strict に reject する。
- wire codec は producer/consumer の意味論を持たない — report の
  since-last-drain・順序・bounded drop は plugin/runner が所有する。

### R2 runner（`ruviform-plugin-export`）

- `InstanceEntry` が per-instance の report backlog を所有し、
  `CreateInstance` で初期化する。
- `DrainOutputReports` dispatch は `ErasedPlugin::
  drain_output_reports` を呼び、backlog に append された report の
  うち最大 `WIRE_MAX_EVENTS` 件を oldest-first で
  `Reply::Reports` に載せ、残りを queue に残す。
- plugin panic は既存 `guarded` 機構経由で `HostError::PluginFailed`
  に写像し、audio/video cross-kind rejection の既存挙動は変えない。
- drain は lifecycle-ungated — report buffer は plugin 状態であり、
  L0 drain と同じく状態遷移を要求しない。

### R3 host（`ruviform-plugin-host`）

- `L1Instance::drain_output_reports(&mut self, out: &mut Vec<Event>)
  -> Result<(), HostError>` を追加し、`DrainOutputReports` を発行
  して `Reply::Reports` chunk を `out` に append、short reply
  （`WIRE_MAX_EVENTS` 未満）まで loop する。child が 1 reply に
  収まらない backlog を返しても host 側 1 pull で全件回収できる。
- `composite.rs` の `Slot::OutOfProcess`（audio）arm をこの
  instance drain に配線し、`PluginHost::drain_output_reports` が
  L1 audio で `Ok(ReportDrain::Drained)` を返す。L0 委譲と
  video `Unavailable`、stale handle の `InvalidHandle`、`Failed`
  の `Err(Failed)` は既存どおり。
- drain は非 RT 契約を維持する — control-plane command/reply 経路
  のみを使い、RT data plane・shm slot・endpoint channel に一切
  触れない。`out` append の allocation は caller 側責務として
  許容する。

### R4 保存する意味論

- `EventKind::ParamValue` と `EventKind::Midi` の両方を運び、
  producer が queue した順序を保存する。
- since-last-drain: plugin の pending buffer / runner backlog /
  host loop のいずれの層でも「前回 drain 以降の新規分」だけが
  返り、repeat drain は空を返す。
- bounded: producer（plugin）側の bounded pending buffer が
  authoritative で、capacity 超過分は producer 側の defined drop。
  runner/host は reorder・複製・無制限 buffer を持たない。
- wire reply は `WIRE_MAX_EVENTS` で chunk され、host loop が
  short reply まで再要求する — 1 回の wire verb で無制限の
  allocation を起こさない。

### R5 検証と記録

- codec: verb/reply roundtrip、`WIRE_MAX_EVENTS` 境界、
  instance scope（`WIRE_INSTANCE_NONE` reject）、truncated/
  malformed/reserved 非零、未知 reply tag の strict reject を
  unit test で固定する。
- runner E2E: 実 `ruviform-plugin-l1-runner` child に対し、空
  drain・`InvalidHandle`・両 `EventKind`・producer order・
  repeat drain・lifecycle 遷移後 drain・destroy/shutdown を検証
  する。
- proxy E2E: `--reports-per-block` fixture で block 処理 →
  `L1Instance::drain_output_reports` が chunking（`WIRE_MAX_EVENTS`
  超過 backlog）・順序・repeat 空・`Failed`/destroy 後の defined
  failure を返すことを検証する。
- graph E2E: `plugin_graph_e2e` で実 `l1-test-worker` child +
  endpoint attach 中の composite L1 arm が `Drained` + canonical
  `Event` を返すことを検証し、旧 `Unavailable` 期待を置き換える。
  L0 経路と video `Unavailable` の既存 coverage は維持する。
- impl record に wire 語彙・chunking・capability 判断・検証結果・
  残 NotRun を記録する。

## 受入条件（本 slice — Issue #253 残件の「L1 wire verb」1 行）

- `PluginHost::drain_output_reports` の L1 audio slot が
  `Unavailable` ではなく `Drained` を返し、caller の `Vec` に
  canonical `Event` が append される。
- wire 側: `Command::DrainOutputReports` + `Reply::Reports`、
  `WIRE_VERSION` 3→4、runner dispatch を `wire.rs`・
  `plugin-export/src/lib.rs` に実装し、既存 `Event`/`EventList`
  codec を再利用する。
- host 側: `composite.rs` の composite L1 arm と
  `src/bin/l1-test-worker.rs` の arm を配線する。
- codec 往復テストと `tests/l1_proxy_e2e.rs`・
  `tests/plugin_graph_e2e.rs` の L1 経路 E2E を追加し、
  `l1-test-worker` fixture で実 plugin・実 device なしに全経路を
  検証する。
- 実 OS MIDI・実デバイス・実 DAW・macOS/Linux CoreMIDI/ALSA・
  consumer 消費・`Declared`→`Verified` 昇格は本 slice 対象外。
  未実行は `NotRun`/`Unsupported` と記録し Pass に繰り上げない。

## Deferred Design Concerns

- video report verb — video protocol に report を運ぶ語彙が無い。
  consumer が video report を必要とする Issue が起きた時点で
  video wire 語彙の拡張 slice として再検討する
  （re-engagement trigger: video plugin→host report の consumer
  需要）。
- bridge 側 `unsupported` 計数の透過欠落 — L1 report drain が
  bridge の capability 行列へどう計上されるかは別 slice の設計
  事項（`param_reporting` 昇格判断と一緒に扱う）。

## Sources and Relations

- Issue #253（`plugin→host report` 系列の残件管理）:
  <https://github.com/scarecrow173/ruviform/issues/253>
- 前提 slice: `docs/specs/issue-253-hostgraph-reports.md`（host
  drain API + `ReportDrain`）、`docs/specs/
  issue-253-vst3-midi-canonical-path.md`（canonical `Event` codec
  と VST3 emit/収集経路、PR #555 `641f49dd`）。
- 関連: `docs/specs/issue-253-os-midi-backend.md`（同 Issue の別
  slice — 本 slice とは独立）。
